use std::collections::HashSet;

use rusqlite::params;
use tracing::{debug, info};

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;
use crate::models::relation::CallRelation;

/// 调用关系查询引擎 — 支持多层级递归查询
///
/// # 设计说明
/// 通过 BFS 实现指定深度的调用链查询，避免递归 SQL 的性能问题。
/// `depth` 参数控制递归层数，防止在大型代码库中查询失控。
pub struct CallQueryEngine;

impl CallQueryEngine {
    /// 查询调用者（谁调用了此符号）
    ///
    /// # 参数
    /// - `symbol_id`: 目标符号 ID
    /// - `depth`: 递归深度（1 = 仅直接调用者）
    pub fn query_callers(
        db: &IndexDatabase,
        symbol_id: i64,
        depth: u32,
    ) -> Result<Vec<CallRelation>, CctError> {
        info!(
            symbol_id = symbol_id,
            depth = depth,
            "CallQueryEngine::query_callers 查询调用者"
        );

        let mut all_relations = Vec::new();
        let mut current_ids = vec![symbol_id];
        let mut visited = HashSet::new();
        visited.insert(symbol_id);

        for level in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                let relations = query_direct_callers(db, id)?;
                for r in &relations {
                    if visited.insert(r.caller_id) {
                        next_ids.push(r.caller_id);
                    }
                }
                all_relations.extend(relations);
            }
            debug!(level = level + 1, found = next_ids.len(), "层级查询完成");
            current_ids = next_ids;
            if current_ids.is_empty() {
                break;
            }
        }

        debug!(total = all_relations.len(), "调用者查询完成");
        Ok(all_relations)
    }

    /// 查询被调用者（此符号调用了谁）
    ///
    /// # 参数
    /// - `symbol_id`: 目标符号 ID
    /// - `depth`: 递归深度（1 = 仅直接被调用者）
    pub fn query_callees(
        db: &IndexDatabase,
        symbol_id: i64,
        depth: u32,
    ) -> Result<Vec<CallRelation>, CctError> {
        info!(
            symbol_id = symbol_id,
            depth = depth,
            "CallQueryEngine::query_callees 查询被调用者"
        );

        let mut all_relations = Vec::new();
        let mut current_ids = vec![symbol_id];
        let mut visited = HashSet::new();
        visited.insert(symbol_id);

        for level in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                let relations = query_direct_callees(db, id)?;
                for r in &relations {
                    if visited.insert(r.callee_id) {
                        next_ids.push(r.callee_id);
                    }
                }
                all_relations.extend(relations);
            }
            debug!(level = level + 1, found = next_ids.len(), "层级查询完成");
            current_ids = next_ids;
            if current_ids.is_empty() {
                break;
            }
        }

        debug!(total = all_relations.len(), "被调用者查询完成");
        Ok(all_relations)
    }
}

fn query_direct_callers(db: &IndexDatabase, callee_id: i64) -> Result<Vec<CallRelation>, CctError> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, caller_id, callee_id, file_path, line, column, is_virtual, is_indirect \
             FROM call_relations WHERE callee_id = ?1",
        )
        .map_err(|e| CctError::Database(e.to_string()))?;

    let results: Vec<CallRelation> = stmt
        .query_map(params![callee_id], |row| row_to_call_relation(row))
        .map_err(|e| CctError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CctError::Database(e.to_string()))?;

    Ok(results)
}

fn query_direct_callees(db: &IndexDatabase, caller_id: i64) -> Result<Vec<CallRelation>, CctError> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, caller_id, callee_id, file_path, line, column, is_virtual, is_indirect \
             FROM call_relations WHERE caller_id = ?1",
        )
        .map_err(|e| CctError::Database(e.to_string()))?;

    let results: Vec<CallRelation> = stmt
        .query_map(params![caller_id], |row| row_to_call_relation(row))
        .map_err(|e| CctError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CctError::Database(e.to_string()))?;

    Ok(results)
}

fn row_to_call_relation(row: &rusqlite::Row) -> Result<CallRelation, rusqlite::Error> {
    Ok(CallRelation {
        id: row.get(0)?,
        caller_id: row.get(1)?,
        callee_id: row.get(2)?,
        call_site_file: row.get(3)?,
        call_site_line: row.get(4)?,
        call_site_column: row.get(5)?,
        is_virtual_dispatch: row.get(6)?,
        is_indirect: row.get(7)?,
    })
}
