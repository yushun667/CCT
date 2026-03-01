use rusqlite::params;
use tracing::{debug, info};

use crate::error::CctError;
use crate::indexer::database::{row_to_symbol, symbol_kind_to_str};
use crate::indexer::database::IndexDatabase;
use crate::models::symbol::{Symbol, SymbolKind};

const SYMBOL_COLUMNS: &str =
    "id, name, qualified_name, kind, sub_kind, file_path, \
     line, column, end_line, is_definition, return_type, \
     parameters, access, attributes, project_id";

/// 符号搜索引擎 — 模糊搜索与精确过滤
///
/// # 设计说明
/// 使用 SQL LIKE 实现前缀/子串匹配，为前端全局搜索提供后端支持。
/// 后续可扩展为 FTS5 全文搜索。
pub struct SymbolSearchEngine;

impl SymbolSearchEngine {
    /// 模糊搜索符号（名称子串匹配）
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `query`: 搜索关键词
    /// - `limit`: 最大返回条数
    pub fn search(db: &IndexDatabase, query: &str, limit: usize) -> Result<Vec<Symbol>, CctError> {
        info!(query = %query, limit = limit, "SymbolSearchEngine::search 模糊搜索符号");

        let conn = db.conn();
        let pattern = format!("%{query}%");

        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {SYMBOL_COLUMNS} FROM symbols \
                 WHERE name LIKE ?1 OR qualified_name LIKE ?1 \
                 ORDER BY \
                   CASE WHEN name = ?2 THEN 0 \
                        WHEN name LIKE ?3 THEN 1 \
                        ELSE 2 END, \
                   length(name) \
                 LIMIT ?4"
            ))
            .map_err(|e| CctError::Database(e.to_string()))?;

        let prefix_pattern = format!("{query}%");
        let results: Vec<Symbol> = stmt
            .query_map(params![pattern, query, prefix_pattern, limit as i64], |row| {
                row_to_symbol(row)
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "搜索完成");
        Ok(results)
    }

    /// 按符号类型搜索
    ///
    /// # 参数
    /// - `kind`: 过滤的符号类型（Function / Variable / Type / Macro）
    pub fn search_by_kind(
        db: &IndexDatabase,
        query: &str,
        kind: SymbolKind,
        limit: usize,
    ) -> Result<Vec<Symbol>, CctError> {
        info!(
            query = %query,
            kind = %kind,
            limit = limit,
            "SymbolSearchEngine::search_by_kind 按类型搜索符号"
        );

        let conn = db.conn();
        let pattern = format!("%{query}%");
        let kind_str = symbol_kind_to_str(&kind);

        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {SYMBOL_COLUMNS} FROM symbols \
                 WHERE (name LIKE ?1 OR qualified_name LIKE ?1) AND kind = ?2 \
                 ORDER BY length(name) \
                 LIMIT ?3"
            ))
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<Symbol> = stmt
            .query_map(params![pattern, kind_str, limit as i64], |row| {
                row_to_symbol(row)
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "按类型搜索完成");
        Ok(results)
    }

    /// 查询指定文件中的所有符号
    ///
    /// # 参数
    /// - `file_path`: 文件路径
    pub fn search_by_file(
        db: &IndexDatabase,
        file_path: &str,
    ) -> Result<Vec<Symbol>, CctError> {
        info!(file = %file_path, "SymbolSearchEngine::search_by_file 查询文件符号");

        let conn = db.conn();
        let mut stmt = conn
            .prepare_cached(&format!(
                "SELECT {SYMBOL_COLUMNS} FROM symbols \
                 WHERE file_path = ?1 \
                 ORDER BY line"
            ))
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<Symbol> = stmt
            .query_map(params![file_path], |row| row_to_symbol(row))
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), file = %file_path, "文件符号查询完成");
        Ok(results)
    }
}
