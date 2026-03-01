//! AI 技能定义与执行 — 对应 doc/07 中的 14 项内置技能
//!
//! 每项技能描述了 AI 助手可执行的代码分析任务，
//! 包含名称、描述和所需参数定义。
//!
//! `execute_skill()` 连接 `IndexDatabase` 和各查询引擎，
//! 将索引数据转换为供 LLM 理解的文本格式。

use std::path::{Path, PathBuf};

use cct_core::error::CctError;
use cct_core::indexer::database::IndexDatabase;
use cct_core::query::{
    CallQueryEngine, IncludeQueryEngine, ReferenceQueryEngine, SymbolSearchEngine,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// 技能参数描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}

/// AI 技能定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub parameters: Vec<SkillParameter>,
}

/// 获取所有内置技能列表
///
/// # 返回
/// 14 项内置 AI 分析技能的完整定义
pub fn list_skills() -> Vec<Skill> {
    info!("获取 AI 技能列表");

    let skills = vec![
        Skill {
            name: "explain_function".to_string(),
            description: "解释函数的功能、参数、返回值和副作用".to_string(),
            parameters: vec![
                param("symbol_id", "函数符号 ID", "integer", true),
                param("detail_level", "详细程度: brief/normal/detailed", "string", false),
            ],
        },
        Skill {
            name: "explain_code".to_string(),
            description: "解释一段代码的逻辑和意图".to_string(),
            parameters: vec![
                param("file_path", "源文件路径", "string", true),
                param("start_line", "起始行", "integer", true),
                param("end_line", "结束行", "integer", true),
            ],
        },
        Skill {
            name: "find_callers".to_string(),
            description: "查找调用指定函数的所有位置".to_string(),
            parameters: vec![
                param("symbol_id", "函数符号 ID", "integer", true),
                param("max_depth", "最大搜索深度", "integer", false),
            ],
        },
        Skill {
            name: "find_callees".to_string(),
            description: "查找指定函数调用的所有函数".to_string(),
            parameters: vec![
                param("symbol_id", "函数符号 ID", "integer", true),
                param("max_depth", "最大搜索深度", "integer", false),
            ],
        },
        Skill {
            name: "find_references".to_string(),
            description: "查找符号的所有引用位置".to_string(),
            parameters: vec![
                param("symbol_id", "符号 ID", "integer", true),
                param("ref_kind", "引用类型: read/write/all", "string", false),
            ],
        },
        Skill {
            name: "analyze_dependencies".to_string(),
            description: "分析文件或模块的依赖关系".to_string(),
            parameters: vec![
                param("file_path", "文件路径", "string", true),
                param("include_transitive", "是否包含传递依赖", "boolean", false),
            ],
        },
        Skill {
            name: "explain_architecture".to_string(),
            description: "解释项目或模块的整体架构设计".to_string(),
            parameters: vec![
                param("module_name", "模块名称（可选，为空则分析全局）", "string", false),
            ],
        },
        Skill {
            name: "suggest_refactoring".to_string(),
            description: "针对指定代码提出重构建议".to_string(),
            parameters: vec![
                param("file_path", "源文件路径", "string", true),
                param("symbol_id", "符号 ID（可选）", "integer", false),
            ],
        },
        Skill {
            name: "find_similar_code".to_string(),
            description: "查找与指定代码片段相似的代码".to_string(),
            parameters: vec![
                param("file_path", "源文件路径", "string", true),
                param("start_line", "起始行", "integer", true),
                param("end_line", "结束行", "integer", true),
                param("threshold", "相似度阈值 0.0~1.0", "number", false),
            ],
        },
        Skill {
            name: "explain_macro".to_string(),
            description: "解释 C/C++ 宏的定义和展开结果".to_string(),
            parameters: vec![
                param("symbol_id", "宏符号 ID", "integer", true),
            ],
        },
        Skill {
            name: "trace_data_flow".to_string(),
            description: "追踪变量或数据的流向".to_string(),
            parameters: vec![
                param("symbol_id", "变量/参数符号 ID", "integer", true),
                param("direction", "方向: forward/backward/both", "string", false),
            ],
        },
        Skill {
            name: "analyze_complexity".to_string(),
            description: "分析函数或模块的复杂度指标".to_string(),
            parameters: vec![
                param("file_path", "源文件路径（可选）", "string", false),
                param("symbol_id", "函数符号 ID（可选）", "integer", false),
            ],
        },
        Skill {
            name: "compare_implementations".to_string(),
            description: "比较两个函数或类的实现差异".to_string(),
            parameters: vec![
                param("symbol_id_a", "第一个符号 ID", "integer", true),
                param("symbol_id_b", "第二个符号 ID", "integer", true),
            ],
        },
        Skill {
            name: "generate_documentation".to_string(),
            description: "为指定代码生成文档注释".to_string(),
            parameters: vec![
                param("symbol_id", "符号 ID", "integer", true),
                param("style", "文档风格: doxygen/javadoc/markdown", "string", false),
            ],
        },
    ];

    debug!(count = skills.len(), "技能列表加载完成");
    skills
}

/// 执行指定技能 — 连接索引数据库和查询引擎获取真实数据
///
/// # 参数
/// - `skill_name`: 技能名称
/// - `params`: 技能参数（JSON 格式）
/// - `project_id`: 项目 UUID，用于定位索引数据库
///
/// # 返回
/// 技能执行结果文本（供 LLM 参考的格式化信息）
pub fn execute_skill(
    skill_name: &str,
    params: &serde_json::Value,
    project_id: &str,
) -> Result<String, CctError> {
    info!(skill = %skill_name, project_id = %project_id, "执行 AI 技能");
    debug!(params = %params, "技能参数");

    let skills = list_skills();
    if !skills.iter().any(|s| s.name == skill_name) {
        warn!(skill = %skill_name, "未知技能");
        return Err(CctError::AiConfigInvalid(format!("未知技能: {}", skill_name)));
    }

    let db = open_project_db(project_id)?;

    let result = match skill_name {
        "find_callers" => execute_find_callers(&db, params)?,
        "find_callees" => execute_find_callees(&db, params)?,
        "find_references" => execute_find_references(&db, params)?,
        "explain_function" => execute_explain_function(&db, params)?,
        "explain_code" => execute_explain_code(params)?,
        "analyze_dependencies" => execute_analyze_dependencies(&db, params)?,
        "explain_macro" => execute_explain_function(&db, params)?,
        other => {
            let base_info = gather_skill_context(&db, params)?;
            format!(
                "技能 `{other}` 需要 LLM 结合以下索引数据进行分析：\n\n{base_info}\n\n\
                 请基于上述代码结构信息，执行 `{other}` 分析。\n\
                 参数: {}",
                serde_json::to_string_pretty(params).unwrap_or_default()
            )
        }
    };

    debug!(skill = %skill_name, result_len = result.len(), "技能执行完成");
    Ok(result)
}

// ── 各技能的具体执行逻辑 ──────────────────────────────────────────────

fn execute_find_callers(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_find_callers 查找调用者");
    let symbol_id = extract_i64(params, "symbol_id")?;
    let depth = params.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(3) as u32;

    let symbol_name = db.lookup_symbol_name(symbol_id)
        .unwrap_or_else(|| format!("symbol#{}", symbol_id));

    let relations = CallQueryEngine::query_callers(db, symbol_id, depth)?;

    if relations.is_empty() {
        return Ok(format!("未找到 `{}` (ID={}) 的调用者。", symbol_name, symbol_id));
    }

    let mut lines = vec![format!("## `{}` 的调用者（共 {} 处）\n", symbol_name, relations.len())];
    for r in &relations {
        let caller_name = db.lookup_symbol_name(r.caller_id)
            .unwrap_or_else(|| format!("symbol#{}", r.caller_id));
        lines.push(format!(
            "- `{}` → `{}` @ {}:{}{}",
            caller_name,
            symbol_name,
            r.call_site_file,
            r.call_site_line,
            if r.is_virtual_dispatch { " (virtual)" } else { "" },
        ));
    }

    Ok(lines.join("\n"))
}

fn execute_find_callees(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_find_callees 查找被调用者");
    let symbol_id = extract_i64(params, "symbol_id")?;
    let depth = params.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(3) as u32;

    let symbol_name = db.lookup_symbol_name(symbol_id)
        .unwrap_or_else(|| format!("symbol#{}", symbol_id));

    let relations = CallQueryEngine::query_callees(db, symbol_id, depth)?;

    if relations.is_empty() {
        return Ok(format!("未找到 `{}` (ID={}) 的被调用函数。", symbol_name, symbol_id));
    }

    let mut lines = vec![format!("## `{}` 调用的函数（共 {} 处）\n", symbol_name, relations.len())];
    for r in &relations {
        let callee_name = db.lookup_symbol_name(r.callee_id)
            .unwrap_or_else(|| format!("symbol#{}", r.callee_id));
        lines.push(format!(
            "- `{}` → `{}` @ {}:{}{}",
            symbol_name,
            callee_name,
            r.call_site_file,
            r.call_site_line,
            if r.is_indirect { " (indirect)" } else { "" },
        ));
    }

    Ok(lines.join("\n"))
}

fn execute_find_references(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_find_references 查找引用");
    let symbol_id = extract_i64(params, "symbol_id")?;

    let symbol_name = db.lookup_symbol_name(symbol_id)
        .unwrap_or_else(|| format!("symbol#{}", symbol_id));

    let refs = ReferenceQueryEngine::query_references(db, symbol_id)?;

    if refs.is_empty() {
        return Ok(format!("未找到 `{}` (ID={}) 的引用。", symbol_name, symbol_id));
    }

    let mut lines = vec![format!("## `{}` 的引用位置（共 {} 处）\n", symbol_name, refs.len())];
    for r in &refs {
        lines.push(format!(
            "- [{}] {}:{}:{}",
            r.reference_kind, r.reference_file, r.reference_line, r.reference_column
        ));
    }

    Ok(lines.join("\n"))
}

fn execute_explain_function(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_explain_function 解释函数");
    let symbol_id = extract_i64(params, "symbol_id")?;

    let symbol = db.lookup_symbol(symbol_id).ok_or_else(|| {
        CctError::SymbolNotFound(format!("符号 ID={} 未找到", symbol_id))
    })?;

    let mut lines = vec![
        format!("## 符号详情: `{}`\n", symbol.qualified_name),
        format!("- **类型**: {}", symbol.kind),
        format!("- **名称**: {}", symbol.name),
        format!("- **限定名**: {}", symbol.qualified_name),
        format!("- **文件**: {}:{}", symbol.file_path, symbol.line),
    ];

    if let Some(ref rt) = symbol.return_type {
        lines.push(format!("- **返回类型**: {}", rt));
    }
    if let Some(ref p) = symbol.parameters {
        lines.push(format!("- **参数**: {}", p));
    }
    if let Some(ref access) = symbol.access {
        lines.push(format!("- **访问级别**: {:?}", access));
    }
    if let Some(end) = symbol.end_line {
        lines.push(format!("- **范围**: 行 {} ~ {}", symbol.line, end));
    }
    if symbol.is_definition {
        lines.push("- **定义**: 是".to_string());
    }

    Ok(lines.join("\n"))
}

fn execute_explain_code(params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_explain_code 解释代码片段");
    let file_path = extract_str(params, "file_path")?;
    let start_line = extract_i64(params, "start_line")? as usize;
    let end_line = extract_i64(params, "end_line")? as usize;

    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        CctError::ParseFileRead(format!("读取 {} 失败: {}", file_path, e))
    })?;

    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let start = start_line.saturating_sub(1).min(total);
    let end = end_line.min(total);

    if start >= end {
        return Ok(format!("指定行范围无效: {}~{} (文件共 {} 行)", start_line, end_line, total));
    }

    let snippet: Vec<String> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>5} | {}", start + i + 1, line))
        .collect();

    Ok(format!(
        "## 代码片段: `{}` 行 {}~{}\n\n```\n{}\n```",
        file_path, start_line, end_line,
        snippet.join("\n")
    ))
}

fn execute_analyze_dependencies(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    info!("execute_analyze_dependencies 分析依赖");
    let file_path = extract_str(params, "file_path")?;

    let includes = IncludeQueryEngine::query_includes(db, &file_path)?;
    let included_by = IncludeQueryEngine::query_included_by(db, &file_path)?;

    let mut lines = vec![format!("## 文件依赖分析: `{}`\n", file_path)];

    lines.push(format!("### 包含的头文件（共 {} 个）\n", includes.len()));
    for inc in &includes {
        let kind = if inc.is_system_header { "系统" } else { "项目" };
        lines.push(format!(
            "- [{}] `{}` (行 {})",
            kind, inc.target_file, inc.include_line
        ));
    }

    lines.push(format!("\n### 被以下文件包含（共 {} 个）\n", included_by.len()));
    for inc in &included_by {
        lines.push(format!(
            "- `{}` (行 {})",
            inc.source_file, inc.include_line
        ));
    }

    Ok(lines.join("\n"))
}

/// 为不直接支持查询的技能收集通用上下文信息
fn gather_skill_context(db: &IndexDatabase, params: &serde_json::Value) -> Result<String, CctError> {
    debug!("gather_skill_context 收集通用技能上下文");
    let mut parts = Vec::new();

    if let Some(sid) = params.get("symbol_id").and_then(|v| v.as_i64()) {
        if let Some(sym) = db.lookup_symbol(sid) {
            parts.push(format!(
                "**目标符号**: `{}` ({}) @ {}:{}",
                sym.qualified_name, sym.kind, sym.file_path, sym.line
            ));
        }
    }

    if let Some(fp) = params.get("file_path").and_then(|v| v.as_str()) {
        let symbols = SymbolSearchEngine::search_by_file(db, fp).unwrap_or_default();
        parts.push(format!("**文件 `{}`** 包含 {} 个符号", fp, symbols.len()));
        for s in symbols.iter().take(20) {
            parts.push(format!("  - {} `{}` 行 {}", s.kind, s.qualified_name, s.line));
        }
        if symbols.len() > 20 {
            parts.push(format!("  - ... 还有 {} 个符号", symbols.len() - 20));
        }
    }

    if parts.is_empty() {
        let stats = db.get_statistics()?;
        parts.push(format!(
            "**项目统计**: {} 个符号, {} 个函数, {} 个文件",
            stats.total_symbols, stats.total_functions, stats.total_files
        ));
    }

    Ok(parts.join("\n"))
}

// ── 辅助函数 ──────────────────────────────────────────────────────────

/// 打开项目对应的索引数据库
fn open_project_db(project_id: &str) -> Result<IndexDatabase, CctError> {
    let db_path = index_db_path(project_id);
    debug!(path = %db_path.display(), "打开项目索引数据库");

    if !db_path.exists() {
        return Err(CctError::Database(format!(
            "项目索引数据库不存在: {}",
            db_path.display()
        )));
    }

    IndexDatabase::open(&db_path)
}

/// 计算项目索引数据库的路径
pub fn index_db_path(project_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join("cct")
        .join("index")
        .join(format!("{}.db", project_id))
}

fn extract_i64(params: &serde_json::Value, key: &str) -> Result<i64, CctError> {
    params
        .get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| CctError::AiConfigInvalid(format!("缺少必需参数: {}", key)))
}

fn extract_str(params: &serde_json::Value, key: &str) -> Result<String, CctError> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CctError::AiConfigInvalid(format!("缺少必需参数: {}", key)))
}

fn param(name: &str, desc: &str, ptype: &str, required: bool) -> SkillParameter {
    SkillParameter {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ptype.to_string(),
        required,
    }
}
