//! 上下文收集器 — 为 AI 对话构建代码上下文
//!
//! 从项目解析索引中提取当前文件、符号及其关联信息，
//! 组装成结构化上下文供 LLM 理解代码语境。

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

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

/// 收集指定项目和位置的上下文信息
///
/// # 参数
/// - `project_id`: 项目 UUID 字符串
/// - `file_path`: 当前打开的文件路径（可选）
/// - `symbol_id`: 当前选中的符号 ID（可选）
///
/// # 返回
/// 包含项目、文件、符号等信息的上下文结构体。
/// 当前为占位实现，后续将连接解析索引。
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

    let context = ContextInfo {
        project_name: format!("project-{}", &project_id[..8.min(project_id.len())]),
        current_file: file_path.map(|s| s.to_string()),
        current_symbol: symbol_id.map(|id| format!("symbol_{}", id)),
        related_symbols: Vec::new(),
        relevant_code: Vec::new(),
    };

    debug!(
        project = %context.project_name,
        has_file = context.current_file.is_some(),
        has_symbol = context.current_symbol.is_some(),
        "上下文收集完成"
    );

    context
}
