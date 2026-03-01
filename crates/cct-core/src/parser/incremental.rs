//! 增量解析引擎
//!
//! 通过比较文件哈希值检测变更文件，并分析头文件 include 关系
//! 确定受影响的文件范围，仅对发生变化的文件执行重新解析，
//! 大幅提升大规模代码库的二次解析效率。
//!
//! # 设计说明（策略模式）
//! `IncrementalParser` 可作为全量解析的替代策略使用，
//! 调度器根据用户选择切换全量/增量解析策略。

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace, warn};

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;
use crate::indexer::file_scanner;
use super::Parser;
use crate::models::graph::ParseStatistics;
use crate::models::project::ParseProgress;

/// 文件变更类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// 新增文件
    Added,
    /// 已有文件内容被修改
    Modified,
    /// 文件已被删除
    Deleted,
}

/// 变更文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    /// 文件路径
    pub path: PathBuf,
    /// 变更类型
    pub change_type: ChangeType,
}

/// 增量解析器
///
/// 通过比对文件哈希与索引数据库中的记录来检测变更，
/// 并利用 include 关系图传播影响范围，最终只重新解析
/// 真正需要更新的文件集合。
pub struct IncrementalParser;

impl IncrementalParser {
    /// 检测发生变更的文件
    ///
    /// 扫描源码目录中的所有文件，与索引数据库中存储的 content_hash 比对，
    /// 找出新增、修改和已删除的文件。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `source_root`: 源码根目录
    /// - `extensions`: 待扫描的文件扩展名列表
    ///
    /// # 返回
    /// 变更文件列表
    pub fn detect_changed_files(
        db: &IndexDatabase,
        source_root: &Path,
        extensions: &[&str],
    ) -> Result<Vec<ChangedFile>, CctError> {
        info!(
            root = %source_root.display(),
            extensions = ?extensions,
            "IncrementalParser::detect_changed_files 检测变更文件"
        );

        let disk_files = file_scanner::scan_source_files(source_root, extensions);
        let mut changed = Vec::new();
        let mut seen_paths: HashSet<String> = HashSet::new();

        for file_path in &disk_files {
            let path_str = file_path.display().to_string();
            seen_paths.insert(path_str.clone());

            let current_hash = file_scanner::compute_file_hash(file_path)?;

            match db.get_file_info(&path_str)? {
                Some(info) => {
                    if info.content_hash != current_hash {
                        debug!(file = %path_str, "文件内容已修改");
                        changed.push(ChangedFile {
                            path: file_path.clone(),
                            change_type: ChangeType::Modified,
                        });
                    } else {
                        trace!(file = %path_str, "文件未变更");
                    }
                }
                None => {
                    debug!(file = %path_str, "发现新增文件");
                    changed.push(ChangedFile {
                        path: file_path.clone(),
                        change_type: ChangeType::Added,
                    });
                }
            }
        }

        // 检测已删除的文件：在数据库中存在但磁盘上不存在
        let db_files = Self::list_indexed_files(db)?;
        for db_path in db_files {
            if !seen_paths.contains(&db_path) {
                debug!(file = %db_path, "文件已被删除");
                changed.push(ChangedFile {
                    path: PathBuf::from(&db_path),
                    change_type: ChangeType::Deleted,
                });
            }
        }

        info!(
            total_on_disk = disk_files.len(),
            changed = changed.len(),
            "变更检测完成"
        );

        Ok(changed)
    }

    /// 分析变更文件的影响范围
    ///
    /// 当头文件发生变更时，所有直接或间接 include 该头文件的源文件
    /// 都可能受到影响，需要重新解析。使用 BFS 沿 include 关系图传播。
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `changed_files`: 已知的变更文件列表
    ///
    /// # 返回
    /// 受影响的文件路径列表（包含直接变更文件和间接受影响文件）
    pub fn analyze_impact(
        db: &IndexDatabase,
        changed_files: &[ChangedFile],
    ) -> Result<Vec<String>, CctError> {
        info!(
            changed_count = changed_files.len(),
            "IncrementalParser::analyze_impact 分析影响范围"
        );

        let mut affected: HashSet<String> = HashSet::new();

        // 所有变更文件自身都受影响
        for cf in changed_files {
            affected.insert(cf.path.display().to_string());
        }

        // 收集变更的头文件，用于 BFS 传播
        let header_exts = ["h", "hh", "hpp", "hxx"];
        let changed_headers: Vec<String> = changed_files
            .iter()
            .filter(|cf| {
                cf.path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| header_exts.contains(&e))
                    .unwrap_or(false)
            })
            .map(|cf| cf.path.display().to_string())
            .collect();

        if changed_headers.is_empty() {
            debug!("无头文件变更，跳过影响传播");
            return Ok(affected.into_iter().collect());
        }

        // BFS: 查找所有直接或间接 include 了变更头文件的源文件
        let mut queue: VecDeque<String> = changed_headers.into_iter().collect();

        while let Some(header) = queue.pop_front() {
            let includers = Self::find_includers(db, &header)?;
            for includer in includers {
                if affected.insert(includer.clone()) {
                    trace!(file = %includer, header = %header, "发现受影响文件");
                    queue.push_back(includer);
                }
            }
        }

        info!(affected_count = affected.len(), "影响分析完成");
        Ok(affected.into_iter().collect())
    }

    /// 执行增量解析
    ///
    /// 仅对变更文件执行重新解析，先清除旧索引数据再写入新结果。
    /// 通过回调函数实时上报解析进度。
    ///
    /// # 参数
    /// - `db`: 索引数据库（可变引用，用于清除和写入数据）
    /// - `source_root`: 源码根目录
    /// - `changed_files`: 变更文件列表
    /// - `callback`: 进度回调函数
    ///
    /// # 返回
    /// 本次增量解析的统计信息
    pub fn run_incremental<F>(
        db: &mut IndexDatabase,
        _source_root: &Path,
        changed_files: &[ChangedFile],
        callback: F,
    ) -> Result<ParseStatistics, CctError>
    where
        F: Fn(ParseProgress),
    {
        info!(
            file_count = changed_files.len(),
            "IncrementalParser::run_incremental 开始增量解析"
        );

        let start = Instant::now();
        let total = changed_files.len() as u64;
        let mut parsed: u64 = 0;
        let mut failed: u64 = 0;

        for cf in changed_files {
            let path_str = cf.path.display().to_string();

            match cf.change_type {
                ChangeType::Deleted => {
                    debug!(file = %path_str, "清除已删除文件的索引数据");
                    if let Err(e) = db.clear_file_data(&path_str) {
                        warn!(file = %path_str, error = %e, "清除索引数据失败");
                        failed += 1;
                    }
                }
                ChangeType::Added | ChangeType::Modified => {
                    debug!(file = %path_str, change = ?cf.change_type, "重新解析文件");

                    if cf.change_type == ChangeType::Modified {
                        if let Err(e) = db.clear_file_data(&path_str) {
                            warn!(file = %path_str, error = %e, "清除旧索引数据失败");
                        }
                    }

                    let parser = match super::clang_bridge::ClangBridgeParser::new(None) {
                        Ok(p) => p,
                        Err(e) => {
                            warn!(error = %e, "创建解析器失败");
                            failed += 1;
                            parsed += 1;
                            continue;
                        }
                    };

                    match parser.parse_file(&cf.path, &[]) {
                        Ok(pr) => {
                            let sym_count = pr.symbols.len() as u32;
                            let parse_start = Instant::now();

                            if !pr.symbols.is_empty() {
                                if let Err(e) = db.insert_symbols(&pr.symbols) {
                                    warn!(error = %e, "写入符号失败");
                                }
                            }
                            if !pr.call_relations.is_empty() {
                                if let Err(e) = db.insert_call_relations(&pr.call_relations) {
                                    warn!(error = %e, "写入调用关系失败");
                                }
                            }
                            if !pr.include_relations.is_empty() {
                                if let Err(e) = db.insert_include_relations(&pr.include_relations) {
                                    warn!(error = %e, "写入包含关系失败");
                                }
                            }
                            if !pr.reference_relations.is_empty() {
                                if let Err(e) = db.insert_reference_relations(&pr.reference_relations) {
                                    warn!(error = %e, "写入引用关系失败");
                                }
                            }
                            if !pr.inheritance_relations.is_empty() {
                                if let Err(e) = db.insert_inheritance_relations(&pr.inheritance_relations) {
                                    warn!(error = %e, "写入继承关系失败");
                                }
                            }

                            let hash = file_scanner::compute_file_hash(&cf.path).unwrap_or_default();
                            let fi = crate::models::relation::FileInfo {
                                file_path: path_str.clone(),
                                last_modified: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64,
                                content_hash: hash,
                                parse_status: crate::models::relation::FileParseStatus::Success,
                                error_message: None,
                                symbol_count: sym_count,
                                parse_time_ms: Some(parse_start.elapsed().as_millis() as u32),
                            };
                            if let Err(e) = db.upsert_file_info(&fi) {
                                warn!(error = %e, "更新文件信息失败");
                            }

                            debug!(file = %path_str, symbols = sym_count, "文件解析写入完成");
                        }
                        Err(e) => {
                            warn!(file = %path_str, error = %e, "文件解析失败");
                            failed += 1;
                        }
                    }
                }
            }

            parsed += 1;
            let elapsed = start.elapsed().as_secs_f64();
            let rate = parsed as f64 / elapsed.max(0.001);
            let remaining = (total - parsed) as f64 / rate.max(0.001);

            callback(ParseProgress {
                total_files: total,
                parsed_files: parsed,
                failed_files: failed,
                percentage: (parsed as f32 / total as f32) * 100.0,
                current_file: path_str,
                symbols_found: 0,
                relations_found: 0,
                elapsed_seconds: elapsed,
                estimated_remaining: remaining,
            });
        }

        let elapsed = start.elapsed().as_secs_f64();

        info!(
            total,
            parsed,
            failed,
            elapsed_secs = format!("{elapsed:.2}"),
            "增量解析完成"
        );

        Ok(ParseStatistics {
            total_files: total,
            parsed_files: parsed - failed,
            failed_files: failed,
            elapsed_seconds: elapsed,
            ..Default::default()
        })
    }

    /// 从索引数据库中获取所有已索引文件路径
    fn list_indexed_files(db: &IndexDatabase) -> Result<Vec<String>, CctError> {
        debug!("IncrementalParser::list_indexed_files 列出已索引文件");

        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT file_path FROM file_info")
            .map_err(|e| CctError::Database(format!("查询已索引文件失败: {e}")))?;

        let paths = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| CctError::Database(format!("读取文件路径失败: {e}")))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        debug!(count = paths.len(), "已索引文件列表获取完成");
        Ok(paths)
    }

    /// 查找所有 include 了指定头文件的源文件
    fn find_includers(db: &IndexDatabase, target_file: &str) -> Result<Vec<String>, CctError> {
        debug!(
            target = target_file,
            "IncrementalParser::find_includers 查找包含者"
        );

        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT source_file FROM include_relations \
                 WHERE target_file = ?1 OR resolved_path = ?1",
            )
            .map_err(|e| CctError::Database(format!("查询 include 关系失败: {e}")))?;

        let includers = stmt
            .query_map(rusqlite::params![target_file], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| CctError::Database(format!("读取 include 关系失败: {e}")))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        debug!(
            target = target_file,
            count = includers.len(),
            "包含者查询完成"
        );
        Ok(includers)
    }
}
