use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

/// 全局应用配置 — 对应 doc/08 §4.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub log: LogConfig,
    pub parse: ParseConfig,
    pub ui: UiConfig,
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub max_file_size_mb: u64,
    pub max_file_count: u32,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseConfig {
    pub max_threads: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub file_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub language: String,
    pub font_size: u32,
    pub sidebar_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key_ref: Option<String>,
    pub base_url: Option<String>,
    pub privacy_mode: PrivacyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyMode {
    Full,
    Anonymized,
    Local,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            log: LogConfig::default(),
            parse: ParseConfig::default(),
            ui: UiConfig::default(),
            ai: AiConfig::default(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            max_file_size_mb: 10,
            max_file_count: 10,
            retention_days: 30,
        }
    }
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            max_threads: None,
            max_memory_mb: None,
            file_extensions: vec![
                "c".to_string(),
                "cc".to_string(),
                "cpp".to_string(),
                "cxx".to_string(),
                "h".to_string(),
                "hh".to_string(),
                "hpp".to_string(),
                "hxx".to_string(),
            ],
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: "zh-CN".to_string(),
            font_size: 14,
            sidebar_width: 280,
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            api_key_ref: None,
            base_url: None,
            privacy_mode: PrivacyMode::Local,
        }
    }
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cct")
}

impl AppConfig {
    /// 从文件加载配置，不存在则返回默认值
    pub fn load(config_path: &Path) -> Self {
        info!("加载配置文件: {}", config_path.display());
        match std::fs::read_to_string(config_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => {
                    debug!("配置加载成功");
                    config
                }
                Err(e) => {
                    error!("配置文件解析失败, 使用默认配置: {}", e);
                    Self::default()
                }
            },
            Err(_) => {
                info!("配置文件不存在, 使用默认配置");
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self, config_path: &Path) -> Result<(), crate::error::CctError> {
        info!("保存配置文件: {}", config_path.display());
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        debug!("配置保存成功");
        Ok(())
    }

    /// 获取日志目录
    pub fn log_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// 获取项目数据目录
    pub fn projects_dir(&self) -> PathBuf {
        self.data_dir.join("projects")
    }

    /// 获取索引数据目录
    pub fn index_dir(&self) -> PathBuf {
        self.data_dir.join("index")
    }
}
