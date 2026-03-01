pub mod ai;
pub mod analysis;
pub mod editor;
pub mod graph;
pub mod parse;
pub mod project;
pub mod query;
pub mod remote;

use std::path::PathBuf;

use cct_core::config::AppConfig;
use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use tracing::{debug, info};

/// 计算项目索引数据库路径 — 保存在项目目录下的 `.cct/index.db`
///
/// 通过 project_id 查找项目的 source_root，将数据库存储在
/// `{source_root}/.cct/index.db`，使索引数据跟随项目目录。
pub fn project_db_path(project_id: &str) -> Result<PathBuf, CctError> {
    let service = crate::services::project_service::ProjectService::from_default();
    let uuid = crate::services::project_service::parse_project_id(project_id)?;
    let project = service.get(&uuid)?;
    project_db_path_from_root(&project.source_root)
}

/// 直接从 source_root 计算数据库路径（已有项目对象时使用）
pub fn project_db_path_from_root(source_root: &str) -> Result<PathBuf, CctError> {
    let db_dir = PathBuf::from(source_root).join(".cct");
    std::fs::create_dir_all(&db_dir).map_err(|e| {
        CctError::Internal(format!("创建 .cct 目录失败: {e}"))
    })?;
    Ok(db_dir.join("index.db"))
}

/// 打开项目索引数据库的通用辅助函数
pub fn open_project_index_db(project_id: &str) -> Result<IndexDatabase, CctError> {
    let db_path = project_db_path(project_id)?;
    if !db_path.exists() {
        return Err(CctError::Database(format!(
            "索引数据库不存在: {}",
            db_path.display()
        )));
    }
    debug!(path = %db_path.display(), "打开项目索引数据库");
    IndexDatabase::open(&db_path)
}

/// 获取应用配置
#[tauri::command]
pub fn get_app_config() -> Result<AppConfig, CctError> {
    info!("Tauri Command: get_app_config");
    let config_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cct")
        .join("config.json");
    let config = AppConfig::load(&config_path);
    debug!("配置加载完成: {:?}", config.data_dir);
    Ok(config)
}

/// 保存应用配置
#[tauri::command]
pub fn save_app_config(config: AppConfig) -> Result<(), CctError> {
    info!("Tauri Command: save_app_config");
    let config_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cct")
        .join("config.json");
    config.save(&config_path)
}

/// 获取应用版本号
#[tauri::command]
pub fn get_app_version() -> String {
    info!("Tauri Command: get_app_version");
    env!("CARGO_PKG_VERSION").to_string()
}
