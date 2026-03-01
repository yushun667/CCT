use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::Emitter;
use tracing::{debug, error, info, warn};

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::graph::ParseStatistics;
use cct_core::models::project::{ParseStatus, ProjectType};
use cct_core::parser::incremental::IncrementalParser;
use cct_core::parser::scheduler::ParseScheduler;

use crate::services::project_service::{parse_project_id, ProjectService};

/// 全局取消标志 — 用于中断正在进行的解析任务
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

/// 获取项目对应的索引数据库路径
fn index_db_path(project_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cct")
        .join("index")
        .join(format!("{}.db", project_id))
}

/// 启动全量解析
///
/// 在后台线程中执行 ParseScheduler，通过 Tauri 事件实时推送进度。
/// 解析完成后将结果写入 SQLite 索引数据库。
#[tauri::command]
pub async fn start_full_parse(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<(), CctError> {
    info!(project_id = %project_id, "Tauri Command: start_full_parse");

    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    let project = service.get(&uuid)?;

    info!(
        name = %project.name,
        source_root = %project.source_root,
        project_type = ?project.project_type,
        "开始全量解析"
    );

    service.update_parse_status(&uuid, ParseStatus::InProgress)?;
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let source_root = project.source_root.clone();
    let compile_db = project.compile_db_path.clone().or_else(|| {
        find_compile_commands(&source_root)
    });
    if let Some(ref db) = compile_db {
        info!(compile_db = %db, "使用编译数据库");
    }
    let excluded_dirs = project.excluded_dirs.clone();
    let pid = project_id.clone();
    let db_path = index_db_path(&project_id);

    app.emit("parse-progress", serde_json::json!({
        "project_id": project_id,
        "phase": "scanning",
        "total_files": 0,
        "parsed_files": 0,
        "percentage": 0.0,
        "current_file": "扫描文件中...",
        "symbols_found": 0,
        "relations_found": 0,
        "elapsed_seconds": 0.0,
        "estimated_remaining": 0.0
    })).map_err(|e| {
        error!(error = %e, "发送初始进度事件失败");
        CctError::Internal(e.to_string())
    })?;

    match project.project_type {
        ProjectType::Local => {
            let cancel = Arc::new(AtomicBool::new(false));
            let cancel_clone = cancel.clone();
            let app_bg = app.clone();

            tokio::task::spawn_blocking(move || {
                let app = app_bg;
                info!("本地解析后台线程启动");

                let scheduler = ParseScheduler::new(None);
                let compile_db_path = compile_db.as_deref().map(std::path::Path::new);

                let result = scheduler.schedule_parse(
                    std::path::Path::new(&source_root),
                    compile_db_path,
                    Some(&db_path),
                    &excluded_dirs,
                    |progress| {
                        if CANCEL_FLAG.load(Ordering::SeqCst) || cancel_clone.load(Ordering::SeqCst) {
                            return;
                        }

                        let _ = app.emit("parse-progress", serde_json::json!({
                            "project_id": pid,
                            "phase": progress.phase,
                            "total_files": progress.total_files,
                            "parsed_files": progress.parsed_files,
                            "failed_files": progress.failed_files,
                            "percentage": progress.percentage,
                            "current_file": progress.current_file,
                            "symbols_found": progress.symbols_found,
                            "relations_found": progress.relations_found,
                            "elapsed_seconds": progress.elapsed_seconds,
                            "estimated_remaining": progress.estimated_remaining,
                        }));
                    },
                );

                update_parse_result(&app, &pid, result);
            });
        }
        ProjectType::Remote => {
            let ssh_config = project.ssh_config.clone().ok_or_else(|| {
                CctError::SshConnectionFailed("远程项目未配置 SSH".to_string())
            })?;

            let app_bg = app.clone();
            tokio::spawn(async move {
                info!("远程解析异步任务启动");

                let conn = match cct_ssh::SshConnection::connect(&ssh_config).await {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "SSH 连接失败");
                        let _ = app_bg.emit("parse-error", serde_json::json!({
                            "project_id": pid,
                            "error": format!("SSH 连接失败: {e}"),
                        }));
                        update_project_status(&pid, ParseStatus::Failed);
                        return;
                    }
                };

                let agent_bin = "~/.cct/agent/cct-agent";
                let mut rpc = match cct_ssh::AgentRpcClient::from_connection(&conn, agent_bin).await {
                    Ok(r) => r,
                    Err(e) => {
                        error!(error = %e, "Agent 启动失败");
                        let _ = app_bg.emit("parse-error", serde_json::json!({
                            "project_id": pid,
                            "error": format!("Agent 启动失败: {e}"),
                        }));
                        update_project_status(&pid, ParseStatus::Failed);
                        return;
                    }
                };

                match rpc.start_parse(&source_root, compile_db.as_deref()).await {
                    Ok(_) => {
                        info!("远程解析已启动，开始轮询状态");
                    }
                    Err(e) => {
                        error!(error = %e, "远程解析启动失败");
                        let _ = app_bg.emit("parse-error", serde_json::json!({
                            "project_id": pid,
                            "error": format!("远程解析启动失败: {e}"),
                        }));
                        update_project_status(&pid, ParseStatus::Failed);
                        let _ = rpc.shutdown().await;
                        return;
                    }
                }

                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                    if CANCEL_FLAG.load(Ordering::SeqCst) {
                        warn!("检测到取消标记，取消远程解析");
                        let _ = rpc.cancel_parse().await;
                        update_project_status(&pid, ParseStatus::Failed);
                        break;
                    }

                    match rpc.get_status().await {
                        Ok(status_val) => {
                            let status = status_val.get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            if let Some(progress) = status_val.get("progress") {
                                let _ = app_bg.emit("parse-progress", serde_json::json!({
                                    "project_id": pid,
                                    "total_files": progress.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0),
                                    "parsed_files": progress.get("parsed_files").and_then(|v| v.as_u64()).unwrap_or(0),
                                    "failed_files": progress.get("failed_files").and_then(|v| v.as_u64()).unwrap_or(0),
                                    "percentage": progress.get("percentage").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    "current_file": progress.get("current_file").and_then(|v| v.as_str()).unwrap_or(""),
                                    "symbols_found": progress.get("symbols_found").and_then(|v| v.as_u64()).unwrap_or(0),
                                    "relations_found": progress.get("relations_found").and_then(|v| v.as_u64()).unwrap_or(0),
                                    "elapsed_seconds": progress.get("elapsed_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    "estimated_remaining": progress.get("estimated_remaining").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                }));
                            }

                            match status {
                                "completed" => {
                                    info!("远程解析完成，开始传输索引数据");
                                    let remote_db = format!("{}/.cct/index.db", source_root);
                                    match rpc.transfer_index(&remote_db).await {
                                        Ok(index_val) => {
                                            if let Some(data_b64) = index_val.get("data").and_then(|v| v.as_str()) {
                                                if let Ok(db_bytes) = base64_decode(data_b64) {
                                                    if let Some(parent) = db_path.parent() {
                                                        let _ = std::fs::create_dir_all(parent);
                                                    }
                                                    if let Err(e) = std::fs::write(&db_path, &db_bytes) {
                                                        error!(error = %e, "写入本地索引数据库失败");
                                                    }
                                                }
                                            }
                                            let _ = app_bg.emit("parse-complete", serde_json::json!({
                                                "project_id": pid,
                                                "statistics": status_val.get("progress"),
                                            }));
                                            update_project_status(&pid, ParseStatus::Completed);
                                        }
                                        Err(e) => {
                                            warn!(error = %e, "索引传输失败，但远程解析已完成");
                                            let _ = app_bg.emit("parse-complete", serde_json::json!({
                                                "project_id": pid,
                                                "statistics": status_val.get("progress"),
                                            }));
                                            update_project_status(&pid, ParseStatus::Completed);
                                        }
                                    }
                                    break;
                                }
                                "failed" => {
                                    let err_msg = status_val.get("error")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("未知错误");
                                    error!(error = %err_msg, "远程解析失败");
                                    let _ = app_bg.emit("parse-error", serde_json::json!({
                                        "project_id": pid,
                                        "error": err_msg,
                                    }));
                                    update_project_status(&pid, ParseStatus::Failed);
                                    break;
                                }
                                "cancelled" => {
                                    info!("远程解析已取消");
                                    update_project_status(&pid, ParseStatus::Failed);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "查询远程解析状态失败");
                            let _ = app_bg.emit("parse-error", serde_json::json!({
                                "project_id": pid,
                                "error": format!("查询解析状态失败: {e}"),
                            }));
                            update_project_status(&pid, ParseStatus::Failed);
                            break;
                        }
                    }
                }

                let _ = rpc.shutdown().await;
            });
        }
    }

    Ok(())
}

/// 取消解析
///
/// 设置全局取消标志，后台解析线程将在下一个文件处理前检测到并中止。
#[tauri::command]
pub fn cancel_parse(project_id: String) -> Result<(), CctError> {
    info!(project_id = %project_id, "Tauri Command: cancel_parse");
    CANCEL_FLAG.store(true, Ordering::SeqCst);
    info!("解析取消标志已设置");
    Ok(())
}

/// 获取解析状态
///
/// 查询项目当前的解析状态。
#[tauri::command]
pub fn get_parse_status(project_id: String) -> Result<String, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_parse_status");

    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    let project = service.get(&uuid)?;

    let status = match project.parse_status {
        ParseStatus::NotStarted => "idle",
        ParseStatus::InProgress => "running",
        ParseStatus::Completed => "completed",
        ParseStatus::Failed => "failed",
    };

    debug!(project_id = %project_id, status = %status, "解析状态查询完成");
    Ok(status.to_string())
}

/// 获取解析统计信息
///
/// 从项目对应的 SQLite 索引数据库读取统计数据。
/// 若数据库尚不存在则返回默认空统计。
#[tauri::command]
pub fn get_parse_statistics(project_id: String) -> Result<ParseStatistics, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_parse_statistics");

    let db_path = index_db_path(&project_id);

    if !db_path.exists() {
        info!(path = %db_path.display(), "索引数据库不存在，返回空统计");
        return Ok(ParseStatistics::default());
    }

    let db = IndexDatabase::open(&db_path)?;
    db.get_statistics()
}

/// 启动增量解析
///
/// 仅解析自上次解析以来发生变更的文件。
#[tauri::command]
pub async fn start_incremental_parse(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<(), CctError> {
    info!(project_id = %project_id, "Tauri Command: start_incremental_parse");

    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    let project = service.get(&uuid)?;

    service.update_parse_status(&uuid, ParseStatus::InProgress)?;
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let source_root = project.source_root.clone();
    let pid = project_id.clone();
    let db_path = index_db_path(&project_id);

    tokio::task::spawn_blocking(move || {
        info!("增量解析后台线程启动");

        let extensions: &[&str] = &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx"];

        let mut db = match IndexDatabase::open(&db_path) {
            Ok(db) => db,
            Err(e) => {
                error!(error = %e, "打开索引数据库失败");
                return;
            }
        };

        let changed_files = match IncrementalParser::detect_changed_files(
            &db,
            std::path::Path::new(&source_root),
            extensions,
        ) {
            Ok(files) => files,
            Err(e) => {
                error!(error = %e, "变更检测失败");
                return;
            }
        };

        if changed_files.is_empty() {
            info!("无文件变更，跳过增量解析");
            let _ = app.emit("parse-complete", serde_json::json!({
                "project_id": pid,
                "statistics": ParseStatistics::default(),
                "message": "无文件变更",
            }));
            return;
        }

        info!(changed_count = changed_files.len(), "检测到变更文件");

        let result = IncrementalParser::run_incremental(
            &mut db,
            std::path::Path::new(&source_root),
            &changed_files,
            |progress| {
                let _ = app.emit("parse-progress", serde_json::json!({
                    "project_id": pid,
                    "total_files": progress.total_files,
                    "parsed_files": progress.parsed_files,
                    "failed_files": progress.failed_files,
                    "percentage": progress.percentage,
                    "current_file": progress.current_file,
                    "symbols_found": progress.symbols_found,
                    "relations_found": progress.relations_found,
                    "elapsed_seconds": progress.elapsed_seconds,
                    "estimated_remaining": progress.estimated_remaining,
                }));
            },
        );

        match result {
            Ok(stats) => {
                info!("增量解析完成");
                let _ = app.emit("parse-complete", serde_json::json!({
                    "project_id": pid,
                    "statistics": serde_json::to_value(&stats).unwrap_or_default(),
                }));
            }
            Err(e) => {
                error!(error = %e, "增量解析失败");
                let _ = app.emit("parse-error", serde_json::json!({
                    "project_id": pid,
                    "error": e.to_string(),
                }));
            }
        }
    });

    Ok(())
}

/// 获取解析错误列表
///
/// 查询 `file_info` 表中 `parse_status = 'failed'` 的记录。
#[tauri::command]
pub fn get_parse_errors(project_id: String) -> Result<Vec<serde_json::Value>, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_parse_errors");

    let db_path = index_db_path(&project_id);
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let db = IndexDatabase::open(&db_path)?;
    let errors = db.query_failed_files()?;

    let result: Vec<serde_json::Value> = errors
        .into_iter()
        .map(|fi| {
            serde_json::json!({
                "file_path": fi.file_path,
                "error_message": fi.error_message,
                "last_modified": fi.last_modified,
            })
        })
        .collect();

    debug!(project_id = %project_id, count = result.len(), "解析错误查询完成");
    Ok(result)
}

// ─── 私有辅助函数 ──────────────────────────────────────────────────

fn update_parse_result(
    app: &tauri::AppHandle,
    pid: &str,
    result: Result<ParseStatistics, CctError>,
) {
    match result {
        Ok(stats) => {
            info!(
                total = stats.total_files,
                parsed = stats.parsed_files,
                symbols = stats.total_symbols,
                "全量解析完成"
            );
            let _ = app.emit("parse-complete", serde_json::json!({
                "project_id": pid,
                "statistics": serde_json::to_value(&stats).unwrap_or_default(),
            }));
            update_project_status(pid, ParseStatus::Completed);
        }
        Err(e) => {
            error!(error = %e, "全量解析失败");
            let _ = app.emit("parse-error", serde_json::json!({
                "project_id": pid,
                "error": e.to_string(),
            }));
            update_project_status(pid, ParseStatus::Failed);
        }
    }
}

fn update_project_status(pid: &str, status: ParseStatus) {
    if let Ok(id) = uuid::Uuid::parse_str(pid) {
        let svc = ProjectService::from_default();
        let _ = svc.update_parse_status(&id, status);
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, CctError> {
    let cleaned: Vec<u8> = input
        .as_bytes()
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();

    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn decode_char(c: u8, table: &[u8]) -> u8 {
        table.iter().position(|&t| t == c).map(|p| p as u8).unwrap_or(0)
    }

    let mut result = Vec::with_capacity(cleaned.len() * 3 / 4);

    for chunk in cleaned.chunks(4) {
        let mut buf = [0u8; 4];
        let mut len = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                break;
            }
            buf[i] = decode_char(b, table);
            len = i + 1;
        }
        if len >= 2 {
            result.push((buf[0] << 2) | (buf[1] >> 4));
        }
        if len >= 3 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if len >= 4 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(result)
}

/// 自动搜索 compile_commands.json
///
/// 依次在源码目录及常见构建子目录中查找，返回第一个找到的路径。
fn find_compile_commands(source_root: &str) -> Option<String> {
    let root = std::path::Path::new(source_root);
    let candidates = [
        root.join("compile_commands.json"),
        root.join("build").join("compile_commands.json"),
        root.join("cmake-build-debug").join("compile_commands.json"),
        root.join("cmake-build-release").join("compile_commands.json"),
        root.join("out").join("compile_commands.json"),
        root.join("out").join("Default").join("compile_commands.json"),
        root.join("builddir").join("compile_commands.json"),
        root.join(".build").join("compile_commands.json"),
    ];

    for path in &candidates {
        if path.exists() {
            info!(path = %path.display(), "自动发现 compile_commands.json");
            return Some(path.display().to_string());
        }
    }

    debug!(root = %source_root, "未找到 compile_commands.json，使用默认编译参数");
    None
}
