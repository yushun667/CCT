use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use tracing::{debug, error, info, warn};

/// 单个文件解析的超时时间
const PER_FILE_TIMEOUT: Duration = Duration::from_secs(30);

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;
use crate::indexer::file_scanner;
use crate::models::graph::ParseStatistics;
use crate::models::project::ParseProgress;
use crate::models::relation::{FileInfo, FileParseStatus};
use crate::models::symbol::SymbolKind;

use super::clang_bridge::ClangBridgeParser;
use super::{ParseResult, Parser};

/// C/C++ 源文件默认扩展名
const SOURCE_EXTENSIONS: &[&str] = &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx"];

/// 解析调度器 — 管理多线程并行解析 C/C++ 源文件
///
/// # 设计说明
/// 采用 rayon 工作窃取线程池分发解析任务，通过原子计数器
/// 和回调函数实时汇报进度，支持大规模代码库的高效解析。
pub struct ParseScheduler {
    /// 线程池最大线程数
    thread_count: usize,
}

impl ParseScheduler {
    /// 创建解析调度器
    ///
    /// # 参数
    /// - `max_threads`: 最大线程数；`None` 时使用 CPU 核心数
    pub fn new(max_threads: Option<u32>) -> Self {
        let thread_count = max_threads
            .map(|n| n as usize)
            .unwrap_or_else(|| rayon::current_num_threads());

        info!(
            thread_count,
            "ParseScheduler::new 创建解析调度器"
        );

        Self { thread_count }
    }

    /// 执行全量/增量解析
    ///
    /// 扫描源码目录中的 C/C++ 文件，使用 rayon 线程池并行解析，
    /// 通过回调函数实时上报进度。解析完成后将结果写入索引数据库。
    ///
    /// # 参数
    /// - `source_root`: 源码根目录
    /// - `compile_db`: 可选的编译数据库路径
    /// - `db_path`: 索引数据库路径；提供后将解析结果持久化到 SQLite
    /// - `excluded_dirs`: 用户自定义的排除目录名列表
    /// - `progress_callback`: 进度回调，每完成一个文件调用一次
    ///
    /// # 返回
    /// 解析统计信息，包含文件数、符号数、关系数和耗时。
    pub fn schedule_parse<F>(
        &self,
        source_root: &Path,
        compile_db: Option<&Path>,
        db_path: Option<&Path>,
        excluded_dirs: &[String],
        progress_callback: F,
    ) -> Result<ParseStatistics, CctError>
    where
        F: Fn(ParseProgress) + Send + Sync,
    {
        info!(
            root = %source_root.display(),
            compile_db = ?compile_db.map(|p| p.display().to_string()),
            "ParseScheduler::schedule_parse 开始解析任务"
        );

        if !source_root.exists() || !source_root.is_dir() {
            return Err(CctError::InvalidSourceRoot(
                source_root.display().to_string(),
            ));
        }

        let files = file_scanner::scan_source_files(source_root, SOURCE_EXTENSIONS, excluded_dirs);
        if files.is_empty() {
            return Err(CctError::ParseNoSource);
        }

        let total_files = files.len() as u64;
        info!(total_files, "扫描到待解析文件");

        let parser = ClangBridgeParser::new(compile_db)?;

        // 当无 compile_commands.json 时，自动收集项目目录树中的
        // 所有目录作为 -I include 路径，确保 Clang 能解析跨文件的
        // #include 指令，进而正确识别被调用函数的声明。
        let include_args: Vec<String> = if compile_db.is_none() {
            let scan_start = Instant::now();
            let mut args = Vec::new();
            args.push(format!("-I{}", source_root.display()));

            for entry in walkdir::WalkDir::new(source_root)
                .max_depth(10)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_dir() {
                    let name = entry.file_name().to_string_lossy();
                    if name.starts_with('.') {
                        continue;
                    }
                    let name_str = name.to_string();
                    if excluded_dirs.contains(&name_str) {
                        continue;
                    }
                    args.push(format!("-I{}", entry.path().display()));
                }
            }

            info!(
                count = args.len(),
                elapsed_ms = scan_start.elapsed().as_millis(),
                "已自动收集 include 路径（无编译数据库模式）"
            );
            args
        } else {
            Vec::new()
        };

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.thread_count)
            .stack_size(64 * 1024 * 1024) // 64 MB — Clang 递归 AST 求值需要大栈
            .build()
            .map_err(|e| CctError::Internal(format!("线程池创建失败: {e}")))?;

        let start = Instant::now();
        let parsed_count = AtomicU64::new(0);
        let failed_count = AtomicU64::new(0);
        let symbol_count = AtomicU64::new(0);
        let relation_count = AtomicU64::new(0);

        let stats_fn_count = AtomicU64::new(0);
        let stats_var_count = AtomicU64::new(0);
        let stats_type_count = AtomicU64::new(0);
        let stats_macro_count = AtomicU64::new(0);
        let stats_call_count = AtomicU64::new(0);
        let stats_include_count = AtomicU64::new(0);
        let stats_ref_count = AtomicU64::new(0);
        let stats_inherit_count = AtomicU64::new(0);

        let callback = &progress_callback;
        let results: Vec<(PathBuf, Result<ParseResult, CctError>)> = pool.install(|| {
            files
                .par_iter()
                .map(|file_path| {
                    debug!(file = %file_path.display(), "开始解析文件");
                    let file_start = Instant::now();
                    let result = parser.parse_file(file_path, &include_args);
                    let file_elapsed = file_start.elapsed();
                    if file_elapsed > PER_FILE_TIMEOUT {
                        warn!(
                            file = %file_path.display(),
                            elapsed_ms = file_elapsed.as_millis(),
                            "文件解析超时（超过 {}s）",
                            PER_FILE_TIMEOUT.as_secs()
                        );
                    }

                    match &result {
                        Ok(pr) => {
                            let file_symbols = pr.symbols.len() as u64;
                            let file_relations = pr.call_relations.len() as u64
                                + pr.include_relations.len() as u64
                                + pr.reference_relations.len() as u64
                                + pr.inheritance_relations.len() as u64;

                            symbol_count.fetch_add(file_symbols, Ordering::Relaxed);
                            relation_count.fetch_add(file_relations, Ordering::Relaxed);

                            for sym in &pr.symbols {
                                match sym.kind {
                                    SymbolKind::Function => {
                                        stats_fn_count.fetch_add(1, Ordering::Relaxed);
                                    }
                                    SymbolKind::Variable => {
                                        stats_var_count.fetch_add(1, Ordering::Relaxed);
                                    }
                                    SymbolKind::Type => {
                                        stats_type_count.fetch_add(1, Ordering::Relaxed);
                                    }
                                    SymbolKind::Macro => {
                                        stats_macro_count.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            stats_call_count.fetch_add(
                                pr.call_relations.len() as u64,
                                Ordering::Relaxed,
                            );
                            stats_include_count.fetch_add(
                                pr.include_relations.len() as u64,
                                Ordering::Relaxed,
                            );
                            stats_ref_count.fetch_add(
                                pr.reference_relations.len() as u64,
                                Ordering::Relaxed,
                            );
                            stats_inherit_count.fetch_add(
                                pr.inheritance_relations.len() as u64,
                                Ordering::Relaxed,
                            );

                            let done = parsed_count.fetch_add(1, Ordering::Relaxed) + 1;
                            let elapsed = start.elapsed().as_secs_f64();
                            let rate = done as f64 / elapsed.max(0.001);
                            let remaining = (total_files - done) as f64 / rate.max(0.001);

                            let pct = if total_files > 0 {
                                ((done as f32 / total_files as f32) * 100.0).clamp(0.0, 100.0)
                            } else { 0.0 };
                            callback(ParseProgress {
                                phase: "parsing".to_string(),
                                total_files,
                                parsed_files: done,
                                failed_files: failed_count.load(Ordering::Relaxed),
                                percentage: pct,
                                current_file: file_path.display().to_string(),
                                symbols_found: symbol_count.load(Ordering::Relaxed),
                                relations_found: relation_count.load(Ordering::Relaxed),
                                elapsed_seconds: elapsed,
                                estimated_remaining: remaining,
                            });
                        }
                        Err(e) => {
                            error!(
                                file = %file_path.display(),
                                error = %e,
                                "文件解析失败"
                            );
                            failed_count.fetch_add(1, Ordering::Relaxed);
                            let done = parsed_count.fetch_add(1, Ordering::Relaxed) + 1;
                            let elapsed = start.elapsed().as_secs_f64();
                            let pct = if total_files > 0 {
                                ((done as f32 / total_files as f32) * 100.0).clamp(0.0, 100.0)
                            } else { 0.0 };

                            callback(ParseProgress {
                                phase: "parsing".to_string(),
                                total_files,
                                parsed_files: done,
                                failed_files: failed_count.load(Ordering::Relaxed),
                                percentage: pct,
                                current_file: file_path.display().to_string(),
                                symbols_found: symbol_count.load(Ordering::Relaxed),
                                relations_found: relation_count.load(Ordering::Relaxed),
                                elapsed_seconds: elapsed,
                                estimated_remaining: 0.0,
                            });
                        }
                    }

                    (file_path.clone(), result)
                })
                .collect()
        });

        let elapsed = start.elapsed().as_secs_f64();
        let parsed = parsed_count.load(Ordering::Relaxed);
        let failed = failed_count.load(Ordering::Relaxed);

        info!(
            total_files,
            parsed,
            failed,
            elapsed_secs = format!("{elapsed:.2}"),
            "解析任务完成"
        );

        // ── 跨文件调用关系解析 ────────────────────────────────────
        // 构建全局 qualified_name → symbol_id 映射表，
        // 将单文件阶段无法解析的调用关系（被调用者在其他文件）补全。
        let global_resolve_start = Instant::now();
        let mut global_name_to_id: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();

        // 记录每个 qualified_name 对应符号是否为定义，
        // 定义优先：避免声明和定义产生不同 ID 导致调用关系查不到
        for (_path, result) in &results {
            if let Ok(pr) = result {
                for sym in &pr.symbols {
                    match global_name_to_id.entry(sym.qualified_name.clone()) {
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(sym.id);
                        }
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            if sym.is_definition {
                                e.insert(sym.id);
                            }
                        }
                    }
                }
            }
        }

        let mut resolved_cross_file_calls: Vec<crate::models::relation::CallRelation> =
            Vec::new();
        let mut unresolved_count = 0u64;

        for (_path, result) in &results {
            if let Ok(pr) = result {
                for uc in &pr.unresolved_calls {
                    let caller_id = global_name_to_id.get(&uc.caller_name).copied();
                    let callee_id = global_name_to_id.get(&uc.callee_name).copied();

                    if let (Some(crid), Some(ceid)) = (caller_id, callee_id) {
                        use std::sync::atomic::AtomicI64;
                        static CROSS_ID: AtomicI64 = AtomicI64::new(1_000_000_000);
                        resolved_cross_file_calls.push(
                            crate::models::relation::CallRelation {
                                id: CROSS_ID.fetch_add(1, Ordering::Relaxed),
                                caller_id: crid,
                                callee_id: ceid,
                                call_site_file: uc.file_path.clone(),
                                call_site_line: uc.line,
                                call_site_column: uc.column,
                                is_virtual_dispatch: uc.is_virtual,
                                is_indirect: uc.is_indirect,
                            },
                        );
                    } else {
                        unresolved_count += 1;
                    }
                }
            }
        }

        stats_call_count.fetch_add(resolved_cross_file_calls.len() as u64, Ordering::Relaxed);
        relation_count.fetch_add(resolved_cross_file_calls.len() as u64, Ordering::Relaxed);

        info!(
            resolved = resolved_cross_file_calls.len(),
            still_unresolved = unresolved_count,
            elapsed_ms = global_resolve_start.elapsed().as_millis(),
            "跨文件调用关系解析完成"
        );

        if let Some(db_p) = db_path {
            info!(db = %db_p.display(), "开始将解析结果写入索引数据库");

            progress_callback(ParseProgress {
                phase: "indexing".to_string(),
                total_files,
                parsed_files: parsed,
                failed_files: failed,
                percentage: 0.0,
                current_file: "正在写入索引数据库...".to_string(),
                symbols_found: symbol_count.load(Ordering::Relaxed),
                relations_found: relation_count.load(Ordering::Relaxed),
                elapsed_seconds: elapsed,
                estimated_remaining: 0.0,
            });

            if let Some(parent) = db_p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match IndexDatabase::open(db_p) {
                Ok(mut db) => {
                    if let Err(e) = db.initialize() {
                        error!(error = %e, "初始化数据库表结构失败");
                    } else {
                        let write_start = Instant::now();
                        let mut total_written_symbols = 0u64;
                        let mut total_written_relations = 0u64;
                        let total_results = results.len() as u64;
                        let mut written_count = 0u64;

                        for (file_path, result) in &results {
                            let path_str = file_path.display().to_string();
                            match result {
                                Ok(pr) => {
                                    let file_start = Instant::now();

                                    if !pr.symbols.is_empty() {
                                        if let Err(e) = db.insert_symbols(&pr.symbols) {
                                            warn!(file = %path_str, error = %e, "写入符号失败");
                                        } else {
                                            total_written_symbols += pr.symbols.len() as u64;
                                        }
                                    }
                                    if !pr.call_relations.is_empty() {
                                        if let Err(e) = db.insert_call_relations(&pr.call_relations) {
                                            warn!(file = %path_str, error = %e, "写入调用关系失败");
                                        } else {
                                            total_written_relations += pr.call_relations.len() as u64;
                                        }
                                    }
                                    if !pr.include_relations.is_empty() {
                                        if let Err(e) = db.insert_include_relations(&pr.include_relations) {
                                            warn!(file = %path_str, error = %e, "写入包含关系失败");
                                        } else {
                                            total_written_relations += pr.include_relations.len() as u64;
                                        }
                                    }
                                    if !pr.reference_relations.is_empty() {
                                        if let Err(e) = db.insert_reference_relations(&pr.reference_relations) {
                                            warn!(file = %path_str, error = %e, "写入引用关系失败");
                                        } else {
                                            total_written_relations += pr.reference_relations.len() as u64;
                                        }
                                    }
                                    if !pr.inheritance_relations.is_empty() {
                                        if let Err(e) = db.insert_inheritance_relations(&pr.inheritance_relations) {
                                            warn!(file = %path_str, error = %e, "写入继承关系失败");
                                        } else {
                                            total_written_relations += pr.inheritance_relations.len() as u64;
                                        }
                                    }

                                    let hash = file_scanner::compute_file_hash(file_path)
                                        .unwrap_or_default();
                                    let fi = FileInfo {
                                        file_path: path_str.clone(),
                                        last_modified: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs() as i64,
                                        content_hash: hash,
                                        parse_status: FileParseStatus::Success,
                                        error_message: None,
                                        symbol_count: pr.symbols.len() as u32,
                                        parse_time_ms: Some(file_start.elapsed().as_millis() as u32),
                                    };
                                    if let Err(e) = db.upsert_file_info(&fi) {
                                        warn!(file = %path_str, error = %e, "更新文件信息失败");
                                    }

                                    written_count += 1;
                                    if written_count % 50 == 0 || written_count == total_results {
                                        let idx_pct = if total_results > 0 {
                                            ((written_count as f32 / total_results as f32) * 100.0).clamp(0.0, 100.0)
                                        } else { 0.0 };
                                        progress_callback(ParseProgress {
                                            phase: "indexing".to_string(),
                                            total_files: total_results,
                                            parsed_files: written_count,
                                            failed_files: 0,
                                            percentage: idx_pct,
                                            current_file: path_str.clone(),
                                            symbols_found: total_written_symbols,
                                            relations_found: total_written_relations,
                                            elapsed_seconds: write_start.elapsed().as_secs_f64(),
                                            estimated_remaining: 0.0,
                                        });
                                    }
                                }
                                Err(e) => {
                                    let fi = FileInfo {
                                        file_path: path_str.clone(),
                                        last_modified: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs() as i64,
                                        content_hash: String::new(),
                                        parse_status: FileParseStatus::Failed,
                                        error_message: Some(e.to_string()),
                                        symbol_count: 0,
                                        parse_time_ms: None,
                                    };
                                    let _ = db.upsert_file_info(&fi);
                                }
                            }
                        }

                        // 写入跨文件调用关系
                        if !resolved_cross_file_calls.is_empty() {
                            if let Err(e) = db.insert_call_relations(&resolved_cross_file_calls) {
                                warn!(error = %e, "写入跨文件调用关系失败");
                            } else {
                                total_written_relations += resolved_cross_file_calls.len() as u64;
                                info!(
                                    count = resolved_cross_file_calls.len(),
                                    "跨文件调用关系已写入数据库"
                                );
                            }
                        }

                        info!(
                            symbols = total_written_symbols,
                            relations = total_written_relations,
                            write_secs = format!("{:.2}", write_start.elapsed().as_secs_f64()),
                            "索引数据库写入完成"
                        );
                    }
                }
                Err(e) => {
                    error!(error = %e, db = %db_p.display(), "打开索引数据库失败，解析结果未持久化");
                }
            }
        }

        Ok(ParseStatistics {
            total_files,
            parsed_files: parsed - failed,
            failed_files: failed,
            total_symbols: symbol_count.load(Ordering::Relaxed),
            total_functions: stats_fn_count.load(Ordering::Relaxed),
            total_variables: stats_var_count.load(Ordering::Relaxed),
            total_types: stats_type_count.load(Ordering::Relaxed),
            total_macros: stats_macro_count.load(Ordering::Relaxed),
            total_call_relations: stats_call_count.load(Ordering::Relaxed),
            total_include_relations: stats_include_count.load(Ordering::Relaxed),
            total_reference_relations: stats_ref_count.load(Ordering::Relaxed),
            total_inheritance_relations: stats_inherit_count.load(Ordering::Relaxed),
            elapsed_seconds: elapsed,
        })
    }
}
