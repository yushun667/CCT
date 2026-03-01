use cct_core::config::AppConfig;
use cct_core::error::CctError;
use tracing::{debug, info};

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
