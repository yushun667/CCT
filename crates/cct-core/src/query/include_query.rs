use rusqlite::params;
use tracing::{debug, info};

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;
use crate::models::relation::IncludeRelation;

/// 头文件包含关系查询引擎
///
/// # 设计说明
/// 双向查询文件的 `#include` 关系：
/// - `query_includes`: 文件主动包含了哪些头文件
/// - `query_included_by`: 文件被哪些文件包含
pub struct IncludeQueryEngine;

impl IncludeQueryEngine {
    /// 查询指定文件包含的头文件
    ///
    /// # 参数
    /// - `file_path`: 源文件路径
    pub fn query_includes(
        db: &IndexDatabase,
        file_path: &str,
    ) -> Result<Vec<IncludeRelation>, CctError> {
        info!(
            file = %file_path,
            "IncludeQueryEngine::query_includes 查询文件包含的头文件"
        );

        let conn = db.conn();
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, source_file, target_file, line, is_system, resolved_path \
                 FROM include_relations \
                 WHERE source_file = ?1 \
                 ORDER BY line",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<IncludeRelation> = stmt
            .query_map(params![file_path], |row| row_to_include(row))
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "包含查询完成");
        Ok(results)
    }

    /// 查询哪些文件包含了指定文件
    ///
    /// # 参数
    /// - `file_path`: 目标头文件路径
    pub fn query_included_by(
        db: &IndexDatabase,
        file_path: &str,
    ) -> Result<Vec<IncludeRelation>, CctError> {
        info!(
            file = %file_path,
            "IncludeQueryEngine::query_included_by 查询被哪些文件包含"
        );

        let conn = db.conn();
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, source_file, target_file, line, is_system, resolved_path \
                 FROM include_relations \
                 WHERE target_file = ?1 \
                 ORDER BY source_file",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<IncludeRelation> = stmt
            .query_map(params![file_path], |row| row_to_include(row))
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "反向包含查询完成");
        Ok(results)
    }

    /// 查询所有包含关系（用于构建文件依赖图）
    pub fn query_all(db: &IndexDatabase) -> Result<Vec<IncludeRelation>, CctError> {
        info!("IncludeQueryEngine::query_all 查询所有包含关系");

        let conn = db.conn();
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, source_file, target_file, line, is_system, resolved_path \
                 FROM include_relations \
                 ORDER BY source_file, line",
            )
            .map_err(|e| CctError::Database(e.to_string()))?;

        let results: Vec<IncludeRelation> = stmt
            .query_map([], |row| row_to_include(row))
            .map_err(|e| CctError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CctError::Database(e.to_string()))?;

        debug!(count = results.len(), "全量包含关系查询完成");
        Ok(results)
    }
}

fn row_to_include(row: &rusqlite::Row) -> Result<IncludeRelation, rusqlite::Error> {
    Ok(IncludeRelation {
        id: row.get(0)?,
        source_file: row.get(1)?,
        target_file: row.get(2)?,
        include_line: row.get(3)?,
        is_system_header: row.get(4)?,
        resolved_path: row.get(5)?,
    })
}
