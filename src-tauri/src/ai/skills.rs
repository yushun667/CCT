//! AI 技能定义 — 对应 doc/07 中的 14 项内置技能
//!
//! 每项技能描述了 AI 助手可执行的代码分析任务，
//! 包含名称、描述和所需参数定义。

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

/// 执行指定技能（占位实现）
///
/// # 参数
/// - `skill_name`: 技能名称
/// - `params`: 技能参数（JSON 格式）
///
/// # 返回
/// 技能执行结果文本
pub fn execute_skill(skill_name: &str, params: &serde_json::Value) -> Result<String, cct_core::error::CctError> {
    info!(skill = %skill_name, "执行 AI 技能");
    debug!(params = %params, "技能参数");

    let skills = list_skills();
    if !skills.iter().any(|s| s.name == skill_name) {
        warn!(skill = %skill_name, "未知技能");
        return Err(cct_core::error::CctError::AiConfigInvalid(
            format!("未知技能: {}", skill_name),
        ));
    }

    let result = format!(
        "技能 `{}` 执行完成（占位响应）。\n\n\
         参数: ```json\n{}\n```\n\n\
         实际的技能执行需要连接到解析索引和 LLM 服务。",
        skill_name,
        serde_json::to_string_pretty(params).unwrap_or_default()
    );

    debug!(skill = %skill_name, result_len = result.len(), "技能执行完成");
    Ok(result)
}

fn param(name: &str, desc: &str, ptype: &str, required: bool) -> SkillParameter {
    SkillParameter {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ptype.to_string(),
        required,
    }
}
