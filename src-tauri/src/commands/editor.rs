use serde::Serialize;
use tracing::{info, debug};

use cct_core::error::CctError;
use cct_core::models::symbol::Symbol;
use cct_core::query::SymbolSearchEngine;

/// 目录条目 — 供前端文件树渲染
#[derive(Debug, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// 列出目录内容 — 供前端文件树懒加载
///
/// 返回指定目录下的直接子条目，目录排在前面，文件排在后面，各自按名称排序。
/// 跳过隐藏文件（以 `.` 开头）和 macOS 的 `._` 元数据文件。
#[tauri::command]
pub fn list_directory(dir_path: String) -> Result<Vec<DirEntry>, CctError> {
    debug!(dir = %dir_path, "Tauri Command: list_directory");

    let path = std::path::Path::new(&dir_path);
    if !path.exists() || !path.is_dir() {
        return Err(CctError::InvalidSourceRoot(dir_path));
    }

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path).map_err(|e| {
        CctError::ParseFileRead(format!("读取目录失败 {dir_path}: {e}"))
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with("._") {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let item = DirEntry {
            name: name.clone(),
            path: entry.path().display().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        };

        if metadata.is_dir() {
            dirs.push(item);
        } else {
            files.push(item);
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.append(&mut files);

    Ok(dirs)
}

/// 读取文件内容 — 供前端 Monaco 编辑器显示
///
/// # 参数
/// - `file_path`: 文件绝对路径
///
/// # 安全说明
/// 仅读取文件内容，不做写入操作。
#[tauri::command]
pub fn read_file_content(file_path: String) -> Result<String, CctError> {
    info!(file = %file_path, "Tauri Command: read_file_content");

    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(CctError::ParseFileRead(format!("文件不存在: {file_path}")));
    }

    std::fs::read_to_string(path).map_err(|e| {
        CctError::ParseFileRead(format!("读取文件失败 {file_path}: {e}"))
    })
}

/// 获取指定文件中的所有符号
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `file_path`: 文件路径
#[tauri::command]
pub fn get_file_symbols(
    project_id: String,
    file_path: String,
) -> Result<Vec<Symbol>, CctError> {
    info!(
        project_id = %project_id,
        file = %file_path,
        "Tauri Command: get_file_symbols"
    );

    let db = super::open_project_index_db(&project_id)?;
    SymbolSearchEngine::search_by_file(&db, &file_path)
}
