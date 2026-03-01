//! M6 AI 助手引擎
//!
//! 提供 LLM 客户端、技能定义、上下文收集和对话管理功能。
//! 作为 Tauri 命令层与底层 AI 交互之间的桥梁。

pub mod context;
pub mod conversation;
pub mod llm_client;
pub mod skills;
