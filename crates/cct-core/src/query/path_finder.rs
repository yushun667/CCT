use std::collections::{HashMap, VecDeque};

use rusqlite::params;
use tracing::{debug, info};

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;

/// 调用路径查找器 — BFS 最短路径
///
/// # 设计说明
/// 在函数调用图上使用 BFS 查找两个符号之间的最短调用路径。
/// 返回路径上的符号 ID 序列，用于可视化调用链。
pub struct PathFinder;

impl PathFinder {
    /// 查找两个符号之间的最短调用路径
    ///
    /// # 参数
    /// - `from_id`: 起始符号 ID
    /// - `to_id`: 目标符号 ID
    ///
    /// # 返回
    /// 符号 ID 序列（含首尾），无路径时返回空 Vec
    pub fn find_shortest_path(
        db: &IndexDatabase,
        from_id: i64,
        to_id: i64,
    ) -> Result<Vec<i64>, CctError> {
        info!(
            from_id = from_id,
            to_id = to_id,
            "PathFinder::find_shortest_path 查找最短调用路径"
        );

        if from_id == to_id {
            debug!("起止相同，返回单节点路径");
            return Ok(vec![from_id]);
        }

        let mut queue = VecDeque::new();
        let mut predecessors: HashMap<i64, Option<i64>> = HashMap::new();

        queue.push_back(from_id);
        predecessors.insert(from_id, None);

        while let Some(current) = queue.pop_front() {
            if current == to_id {
                let path = reconstruct_path(&predecessors, to_id);
                debug!(path_len = path.len(), "找到最短路径");
                return Ok(path);
            }

            let callee_ids = get_callee_ids(db, current)?;
            for callee_id in callee_ids {
                if !predecessors.contains_key(&callee_id) {
                    predecessors.insert(callee_id, Some(current));
                    queue.push_back(callee_id);
                }
            }
        }

        debug!("未找到路径");
        Ok(vec![])
    }
}

fn get_callee_ids(db: &IndexDatabase, caller_id: i64) -> Result<Vec<i64>, CctError> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached("SELECT DISTINCT callee_id FROM call_relations WHERE caller_id = ?1")
        .map_err(|e| CctError::Database(e.to_string()))?;

    let ids: Vec<i64> = stmt
        .query_map(params![caller_id], |row| row.get(0))
        .map_err(|e| CctError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CctError::Database(e.to_string()))?;

    Ok(ids)
}

fn reconstruct_path(predecessors: &HashMap<i64, Option<i64>>, to_id: i64) -> Vec<i64> {
    let mut path = vec![to_id];
    let mut node = to_id;
    while let Some(Some(prev)) = predecessors.get(&node) {
        path.push(*prev);
        node = *prev;
    }
    path.reverse();
    path
}
