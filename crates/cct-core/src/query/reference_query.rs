use rusqlite::params;
use tracing::{debug, info};

use crate::error::CctError;
use crate::indexer::database::{str_to_ref_kind, IndexDatabase};
use crate::models::relation::ReferenceRelation;

/// 引用关系查询引擎
///
/// # 设计说明
/// 查询指定符号在代码库中的所有引用位置，
/// 支持 Read / Write / Address / Call / Type 等引用类型区分。
pub struct ReferenceQueryEngine;

impl ReferenceQueryEngine {
    /// 查询指定符号的所有引用
    ///
    /// # 参数
    /// - `symbol_id`: 目标符号 ID
    ///
    /// # 返回
    /// 按文件路径和行号排序的引用列表
    pub fn query_references(
        db: &IndexDatabase,
        symbol_id: i64,
    ) -> Result<Vec<ReferenceRelation>, CctError> {
        info!(
            symbol_id = symbol_id,
            "ReferenceQueryEngine::query_references 查询符号引用"
        );

        let conn = db.conn();
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, symbol_id, file_path, line, column, ref_kind \
                 FROM reference_relations \
                 WHERE symbol_id = ?1 \
                 ORDER BY file_path, line",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<ReferenceRelation> = stmt
            .query_map(params![symbol_id], |row| {
                Ok(ReferenceRelation {
                    id: row.get(0)?,
                    symbol_id: row.get(1)?,
                    reference_file: row.get(2)?,
                    reference_line: row.get(3)?,
                    reference_column: row.get(4)?,
                    reference_kind: str_to_ref_kind(&row.get::<_, String>(5)?),
                })
            })
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "引用查询完成");
        Ok(results)
    }
}
