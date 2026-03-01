use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 项目实体 — 对应 doc/01 §3.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub project_type: ProjectType,
    /// 源码根目录（本地路径或远程绝对路径）
    pub source_root: String,
    pub ssh_config: Option<SSHConfig>,
    pub agent_config: Option<AgentConfig>,
    pub compile_db_path: Option<String>,
    /// 解析时忽略的目录名列表（例如 "test", "third_party"）
    #[serde(default)]
    pub excluded_dirs: Vec<String>,
    pub module_definitions: Vec<ModuleDef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_parse_at: Option<DateTime<Utc>>,
    pub parse_status: ParseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectType {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParseStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

/// SSH 连接配置 — 对应 doc/01 §3.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SSHAuthMethod,
    pub key_path: Option<String>,
    /// 指向加密认证信息的引用 ID（密码/密码短语存储在系统密钥链中）
    pub auth_ref: String,
    /// 跳板机地址，如 `user@bastion:22`
    pub proxy_jump: Option<String>,
    pub keep_alive_interval: u32,
    pub connect_timeout: u32,
    pub known_hosts_policy: HostKeyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SSHAuthMethod {
    Key {
        key_path: String,
        passphrase_ref: Option<String>,
    },
    Password {
        password_ref: String,
    },
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostKeyPolicy {
    Accept,
    Reject,
    AskUser,
}

/// 远程 Agent 配置 — 对应 doc/01 §3.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub install_path: String,
    pub data_dir: String,
    pub mode: AgentMode,
    pub max_threads: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentMode {
    OnDemand,
    Daemon,
}

/// 模块定义 — 用于可视化分组和依赖分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub match_type: MatchType,
    pub patterns: Vec<String>,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatchType {
    Directory,
    FileList,
    Regex,
}

/// 解析进度 — 前后端事件通道载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseProgress {
    /// 当前阶段: "scanning" | "parsing" | "indexing" | "completed"
    #[serde(default = "default_phase")]
    pub phase: String,
    pub total_files: u64,
    pub parsed_files: u64,
    pub failed_files: u64,
    pub percentage: f32,
    pub current_file: String,
    pub symbols_found: u64,
    pub relations_found: u64,
    pub elapsed_seconds: f64,
    pub estimated_remaining: f64,
}

fn default_phase() -> String {
    "parsing".to_string()
}

/// 远程状态 — doc/01 UC1.8
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub ssh_state: SshState,
    pub agent_state: AgentState,
    pub agent_version: Option<String>,
    pub server_info: Option<ServerInfo>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SshState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentState {
    NotInstalled,
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub hostname: String,
    pub os: String,
    pub cpu_cores: u32,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub disk_free_mb: u64,
}

impl Default for SSHConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_method: SSHAuthMethod::Agent,
            key_path: None,
            auth_ref: String::new(),
            proxy_jump: None,
            keep_alive_interval: 30,
            connect_timeout: 15,
            known_hosts_policy: HostKeyPolicy::AskUser,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            install_path: "~/.cct/agent/".to_string(),
            data_dir: "~/.cct/data/".to_string(),
            mode: AgentMode::OnDemand,
            max_threads: None,
            max_memory_mb: None,
            version: None,
        }
    }
}

impl Project {
    pub fn new_local(name: String, source_root: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            project_type: ProjectType::Local,
            source_root,
            ssh_config: None,
            agent_config: None,
            compile_db_path: None,
            excluded_dirs: Vec::new(),
            module_definitions: Vec::new(),
            created_at: now,
            updated_at: now,
            last_parse_at: None,
            parse_status: ParseStatus::NotStarted,
        }
    }

    pub fn new_remote(name: String, source_root: String, ssh_config: SSHConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            project_type: ProjectType::Remote,
            source_root,
            ssh_config: Some(ssh_config),
            agent_config: Some(AgentConfig::default()),
            compile_db_path: None,
            excluded_dirs: Vec::new(),
            module_definitions: Vec::new(),
            created_at: now,
            updated_at: now,
            last_parse_at: None,
            parse_status: ParseStatus::NotStarted,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.project_type == ProjectType::Remote
    }
}
