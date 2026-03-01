use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::{debug, error, info};
use uuid::Uuid;

use cct_core::error::CctError;
use cct_core::models::project::Project;

/// 项目服务 — 负责项目 CRUD 操作
///
/// # 设计说明（外观模式）
/// 封装文件系统级别的 JSON 持久化操作，为命令层提供业务语义接口。
/// 每个项目以 `{projects_dir}/{id}.json` 的形式独立存储，
/// 便于手动备份和版本管理。
pub struct ProjectService {
    projects_dir: PathBuf,
}

impl ProjectService {
    pub fn new(data_dir: &Path) -> Self {
        let projects_dir = data_dir.join("projects");
        info!(
            projects_dir = %projects_dir.display(),
            "ProjectService::new 初始化项目服务"
        );
        Self { projects_dir }
    }

    /// 从默认数据目录构建服务实例
    pub fn from_default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cct");
        Self::new(&data_dir)
    }

    fn ensure_dir(&self) -> Result<(), CctError> {
        std::fs::create_dir_all(&self.projects_dir)?;
        Ok(())
    }

    fn project_file(&self, id: &Uuid) -> PathBuf {
        self.projects_dir.join(format!("{}.json", id))
    }

    /// 创建本地项目
    ///
    /// 校验源码目录存在且名称不重复后，持久化到 JSON 文件。
    pub fn create_local(&self, name: String, source_root: String) -> Result<Project, CctError> {
        info!(
            name = %name,
            source_root = %source_root,
            "ProjectService::create_local 创建本地项目"
        );
        self.ensure_dir()?;

        let src_path = Path::new(&source_root);
        if !src_path.exists() || !src_path.is_dir() {
            error!(path = %source_root, "源码目录无效");
            return Err(CctError::InvalidSourceRoot(source_root));
        }

        let existing = self.list()?;
        if existing.iter().any(|p| p.name == name) {
            error!(name = %name, "项目名称已存在");
            return Err(CctError::ProjectNameExists(name));
        }

        let project = Project::new_local(name, source_root);
        let file = self.project_file(&project.id);
        let content = serde_json::to_string_pretty(&project)?;
        std::fs::write(&file, content)?;

        debug!(id = %project.id, name = %project.name, "项目已创建");
        Ok(project)
    }

    /// 创建远程项目
    ///
    /// 不校验本地目录存在性，接受 SSH 配置和远程源码路径。
    pub fn create_remote(
        &self,
        name: String,
        source_root: String,
        ssh_config: cct_core::models::project::SSHConfig,
    ) -> Result<Project, CctError> {
        info!(
            name = %name,
            source_root = %source_root,
            host = %ssh_config.host,
            "ProjectService::create_remote 创建远程项目"
        );
        self.ensure_dir()?;

        let existing = self.list()?;
        if existing.iter().any(|p| p.name == name) {
            error!(name = %name, "项目名称已存在");
            return Err(CctError::ProjectNameExists(name));
        }

        let project = Project::new_remote(name, source_root, ssh_config);
        let file = self.project_file(&project.id);
        let content = serde_json::to_string_pretty(&project)?;
        std::fs::write(&file, content)?;

        debug!(id = %project.id, name = %project.name, "远程项目已创建");
        Ok(project)
    }

    /// 列出所有项目，按更新时间倒序
    pub fn list(&self) -> Result<Vec<Project>, CctError> {
        info!("ProjectService::list 列出所有项目");
        self.ensure_dir()?;

        let mut projects = Vec::new();
        for entry in std::fs::read_dir(&self.projects_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<Project>(&content) {
                        Ok(project) => projects.push(project),
                        Err(e) => {
                            error!(
                                path = %path.display(),
                                error = %e,
                                "项目文件解析失败，跳过"
                            );
                        }
                    },
                    Err(e) => {
                        error!(
                            path = %path.display(),
                            error = %e,
                            "项目文件读取失败，跳过"
                        );
                    }
                }
            }
        }

        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug!(count = projects.len(), "项目列表加载完成");
        Ok(projects)
    }

    /// 获取单个项目
    pub fn get(&self, project_id: &Uuid) -> Result<Project, CctError> {
        info!(id = %project_id, "ProjectService::get 获取项目");
        let file = self.project_file(project_id);
        if !file.exists() {
            error!(id = %project_id, "项目不存在");
            return Err(CctError::ProjectNotFound(project_id.to_string()));
        }
        let content = std::fs::read_to_string(&file)?;
        let project: Project = serde_json::from_str(&content)?;
        debug!(id = %project_id, name = %project.name, "项目加载成功");
        Ok(project)
    }

    /// 更新项目字段（名称、编译数据库路径）
    pub fn update(
        &self,
        project_id: &Uuid,
        name: Option<String>,
        compile_db_path: Option<String>,
    ) -> Result<Project, CctError> {
        info!(id = %project_id, "ProjectService::update 更新项目");
        let mut project = self.get(project_id)?;

        if let Some(ref new_name) = name {
            let existing = self.list()?;
            if existing
                .iter()
                .any(|p| p.name == *new_name && p.id != *project_id)
            {
                error!(name = %new_name, "项目名称已被占用");
                return Err(CctError::ProjectNameExists(new_name.clone()));
            }
            project.name = new_name.clone();
        }

        if let Some(db_path) = compile_db_path {
            project.compile_db_path = if db_path.is_empty() {
                None
            } else {
                Some(db_path)
            };
        }

        project.updated_at = Utc::now();

        let file = self.project_file(project_id);
        let content = serde_json::to_string_pretty(&project)?;
        std::fs::write(&file, content)?;

        debug!(id = %project_id, name = %project.name, "项目已更新");
        Ok(project)
    }

    /// 更新项目的解析状态
    pub fn update_parse_status(
        &self,
        project_id: &Uuid,
        status: cct_core::models::project::ParseStatus,
    ) -> Result<(), CctError> {
        info!(id = %project_id, status = ?status, "ProjectService::update_parse_status 更新解析状态");
        let mut project = self.get(project_id)?;
        project.parse_status = status;
        project.updated_at = Utc::now();

        let file = self.project_file(project_id);
        let content = serde_json::to_string_pretty(&project)?;
        std::fs::write(&file, content)?;

        debug!(id = %project_id, "解析状态已更新");
        Ok(())
    }

    /// 删除项目
    pub fn delete(&self, project_id: &Uuid) -> Result<(), CctError> {
        info!(id = %project_id, "ProjectService::delete 删除项目");
        let file = self.project_file(project_id);
        if !file.exists() {
            error!(id = %project_id, "项目不存在");
            return Err(CctError::ProjectNotFound(project_id.to_string()));
        }
        std::fs::remove_file(&file)?;
        debug!(id = %project_id, "项目已删除");
        Ok(())
    }
}

/// 将字符串形式的项目 ID 解析为 UUID
pub fn parse_project_id(id: &str) -> Result<Uuid, CctError> {
    Uuid::parse_str(id).map_err(|_| CctError::ProjectNotFound(id.to_string()))
}
