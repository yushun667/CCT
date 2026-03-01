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
///
/// 同一个函数可能在头文件和源文件中分别有声明和定义，
/// 产生不同的符号 ID。查询时通过 qualified_name 合并所有
/// 同名符号的 ID，确保不遗漏任何调用关系。
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

        let seed_ids = resolve_all_ids_for_symbol(db, symbol_id)?;
        debug!(seed_ids = ?seed_ids, "已展开同名符号 ID");

        let mut all_relations = Vec::new();
        let mut current_ids: Vec<i64> = seed_ids.iter().copied().collect();
        let mut visited: HashSet<i64> = seed_ids;

        for level in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                let relations = query_direct_callers(db, id)?;
                for r in &relations {
                    let expanded = resolve_all_ids_for_symbol(db, r.caller_id)?;
                    for &eid in &expanded {
                        if visited.insert(eid) {
                            next_ids.push(eid);
                        }
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

        dedup_relations(&mut all_relations);
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

        let seed_ids = resolve_all_ids_for_symbol(db, symbol_id)?;
        debug!(seed_ids = ?seed_ids, "已展开同名符号 ID");

        let mut all_relations = Vec::new();
        let mut current_ids: Vec<i64> = seed_ids.iter().copied().collect();
        let mut visited: HashSet<i64> = seed_ids;

        for level in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                let relations = query_direct_callees(db, id)?;
                for r in &relations {
                    let expanded = resolve_all_ids_for_symbol(db, r.callee_id)?;
                    for &eid in &expanded {
                        if visited.insert(eid) {
                            next_ids.push(eid);
                        }
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

        dedup_relations(&mut all_relations);
        debug!(total = all_relations.len(), "被调用者查询完成");
        Ok(all_relations)
    }
}

/// 给定一个符号 ID，找到数据库中所有拥有相同 qualified_name 的符号 ID 集合。
/// 这解决了同一个函数在头文件（声明）和源文件（定义）中产生不同 ID 的问题。
fn resolve_all_ids_for_symbol(db: &IndexDatabase, symbol_id: i64) -> Result<HashSet<i64>, CctError> {
    let conn = db.conn();

    let qname: Option<String> = conn
        .query_row(
            "SELECT qualified_name FROM symbols WHERE id = ?1",
            params![symbol_id],
            |row| row.get(0),
        )
        .ok();

    let mut ids = HashSet::new();
    ids.insert(symbol_id);

    if let Some(ref name) = qname {
        let mut stmt = conn
            .prepare_cached("SELECT id FROM symbols WHERE qualified_name = ?1")
            .map_err(|e| CctError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![name], |row| row.get::<_, i64>(0))
            .map_err(|e| CctError::Database(e.to_string()))?;

        for r in rows {
            if let Ok(id) = r {
                ids.insert(id);
            }
        }
    }

    Ok(ids)
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

/// 按 (caller_id, callee_id) 去重
fn dedup_relations(rels: &mut Vec<CallRelation>) {
    let mut seen = HashSet::new();
    rels.retain(|r| seen.insert((r.caller_id, r.callee_id)));
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
