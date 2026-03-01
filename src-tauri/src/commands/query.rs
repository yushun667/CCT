use tracing::info;

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::models::relation::{CallRelation, ReferenceRelation};
use cct_core::models::symbol::{Symbol, SymbolKind};
use cct_core::query::{
    CallQueryEngine, PathFinder, ReferenceQueryEngine, SymbolSearchEngine,
};

/// 打开项目对应的索引数据库
fn open_project_db(project_id: &str) -> Result<IndexDatabase, CctError> {
    super::open_project_index_db(project_id)
}

/// 按 ID 列表批量获取符号
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `ids`: 符号 ID 列表
#[tauri::command]
pub fn get_symbols_by_ids(
    project_id: String,
    ids: Vec<i64>,
) -> Result<Vec<Symbol>, CctError> {
    info!(
        project_id = %project_id,
        count = ids.len(),
        "Tauri Command: get_symbols_by_ids"
    );

    let db = open_project_db(&project_id)?;
    SymbolSearchEngine::get_by_ids(&db, &ids)
}

/// 搜索符号 — 支持模糊搜索和按类型过滤
///
/// # 参数
/// - `project_id`: 项目 UUID
/// - `query`: 搜索关键词
/// - `kind`: 可选的符号类型过滤（"function" / "variable" / "type" / "macro"）
/// - `limit`: 最大返回条数，默认 50
#[tauri::command]
pub fn search_symbols(
    project_id: String,
    query: String,
    kind: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<Symbol>, CctError> {
    info!(
        project_id = %project_id,
        query = %query,
        kind = ?kind,
        "Tauri Command: search_symbols"
    );

    let db = open_project_db(&project_id)?;
    let limit = limit.unwrap_or(50);

    match kind.as_deref() {
        Some("function") => {
            SymbolSearchEngine::search_by_kind(&db, &query, SymbolKind::Function, limit)
        }
        Some("variable") => {
            SymbolSearchEngine::search_by_kind(&db, &query, SymbolKind::Variable, limit)
        }
        Some("type") => {
            SymbolSearchEngine::search_by_kind(&db, &query, SymbolKind::Type, limit)
        }
        Some("macro") => {
            SymbolSearchEngine::search_by_kind(&db, &query, SymbolKind::Macro, limit)
        }
        _ => SymbolSearchEngine::search(&db, &query, limit),
    }
}

/// 查询调用者
#[tauri::command]
pub fn query_callers(
    project_id: String,
    symbol_id: i64,
    depth: Option<u32>,
) -> Result<Vec<CallRelation>, CctError> {
    info!(
        project_id = %project_id,
        symbol_id = symbol_id,
        depth = ?depth,
        "Tauri Command: query_callers"
    );

    let db = open_project_db(&project_id)?;
    CallQueryEngine::query_callers(&db, symbol_id, depth.unwrap_or(1))
}

/// 查询被调用者
#[tauri::command]
pub fn query_callees(
    project_id: String,
    symbol_id: i64,
    depth: Option<u32>,
) -> Result<Vec<CallRelation>, CctError> {
    info!(
        project_id = %project_id,
        symbol_id = symbol_id,
        depth = ?depth,
        "Tauri Command: query_callees"
    );

    let db = open_project_db(&project_id)?;
    CallQueryEngine::query_callees(&db, symbol_id, depth.unwrap_or(1))
}

/// 查询符号引用
#[tauri::command]
pub fn query_references(
    project_id: String,
    symbol_id: i64,
) -> Result<Vec<ReferenceRelation>, CctError> {
    info!(
        project_id = %project_id,
        symbol_id = symbol_id,
        "Tauri Command: query_references"
    );

    let db = open_project_db(&project_id)?;
    ReferenceQueryEngine::query_references(&db, symbol_id)
}

/// 查询两个符号之间的最短调用路径
#[tauri::command]
pub fn query_call_path(
    project_id: String,
    from_id: i64,
    to_id: i64,
) -> Result<Vec<i64>, CctError> {
    info!(
        project_id = %project_id,
        from_id = from_id,
        to_id = to_id,
        "Tauri Command: query_call_path"
    );

    let db = open_project_db(&project_id)?;
    PathFinder::find_shortest_path(&db, from_id, to_id)
}
