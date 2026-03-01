use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::CctError;
use crate::indexer::database::IndexDatabase;

/// 自定义规则 — YAML 可序列化
///
/// # 字段
/// - `name`: 规则名称（唯一标识）
/// - `description`: 规则描述
/// - `pattern`: 匹配模式
/// - `severity`: 严重程度
/// - `action`: 触发动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub name: String,
    pub description: String,
    pub pattern: RulePattern,
    pub severity: Severity,
    #[serde(default = "default_action")]
    pub action: Action,
}

fn default_action() -> Action {
    Action::Report
}

/// 规则匹配模式
///
/// # 字段
/// - `symbol_kind`: 目标符号类型（可选，如 "function" / "variable"）
/// - `name_regex`: 符号名称正则表达式
/// - `file_pattern`: 文件路径通配符（可选）
/// - `has_attribute`: 要求存在的属性关键词（可选）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePattern {
    pub symbol_kind: Option<String>,
    pub name_regex: String,
    pub file_pattern: Option<String>,
    pub has_attribute: Option<String>,
}

/// 规则严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// 规则触发动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Report,
    Highlight,
}

/// 规则匹配结果
///
/// # 字段
/// - `rule_name`: 触发的规则名称
/// - `symbol_name`: 匹配到的符号名
/// - `file_path`: 符号所在文件
/// - `line`: 符号所在行
/// - `message`: 匹配提示信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    pub rule_name: String,
    pub symbol_name: String,
    pub file_path: String,
    pub line: u32,
    pub message: String,
}

/// 自定义规则引擎 — 加载、校验、执行基于 YAML 的分析规则
///
/// # 设计说明（策略模式 + 模板方法）
/// 每条规则定义一个匹配策略（RulePattern），引擎负责遍历
/// 索引数据库中的符号并逐条应用规则。
pub struct RuleEngine;

impl RuleEngine {
    /// 从 YAML 文件加载规则列表
    ///
    /// # 参数
    /// - `yaml_path`: YAML 文件路径
    ///
    /// # 返回
    /// 解析后的规则列表
    ///
    /// # 错误
    /// - 文件不存在或无法读取
    /// - YAML 格式不正确
    pub fn load_rules(yaml_path: &Path) -> Result<Vec<CustomRule>, CctError> {
        info!(
            path = %yaml_path.display(),
            "RuleEngine::load_rules 加载自定义规则"
        );

        let content = std::fs::read_to_string(yaml_path).map_err(|e| {
            warn!(path = %yaml_path.display(), error = %e, "读取规则文件失败");
            CctError::Io(e)
        })?;

        let rules: Vec<CustomRule> = serde_yaml::from_str(&content).map_err(|e| {
            warn!(error = %e, "YAML 解析失败");
            CctError::Internal(format!("规则文件格式错误: {e}"))
        })?;

        for rule in &rules {
            Self::validate_rule(rule)?;
        }

        debug!(count = rules.len(), "规则加载完成");
        Ok(rules)
    }

    /// 校验单条规则的语法正确性
    ///
    /// # 参数
    /// - `rule`: 待校验的规则
    ///
    /// # 错误
    /// - 名称为空
    /// - 正则表达式无效
    /// - symbol_kind 不在支持范围内
    pub fn validate_rule(rule: &CustomRule) -> Result<(), CctError> {
        debug!(rule = %rule.name, "RuleEngine::validate_rule 校验规则");

        if rule.name.is_empty() {
            return Err(CctError::Internal("规则名称不能为空".to_string()));
        }

        if rule.pattern.name_regex.is_empty() {
            return Err(CctError::Internal(format!(
                "规则 '{}' 的 name_regex 不能为空",
                rule.name
            )));
        }

        Regex::new(&rule.pattern.name_regex).map_err(|e| {
            CctError::Internal(format!(
                "规则 '{}' 的正则表达式无效: {e}",
                rule.name
            ))
        })?;

        if let Some(ref kind) = rule.pattern.symbol_kind {
            match kind.as_str() {
                "function" | "variable" | "type" | "macro" => {}
                _ => {
                    return Err(CctError::Internal(format!(
                        "规则 '{}' 的 symbol_kind '{}' 不受支持",
                        rule.name, kind
                    )));
                }
            }
        }

        if let Some(ref fp) = rule.pattern.file_pattern {
            Regex::new(fp).map_err(|e| {
                CctError::Internal(format!(
                    "规则 '{}' 的 file_pattern 正则无效: {e}",
                    rule.name
                ))
            })?;
        }

        debug!(rule = %rule.name, "规则校验通过");
        Ok(())
    }

    /// 对索引数据库中的符号执行规则匹配
    ///
    /// # 参数
    /// - `db`: 索引数据库引用
    /// - `rules`: 要执行的规则列表
    ///
    /// # 返回
    /// 所有匹配结果
    pub fn apply_rules(
        db: &IndexDatabase,
        rules: &[CustomRule],
    ) -> Result<Vec<RuleMatch>, CctError> {
        info!(
            rule_count = rules.len(),
            "RuleEngine::apply_rules 执行规则匹配"
        );

        let conn = db.conn();
        let mut all_matches = Vec::new();

        for rule in rules {
            debug!(rule = %rule.name, "执行规则");

            let name_re = Regex::new(&rule.pattern.name_regex).map_err(|e| {
                CctError::Internal(format!("正则编译失败: {e}"))
            })?;

            let file_re = rule
                .pattern
                .file_pattern
                .as_ref()
                .map(|fp| Regex::new(fp))
                .transpose()
                .map_err(|e| CctError::Internal(format!("文件模式正则编译失败: {e}")))?;

            let sql = if let Some(ref kind) = rule.pattern.symbol_kind {
                format!(
                    "SELECT name, qualified_name, file_path, line, attributes
                     FROM symbols
                     WHERE kind = '{}' AND is_definition = 1",
                    kind
                )
            } else {
                "SELECT name, qualified_name, file_path, line, attributes
                 FROM symbols
                 WHERE is_definition = 1"
                    .to_string()
            };

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| CctError::Database(e.to_string()))?;

            let matches: Vec<RuleMatch> = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let qualified: String = row.get(1)?;
                    let file_path: String = row.get(2)?;
                    let line: u32 = row.get(3)?;
                    let attributes: Option<String> = row.get(4)?;
                    Ok((name, qualified, file_path, line, attributes))
                })
                .map_err(|e| CctError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .filter(|(name, qualified, file_path, _line, attributes)| {
                    if !name_re.is_match(name) && !name_re.is_match(qualified) {
                        return false;
                    }

                    if let Some(ref fre) = file_re {
                        if !fre.is_match(file_path) {
                            return false;
                        }
                    }

                    if let Some(ref attr_key) = rule.pattern.has_attribute {
                        if let Some(ref attrs) = attributes {
                            if !attrs.contains(attr_key.as_str()) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }

                    true
                })
                .map(|(name, _qualified, file_path, line, _attrs)| RuleMatch {
                    rule_name: rule.name.clone(),
                    symbol_name: name,
                    file_path,
                    line,
                    message: rule.description.clone(),
                })
                .collect();

            debug!(
                rule = %rule.name,
                match_count = matches.len(),
                "规则执行完成"
            );
            all_matches.extend(matches);
        }

        info!(
            total_matches = all_matches.len(),
            "所有规则执行完成"
        );
        Ok(all_matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rule_ok() {
        let rule = CustomRule {
            name: "test_rule".to_string(),
            description: "Test".to_string(),
            pattern: RulePattern {
                symbol_kind: Some("function".to_string()),
                name_regex: r"^test_.*".to_string(),
                file_pattern: None,
                has_attribute: None,
            },
            severity: Severity::Warning,
            action: Action::Report,
        };
        assert!(RuleEngine::validate_rule(&rule).is_ok());
    }

    #[test]
    fn test_validate_rule_empty_name() {
        let rule = CustomRule {
            name: "".to_string(),
            description: "Test".to_string(),
            pattern: RulePattern {
                symbol_kind: None,
                name_regex: ".*".to_string(),
                file_pattern: None,
                has_attribute: None,
            },
            severity: Severity::Info,
            action: Action::Report,
        };
        assert!(RuleEngine::validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_rule_bad_regex() {
        let rule = CustomRule {
            name: "bad".to_string(),
            description: "Test".to_string(),
            pattern: RulePattern {
                symbol_kind: None,
                name_regex: "[invalid".to_string(),
                file_pattern: None,
                has_attribute: None,
            },
            severity: Severity::Error,
            action: Action::Highlight,
        };
        assert!(RuleEngine::validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_rule_bad_kind() {
        let rule = CustomRule {
            name: "bad_kind".to_string(),
            description: "Test".to_string(),
            pattern: RulePattern {
                symbol_kind: Some("unknown".to_string()),
                name_regex: ".*".to_string(),
                file_pattern: None,
                has_attribute: None,
            },
            severity: Severity::Info,
            action: Action::Report,
        };
        assert!(RuleEngine::validate_rule(&rule).is_err());
    }
}
