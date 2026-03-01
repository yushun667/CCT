use tracing::info;

use cct_core::error::CctError;
use cct_core::models::project::Project;

use crate::services::project_service::{parse_project_id, ProjectService};

/// 创建本地项目
///
/// # 参数
/// - `name`: 项目名称（不可重复）
/// - `source_root`: 源码根目录的绝对路径
#[tauri::command]
pub fn create_local_project(name: String, source_root: String) -> Result<Project, CctError> {
    info!(
        name = %name,
        source_root = %source_root,
        "Tauri Command: create_local_project"
    );
    let service = ProjectService::from_default();
    service.create_local(name, source_root)
}

/// 列出所有项目
#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, CctError> {
    info!("Tauri Command: list_projects");
    let service = ProjectService::from_default();
    service.list()
}

/// 获取单个项目
#[tauri::command]
pub fn get_project(project_id: String) -> Result<Project, CctError> {
    info!(project_id = %project_id, "Tauri Command: get_project");
    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    service.get(&uuid)
}

/// 更新项目
///
/// # 参数
/// - `project_id`: 项目 UUID 字符串
/// - `name`: 新名称（可选）
/// - `compile_db_path`: 新编译数据库路径（可选，空字符串表示清除）
#[tauri::command]
pub fn update_project(
    project_id: String,
    name: Option<String>,
    compile_db_path: Option<String>,
) -> Result<Project, CctError> {
    info!(project_id = %project_id, "Tauri Command: update_project");
    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    service.update(&uuid, name, compile_db_path)
}

/// 删除项目
#[tauri::command]
pub fn delete_project(project_id: String) -> Result<(), CctError> {
    info!(project_id = %project_id, "Tauri Command: delete_project");
    let uuid = parse_project_id(&project_id)?;
    let service = ProjectService::from_default();
    service.delete(&uuid)
}
