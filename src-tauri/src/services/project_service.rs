//! 项目服务 — 项目元数据与注册表
//!
//! 所有与已打开项目相关的数据均保存在项目根目录下的隐藏目录 `.cct/` 中：
//! - `{source_root}/.cct/project.json` — 项目元数据（名称、compile_db_path、排除目录等）
//! - `{source_root}/.cct/index.db` — 索引数据库（由 commands::project_db_path_from_root 使用）
//! 应用数据目录仅保留轻量级注册表 `projects_registry.json`，记录 (id -> source_root) 以便列出与按 id 查找。

use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use cct_core::error::CctError;
use cct_core::models::project::Project;

/// 注册表单项：仅保存 id 与 source_root，用于列表与按 id 查找
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RegistryEntry {
    id: Uuid,
    source_root: String,
}

pub struct ProjectService {
    /// 应用数据目录（用于存放 projects_registry.json）
    data_dir: PathBuf,
    /// 旧版项目目录，用于迁移后不再使用
    #[allow(dead_code)]
    legacy_projects_dir: PathBuf,
}

impl ProjectService {
    pub fn new(data_dir: &Path) -> Self {
        let data_dir = data_dir.to_path_buf();
        let legacy_projects_dir = data_dir.join("projects");
        info!(
            data_dir = %data_dir.display(),
            "ProjectService::new 初始化项目服务（.cct 存储）"
        );
        Self {
            data_dir,
            legacy_projects_dir,
        }
    }

    /// 从默认数据目录构建服务实例
    pub fn from_default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cct");
        Self::new(&data_dir)
    }

    fn registry_path(&self) -> PathBuf {
        self.data_dir.join("projects_registry.json")
    }

    fn read_registry(&self) -> Result<Vec<RegistryEntry>, CctError> {
        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.registry_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let entries: Vec<RegistryEntry> = serde_json::from_str(&content)
            .map_err(|e| CctError::Internal(format!("注册表解析失败: {}", e)))?;
        Ok(entries)
    }

    fn write_registry(&self, entries: &[RegistryEntry]) -> Result<(), CctError> {
        std::fs::create_dir_all(&self.data_dir)?;
        let content = serde_json::to_string_pretty(entries)?;
        std::fs::write(self.registry_path(), content)?;
        Ok(())
    }

    /// 项目元数据文件路径：{source_root}/.cct/project.json
    fn project_file_in_root(source_root: &str) -> PathBuf {
        PathBuf::from(source_root).join(".cct").join("project.json")
    }

    fn ensure_cct_dir(source_root: &str) -> Result<PathBuf, CctError> {
        let dir = PathBuf::from(source_root).join(".cct");
        std::fs::create_dir_all(&dir)
            .map_err(|e| CctError::Internal(format!("创建 .cct 目录失败: {}", e)))?;
        Ok(dir)
    }

    /// 从旧版 {data_dir}/projects/{id}.json 迁移到 .cct/project.json 并写入注册表
    fn migrate_legacy_projects(&self) -> Result<(), CctError> {
        if !self.legacy_projects_dir.exists() {
            return Ok(());
        }
        let mut registry = self.read_registry()?;
        let mut migrated = 0;
        let dir = std::fs::read_dir(&self.legacy_projects_dir);
        let Ok(entries) = dir else { return Ok(()); };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().as_deref() != Some(std::ffi::OsStr::new("json")) {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "迁移时读取旧项目文件失败，跳过");
                    continue;
                }
            };
            let project: Project = match serde_json::from_str(&content) {
                Ok(p) => p,
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "迁移时解析旧项目文件失败，跳过");
                    continue;
                }
            };
            let root = project.source_root.clone();
            if root.is_empty() {
                continue;
            }
            // source_root 在当前机器上不存在时跳过（可能是换盘或换机器后的残留）
            if !Path::new(&root).is_dir() {
                warn!(root = %root, id = %project.id, "迁移跳过：源码目录不存在");
                continue;
            }
            if let Err(e) = Self::ensure_cct_dir(&root) {
                warn!(root = %root, error = %e, "迁移时创建 .cct 目录失败，跳过");
                continue;
            }
            let project_path = Self::project_file_in_root(&root);
            let project_json = serde_json::to_string_pretty(&project)?;
            if let Err(e) = std::fs::write(&project_path, project_json) {
                warn!(root = %root, error = %e, "迁移时写入 .cct/project.json 失败，跳过");
                continue;
            }
            if !registry.iter().any(|e| e.id == project.id) {
                registry.push(RegistryEntry {
                    id: project.id,
                    source_root: root,
                });
                migrated += 1;
            }
            let _ = std::fs::remove_file(&path);
        }
        if migrated > 0 {
            self.write_registry(&registry)?;
            info!(count = migrated, "已从旧版 projects 目录迁移到 .cct");
        }
        Ok(())
    }

    /// 通过 id 从注册表查找 source_root
    fn get_source_root(&self, id: &Uuid) -> Result<String, CctError> {
        let registry = self.read_registry()?;
        registry
            .into_iter()
            .find(|e| e.id == *id)
            .map(|e| e.source_root)
            .ok_or_else(|| CctError::ProjectNotFound(id.to_string()))
    }

    /// 创建本地项目（或返回已有的同目录项目）
    ///
    /// 在 source_root 下创建 .cct/project.json 并写入注册表。
    pub fn create_local(&self, name: String, source_root: String) -> Result<Project, CctError> {
        info!(
            name = %name,
            source_root = %source_root,
            "ProjectService::create_local 创建本地项目"
        );
        self.migrate_legacy_projects()?;

        let src_path = Path::new(&source_root);
        if !src_path.exists() || !src_path.is_dir() {
            error!(path = %source_root, "源码目录无效");
            return Err(CctError::InvalidSourceRoot(source_root));
        }

        // 去除尾部分隔符，保持路径一致性
        let source_root_norm = source_root.trim_end_matches('/').to_string();

        let existing = self.list()?;
        if let Some(found) = existing.iter().find(|p| p.source_root.trim_end_matches('/') == source_root_norm) {
            debug!(id = %found.id, name = %found.name, "目录已有对应项目，直接返回");
            return Ok(found.clone());
        }

        let final_name = if existing.iter().any(|p| p.name == name) {
            let timestamp = Utc::now().format("%m%d-%H%M");
            format!("{}-{}", name, timestamp)
        } else {
            name
        };

        let project = Project::new_local(final_name, source_root_norm.clone());
        Self::ensure_cct_dir(&source_root_norm)?;
        let file = Self::project_file_in_root(&source_root_norm);
        let content = serde_json::to_string_pretty(&project)?;
        std::fs::write(&file, content)?;

        let mut registry = self.read_registry()?;
        registry.push(RegistryEntry {
            id: project.id,
            source_root: source_root_norm,
        });
        self.write_registry(&registry)?;

        debug!(id = %project.id, name = %project.name, "项目已创建并写入 .cct");
        Ok(project)
    }

    /// 创建远程项目
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
        self.migrate_legacy_projects()?;

        let existing = self.list()?;
        if existing.iter().any(|p| p.name == name) {
            error!(name = %name, "项目名称已存在");
            return Err(CctError::ProjectNameExists(name));
        }

        let project = Project::new_remote(name, source_root.clone(), ssh_config);
        // 远程项目没有本地 .cct 目录，仅写入注册表；元数据存于应用目录下的占位 .cct（可选）
        // 为保持“所有数据在项目根”的语义，远程项目仍用 source_root 作为键，元数据存应用目录下
        // 的 projects_meta/{id}.json（仅远程），或我们统一用 registry + 远程时 project 存 data_dir/projects_remote/{id}.json。
        // 需求是“打开的项目”的中间文件在项目根：本地项目有项目根，远程项目“根”在远端，本地没有目录。
        // 因此远程项目仍把 project 存到 data_dir 下某处（例如 data_dir/projects_remote/{id}.json），
        // 仅本地项目使用 .cct/project.json。这样 list/get 需要区分：get 时若 registry 有 source_root，先看本地
        // 是否存在 source_root/.cct/project.json；若不存在则视为远程，从 data_dir/projects_remote/{id}.json 读。
        // 为简化，远程项目也在 registry 里记 (id, source_root)，但 source_root 是远程路径；
        // 元数据存 data_dir/projects_remote/{id}.json，因为远程没有本地 .cct。
        let remote_meta_dir = self.data_dir.join("projects_remote");
        std::fs::create_dir_all(&remote_meta_dir)?;
        let meta_file = remote_meta_dir.join(format!("{}.json", project.id));
        std::fs::write(&meta_file, serde_json::to_string_pretty(&project)?)?;

        let mut registry = self.read_registry()?;
        registry.push(RegistryEntry {
            id: project.id,
            source_root: source_root,
        });
        self.write_registry(&registry)?;

        debug!(id = %project.id, name = %project.name, "远程项目已创建");
        Ok(project)
    }

    /// 列出所有项目，按更新时间倒序
    pub fn list(&self) -> Result<Vec<Project>, CctError> {
        info!("ProjectService::list 列出所有项目");
        self.migrate_legacy_projects()?;

        let registry = self.read_registry()?;
        let mut projects = Vec::new();
        let mut stale = Vec::new();
        let remote_meta_dir = self.data_dir.join("projects_remote");

        for entry in &registry {
            let local_file = Self::project_file_in_root(&entry.source_root);
            if local_file.exists() {
                match std::fs::read_to_string(&local_file) {
                    Ok(content) => match serde_json::from_str::<Project>(&content) {
                        Ok(project) => projects.push(project),
                        Err(e) => {
                            error!(
                                path = %local_file.display(),
                                error = %e,
                                "项目文件解析失败，跳过"
                            );
                        }
                    },
                    Err(e) => {
                        error!(
                            path = %local_file.display(),
                            error = %e,
                            "项目文件读取失败，跳过"
                        );
                    }
                }
            } else {
                let remote_file = remote_meta_dir.join(format!("{}.json", entry.id));
                if remote_file.exists() {
                    match std::fs::read_to_string(&remote_file) {
                        Ok(content) => match serde_json::from_str::<Project>(&content) {
                            Ok(project) => projects.push(project),
                            Err(e) => {
                                error!(path = %remote_file.display(), error = %e, "远程项目文件解析失败");
                            }
                        },
                        Err(e) => {
                            error!(path = %remote_file.display(), error = %e, "远程项目文件读取失败");
                        }
                    }
                } else {
                    stale.push(entry.id);
                }
            }
        }

        if !stale.is_empty() {
            let new_registry: Vec<RegistryEntry> = registry
                .into_iter()
                .filter(|e| !stale.contains(&e.id))
                .collect();
            let _ = self.write_registry(&new_registry);
        }

        projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug!(count = projects.len(), "项目列表加载完成");
        Ok(projects)
    }

    /// 获取单个项目
    pub fn get(&self, project_id: &Uuid) -> Result<Project, CctError> {
        info!(id = %project_id, "ProjectService::get 获取项目");
        let source_root = self.get_source_root(project_id)?;

        let local_file = Self::project_file_in_root(&source_root);
        if local_file.exists() {
            let content = std::fs::read_to_string(&local_file)?;
            let project: Project = serde_json::from_str(&content)?;
            debug!(id = %project_id, name = %project.name, "项目加载成功");
            return Ok(project);
        }

        let remote_file = self.data_dir.join("projects_remote").join(format!("{}.json", project_id));
        if remote_file.exists() {
            let content = std::fs::read_to_string(&remote_file)?;
            let project: Project = serde_json::from_str(&content)?;
            debug!(id = %project_id, name = %project.name, "远程项目加载成功");
            return Ok(project);
        }

        error!(id = %project_id, "项目元数据文件不存在");
        Err(CctError::ProjectNotFound(project_id.to_string()))
    }

    /// 更新项目字段
    pub fn update(
        &self,
        project_id: &Uuid,
        name: Option<String>,
        compile_db_path: Option<String>,
        excluded_dirs: Option<Vec<String>>,
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

        if let Some(dirs) = excluded_dirs {
            project.excluded_dirs = dirs;
        }

        project.updated_at = Utc::now();

        let local_file = Self::project_file_in_root(&project.source_root);
        if local_file.exists() {
            let content = serde_json::to_string_pretty(&project)?;
            std::fs::write(&local_file, content)?;
        } else {
            let remote_file = self
                .data_dir
                .join("projects_remote")
                .join(format!("{}.json", project_id));
            let content = serde_json::to_string_pretty(&project)?;
            std::fs::write(&remote_file, content)?;
        }

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

        let local_file = Self::project_file_in_root(&project.source_root);
        if local_file.exists() {
            let content = serde_json::to_string_pretty(&project)?;
            std::fs::write(&local_file, content)?;
        } else {
            let remote_file = self
                .data_dir
                .join("projects_remote")
                .join(format!("{}.json", project_id));
            let content = serde_json::to_string_pretty(&project)?;
            std::fs::write(&remote_file, content)?;
        }

        debug!(id = %project_id, "解析状态已更新");
        Ok(())
    }

    /// 删除项目（从注册表移除；.cct 目录保留，用户可手动删除）
    pub fn delete(&self, project_id: &Uuid) -> Result<(), CctError> {
        info!(id = %project_id, "ProjectService::delete 删除项目");
        let project = self.get(project_id)?;
        let mut registry = self.read_registry()?;
        let len_before = registry.len();
        registry.retain(|e| e.id != *project_id);
        if registry.len() == len_before {
            error!(id = %project_id, "项目不在注册表中");
            return Err(CctError::ProjectNotFound(project_id.to_string()));
        }
        self.write_registry(&registry)?;
        let local_file = Self::project_file_in_root(&project.source_root);
        if !local_file.exists() {
            let remote_file = self
                .data_dir
                .join("projects_remote")
                .join(format!("{}.json", project_id));
            let _ = std::fs::remove_file(&remote_file);
        }
        debug!(id = %project_id, "已从注册表删除");
        Ok(())
    }
}

/// 将字符串形式的项目 ID 解析为 UUID
pub fn parse_project_id(id: &str) -> Result<Uuid, CctError> {
    Uuid::parse_str(id).map_err(|_| CctError::ProjectNotFound(id.to_string()))
}
