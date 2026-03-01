use std::path::PathBuf;

use tauri::Emitter;
use tracing::{error, info, warn};

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::graph::ParseStatistics;
use cct_core::models::project::ParseStatus;

use crate::services::project_service::{parse_project_id, ProjectService};

/// 启动全量解析
///
/// 验证项目存在后异步发射解析进度事件。
/// 当前为占位实现，后续将对接 cct-core 解析调度器。
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

    // 更新项目状态为解析中
    service.update_parse_status(&uuid, ParseStatus::InProgress)?;

    // 通知前端解析已开始
    app.emit("parse-progress", serde_json::json!({
        "project_id": project_id,
        "total_files": 0,
        "parsed_files": 0,
        "percentage": 0.0,
        "current_file": "",
        "symbols_found": 0,
        "relations_found": 0,
        "elapsed_seconds": 0.0,
        "estimated_remaining": 0.0
    })).map_err(|e| {
        error!(error = %e, "发送解析进度事件失败");
        CctError::Internal(e.to_string())
    })?;

    // TODO: 对接 cct-core 解析调度器，在独立线程中执行解析
    warn!("全量解析尚未实现，当前为占位逻辑");

    Ok(())
}

/// 取消解析（占位）
#[tauri::command]
pub fn cancel_parse(project_id: String) -> Result<(), CctError> {
    info!(project_id = %project_id, "Tauri Command: cancel_parse");
    warn!("cancel_parse 尚未实现");
    Ok(())
}

/// 获取解析状态（占位，返回 "idle"）
#[tauri::command]
pub fn get_parse_status(project_id: String) -> Result<String, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_parse_status");
    Ok("idle".to_string())
}

/// 获取解析统计信息
///
/// 从项目对应的 SQLite 索引数据库读取统计数据。
/// 若数据库尚不存在则返回默认空统计。
#[tauri::command]
pub fn get_parse_statistics(project_id: String) -> Result<ParseStatistics, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_parse_statistics");

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cct");
    let db_path = data_dir
        .join("index")
        .join(format!("{}.db", project_id));

    if !db_path.exists() {
        info!(path = %db_path.display(), "索引数据库不存在，返回空统计");
        return Ok(ParseStatistics::default());
    }

    let db = IndexDatabase::open(&db_path)?;
    db.get_statistics()
}
