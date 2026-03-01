use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::Emitter;
use tracing::{debug, error, info};

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::graph::ParseStatistics;
use cct_core::models::project::ParseStatus;
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
        "开始全量解析"
    );

    service.update_parse_status(&uuid, ParseStatus::InProgress)?;
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let source_root = project.source_root.clone();
    let compile_db = project.compile_db_path.clone();
    let pid = project_id.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();

    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || {
        let app = app_bg;
        info!("解析后台线程启动");

        let scheduler = ParseScheduler::new(None);
        let compile_db_path = compile_db.as_deref().map(std::path::Path::new);

        let result = scheduler.schedule_parse(
            std::path::Path::new(&source_root),
            compile_db_path,
            |progress| {
                if CANCEL_FLAG.load(Ordering::SeqCst) || cancel_clone.load(Ordering::SeqCst) {
                    return;
                }

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

                let uuid = uuid::Uuid::parse_str(&pid).ok();
                if let Some(id) = uuid {
                    let svc = ProjectService::from_default();
                    let _ = svc.update_parse_status(&id, ParseStatus::Completed);
                }
            }
            Err(e) => {
                error!(error = %e, "全量解析失败");

                let _ = app.emit("parse-error", serde_json::json!({
                    "project_id": pid,
                    "error": e.to_string(),
                }));

                let uuid = uuid::Uuid::parse_str(&pid).ok();
                if let Some(id) = uuid {
                    let svc = ProjectService::from_default();
                    let _ = svc.update_parse_status(&id, ParseStatus::Failed);
                }
            }
        }
    });

    app.emit("parse-progress", serde_json::json!({
        "project_id": project_id,
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
