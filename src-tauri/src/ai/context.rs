//! 上下文收集器 — 为 AI 对话构建代码上下文
//!
//! 从项目解析索引中提取当前文件、符号及其关联信息，
//! 组装成结构化上下文供 LLM 理解代码语境。

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use cct_core::indexer::database::IndexDatabase;
use cct_core::query::{CallQueryEngine, SymbolSearchEngine};

use super::skills::index_db_path;

/// AI 对话上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInfo {
    pub project_name: String,
    pub current_file: Option<String>,
    pub current_symbol: Option<String>,
    pub related_symbols: Vec<String>,
    pub relevant_code: Vec<CodeSnippet>,
}

/// 代码片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub description: String,
}

impl ContextInfo {
    /// 将上下文信息格式化为 LLM 系统提示文本
    pub fn to_system_prompt(&self) -> String {
        debug!("ContextInfo::to_system_prompt 生成系统提示");
        let mut parts = vec![format!(
            "你是 CCT（C/C++ 代码分析工具）的 AI 助手。当前项目: {}",
            self.project_name
        )];

        if let Some(ref file) = self.current_file {
            parts.push(format!("用户正在查看文件: {}", file));
        }
        if let Some(ref sym) = self.current_symbol {
            parts.push(format!("当前关注的符号: {}", sym));
        }
        if !self.related_symbols.is_empty() {
            let syms = self.related_symbols.join(", ");
            parts.push(format!("相关符号: {}", syms));
        }
        if !self.relevant_code.is_empty() {
            parts.push("相关代码片段:".to_string());
            for snippet in &self.relevant_code {
                parts.push(format!(
                    "\n--- {} (行 {}~{}) ---\n{}\n{}",
                    snippet.file_path, snippet.start_line, snippet.end_line,
                    snippet.content, snippet.description
                ));
            }
        }

        parts.join("\n")
    }
}

/// 收集指定项目和位置的上下文信息
///
/// 尝试连接索引数据库，按文件或符号 ID 查询真实数据。
/// 任何数据库错误均优雅降级为空上下文，不影响对话流程。
///
/// # 参数
/// - `project_id`: 项目 UUID 字符串
/// - `file_path`: 当前打开的文件路径（可选）
/// - `symbol_id`: 当前选中的符号 ID（可选）
///
/// # 返回
/// 包含项目、文件、符号等信息的上下文结构体
pub fn collect_context(
    project_id: &str,
    file_path: Option<&str>,
    symbol_id: Option<i64>,
) -> ContextInfo {
    info!(
        project_id = %project_id,
        file_path = ?file_path,
        symbol_id = ?symbol_id,
        "收集 AI 对话上下文"
    );

    let project_name = format!("project-{}", &project_id[..8.min(project_id.len())]);

    let db_path = index_db_path(project_id);
    let db = match IndexDatabase::open(&db_path) {
        Ok(db) => Some(db),
        Err(e) => {
            warn!(error = %e, "无法打开索引数据库，将返回空上下文");
            None
        }
    };

    let mut current_file = file_path.map(|s| s.to_string());
    let mut current_symbol: Option<String> = None;
    let mut related_symbols = Vec::new();
    let mut relevant_code = Vec::new();

    if let Some(ref db) = db {
        if let Some(fp) = file_path {
            match SymbolSearchEngine::search_by_file(db, fp) {
                Ok(symbols) => {
                    for s in symbols.iter().take(15) {
                        related_symbols.push(format!(
                            "{} `{}` 行{}",
                            s.kind, s.qualified_name, s.line
                        ));
                    }
                    debug!(count = symbols.len(), file = %fp, "文件符号查询完成");
                }
                Err(e) => {
                    warn!(error = %e, file = %fp, "查询文件符号失败");
                }
            }

            if let Ok(content) = std::fs::read_to_string(fp) {
                let line_count = content.lines().count() as u32;
                let preview_end = line_count.min(50);
                let preview: String = content.lines().take(preview_end as usize).collect::<Vec<_>>().join("\n");
                relevant_code.push(CodeSnippet {
                    file_path: fp.to_string(),
                    start_line: 1,
                    end_line: preview_end,
                    content: preview,
                    description: format!("文件前 {} 行预览", preview_end),
                });
            }
        }

        if let Some(sid) = symbol_id {
            if let Some(sym) = db.lookup_symbol(sid) {
                current_symbol = Some(sym.qualified_name.clone());
                current_file = current_file.or(Some(sym.file_path.clone()));

                if let Ok(callers) = CallQueryEngine::query_callers(db, sid, 1) {
                    for r in callers.iter().take(10) {
                        if let Some(name) = db.lookup_symbol_name(r.caller_id) {
                            related_symbols.push(format!("调用者 `{}`", name));
                        }
                    }
                }
                if let Ok(callees) = CallQueryEngine::query_callees(db, sid, 1) {
                    for r in callees.iter().take(10) {
                        if let Some(name) = db.lookup_symbol_name(r.callee_id) {
                            related_symbols.push(format!("被调用 `{}`", name));
                        }
                    }
                }
            } else {
                warn!(symbol_id = sid, "符号 ID 未找到");
                current_symbol = Some(format!("symbol_{}", sid));
            }
        }
    }

    let context = ContextInfo {
        project_name,
        current_file,
        current_symbol,
        related_symbols,
        relevant_code,
    };

    debug!(
        project = %context.project_name,
        has_file = context.current_file.is_some(),
        has_symbol = context.current_symbol.is_some(),
        related = context.related_symbols.len(),
        snippets = context.relevant_code.len(),
        "上下文收集完成"
    );

    context
}
