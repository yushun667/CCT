use std::path::PathBuf;

use tracing::info;

use cct_core::analysis::{
    CustomRule, IoctlCommand, IpcService, LinuxKernelAnalyzer, OpenHarmonyAnalyzer, RuleEngine,
    RuleMatch, SyscallInfo,
};
use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::symbol::Symbol;

fn open_project_db(project_id: &str) -> Result<IndexDatabase, CctError> {
    super::open_project_index_db(project_id)
}

/// 列出项目中所有识别到的 Linux 系统调用
///
/// # 参数
/// - `project_id`: 项目 UUID
#[tauri::command]
pub fn list_syscalls(project_id: String) -> Result<Vec<SyscallInfo>, CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: list_syscalls"
    );
    let db = open_project_db(&project_id)?;
    LinuxKernelAnalyzer::find_syscall_definitions(&db)
}

/// 追踪系统调用的执行路径
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `syscall_name`: 系统调用名称
/// - `depth`: 最大追踪深度（默认 5）
#[tauri::command]
pub fn get_syscall_path(
    project_id: String,
    syscall_name: String,
    depth: Option<u32>,
) -> Result<Vec<Symbol>, CctError> {
    info!(
        project_id = %project_id,
        syscall = %syscall_name,
        depth = ?depth,
        "Tauri Command: get_syscall_path"
    );
    let db = open_project_db(&project_id)?;
    LinuxKernelAnalyzer::trace_syscall_path(&db, &syscall_name, depth.unwrap_or(5))
}

/// 列出项目中所有识别到的 ioctl 命令
///
/// # 参数
/// - `project_id`: 项目 UUID
#[tauri::command]
pub fn list_ioctl_commands(project_id: String) -> Result<Vec<IoctlCommand>, CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: list_ioctl_commands"
    );
    let db = open_project_db(&project_id)?;
    LinuxKernelAnalyzer::find_ioctl_handlers(&db)
}

/// 列出项目中所有识别到的 OpenHarmony IPC 服务
///
/// # 参数
/// - `project_id`: 项目 UUID
#[tauri::command]
pub fn list_ipc_services(project_id: String) -> Result<Vec<IpcService>, CctError> {
    info!(
        project_id = %project_id,
        "Tauri Command: list_ipc_services"
    );
    let db = open_project_db(&project_id)?;
    OpenHarmonyAnalyzer::find_ipc_services(&db)
}

/// 追踪 IPC 服务的通信路径
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `service_name`: 服务名称
#[tauri::command]
pub fn get_ipc_call_path(
    project_id: String,
    service_name: String,
) -> Result<Vec<Symbol>, CctError> {
    info!(
        project_id = %project_id,
        service = %service_name,
        "Tauri Command: get_ipc_call_path"
    );
    let db = open_project_db(&project_id)?;
    OpenHarmonyAnalyzer::trace_ipc_call(&db, &service_name)
}

/// 加载自定义分析规则（仅加载并校验，不执行）
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `rules_path`: YAML 规则文件的绝对路径
#[tauri::command]
pub fn load_custom_rules(
    project_id: String,
    rules_path: String,
) -> Result<Vec<CustomRule>, CctError> {
    info!(
        project_id = %project_id,
        rules_path = %rules_path,
        "Tauri Command: load_custom_rules"
    );
    let path = PathBuf::from(&rules_path);
    RuleEngine::load_rules(&path)
}

/// 加载并执行自定义分析规则
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `rules_path`: YAML 规则文件的绝对路径
#[tauri::command]
pub fn apply_custom_rules(
    project_id: String,
    rules_path: String,
) -> Result<Vec<RuleMatch>, CctError> {
    info!(
        project_id = %project_id,
        rules_path = %rules_path,
        "Tauri Command: apply_custom_rules"
    );
    let db = open_project_db(&project_id)?;
    let path = PathBuf::from(&rules_path);
    let rules = RuleEngine::load_rules(&path)?;
    RuleEngine::apply_rules(&db, &rules)
}
