//! LLM 客户端 — 封装与大语言模型的交互
//!
//! 支持 OpenAI、Anthropic、Ollama 和自定义提供商。
//! 当前版本为占位实现，返回模拟响应。

use cct_core::config::AiConfig;
use cct_core::error::CctError;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// LLM 提供商枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Ollama,
    Custom,
}

impl LlmProvider {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => Self::OpenAI,
            "anthropic" | "claude" => Self::Anthropic,
            "ollama" => Self::Ollama,
            _ => Self::Custom,
        }
    }
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// LLM 客户端，持有连接配置
pub struct LlmClient {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl LlmClient {
    /// 根据 AiConfig 创建客户端实例
    ///
    /// # 参数
    /// - `config`: 全局 AI 配置引用
    ///
    /// # 返回
    /// 验证通过时返回 `Ok(LlmClient)`，配置无效时返回错误
    pub fn new(config: &AiConfig) -> Result<Self, CctError> {
        info!("初始化 LLM 客户端");

        let provider = config
            .provider
            .as_deref()
            .map(LlmProvider::from_str_loose)
            .unwrap_or(LlmProvider::Ollama);

        let model = config
            .model
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let api_key = config.api_key_ref.clone();
        let base_url = config.base_url.clone();

        if matches!(provider, LlmProvider::OpenAI | LlmProvider::Anthropic) && api_key.is_none() {
            warn!(
                provider = ?provider,
                "云端 LLM 提供商需要 API Key，当前未配置"
            );
        }

        debug!(
            provider = ?provider,
            model = %model,
            has_api_key = api_key.is_some(),
            "LLM 客户端配置完成"
        );

        Ok(Self {
            provider,
            model,
            api_key,
            base_url,
        })
    }

    /// 发送聊天请求（占位实现）
    ///
    /// # 参数
    /// - `messages`: 对话消息历史
    /// - `on_chunk`: 流式回调函数，每次收到增量文本时调用
    ///
    /// # 返回
    /// 完整的助手回复文本
    pub fn chat(
        &self,
        messages: &[ChatMessage],
        on_chunk: impl Fn(&str),
    ) -> Result<String, CctError> {
        info!(
            provider = ?self.provider,
            model = %self.model,
            message_count = messages.len(),
            "LLM chat 请求"
        );

        let user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let response = format!(
            "**[AI 助手 · {} / {}]**\n\n\
             收到您的问题：\n\n> {}\n\n\
             ---\n\n\
             这是一个占位响应。AI 引擎尚未连接到实际的 LLM 服务。\n\
             请在设置中配置 AI 提供商和 API Key 后使用。\n\n\
             当前支持的提供商：\n\
             - **OpenAI** (GPT-4, GPT-3.5-turbo)\n\
             - **Anthropic** (Claude 系列)\n\
             - **Ollama** (本地部署模型)\n\
             - **自定义** (兼容 OpenAI 接口的服务)",
            match self.provider {
                LlmProvider::OpenAI => "OpenAI",
                LlmProvider::Anthropic => "Anthropic",
                LlmProvider::Ollama => "Ollama",
                LlmProvider::Custom => "Custom",
            },
            self.model,
            if user_msg.len() > 200 {
                format!("{}...", &user_msg[..200])
            } else {
                user_msg.to_string()
            }
        );

        let chunks: Vec<&str> = response.split(' ').collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let text = if i > 0 {
                format!(" {}", chunk)
            } else {
                chunk.to_string()
            };
            on_chunk(&text);
        }

        debug!(response_len = response.len(), "LLM chat 响应完成");
        Ok(response)
    }
}
