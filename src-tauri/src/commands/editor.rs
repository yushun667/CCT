use std::path::PathBuf;

use tracing::info;

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::symbol::Symbol;
use cct_core::query::SymbolSearchEngine;

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

    let db_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cct")
        .join("index")
        .join(format!("{project_id}.db"));

    if !db_path.exists() {
        return Err(CctError::Database(format!(
            "索引数据库不存在: {}",
            db_path.display()
        )));
    }

    let db = IndexDatabase::open(&db_path)?;
    SymbolSearchEngine::search_by_file(&db, &file_path)
}
