//! LLM 客户端 — 封装与大语言模型的交互
//!
//! 支持 OpenAI、Anthropic、Ollama 和自定义提供商。
//! 通过流式 HTTP 请求实现实时 token 推送。

use std::sync::atomic::{AtomicBool, Ordering};

use cct_core::config::AiConfig;
use cct_core::error::CctError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// 全局停止标志，由 `ai_stop` 命令设置
pub static STOP_FLAG: AtomicBool = AtomicBool::new(false);

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

    /// 发送聊天请求，流式接收响应
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

        STOP_FLAG.store(false, Ordering::SeqCst);

        let result = tokio::runtime::Handle::current().block_on(async {
            match self.provider {
                LlmProvider::OpenAI | LlmProvider::Custom => {
                    self.chat_openai_compatible(messages, &on_chunk).await
                }
                LlmProvider::Ollama => self.chat_ollama(messages, &on_chunk).await,
                LlmProvider::Anthropic => self.chat_anthropic(messages, &on_chunk).await,
            }
        });

        match &result {
            Ok(text) => debug!(response_len = text.len(), "LLM chat 响应完成"),
            Err(e) => error!(error = %e, "LLM chat 请求失败"),
        }
        result
    }

    /// OpenAI / Custom 兼容接口的流式请求
    ///
    /// 使用 SSE (Server-Sent Events) 解析 `data: {...}` 行，
    /// 从 `choices[0].delta.content` 中提取增量文本。
    async fn chat_openai_compatible(
        &self,
        messages: &[ChatMessage],
        on_chunk: &impl Fn(&str),
    ) -> Result<String, CctError> {
        info!(provider = ?self.provider, "开始 OpenAI 兼容 API 流式请求");

        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com");
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let client = reqwest::Client::new();
        let mut req = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| {
            error!(error = %e, url = %url, "OpenAI HTTP 请求发送失败");
            CctError::AiConnectionFailed(format!("请求失败: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            error!(status = %status, body = %body_text, "OpenAI API 返回错误");
            return Err(CctError::AiConnectionFailed(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            if STOP_FLAG.load(Ordering::SeqCst) {
                info!("用户请求停止生成");
                break;
            }

            let chunk = chunk_result.map_err(|e| {
                CctError::AiConnectionFailed(format!("流式读取错误: {}", e))
            })?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        debug!("收到 [DONE] 信号");
                        break;
                    }
                    if let Ok(json) =
                        serde_json::from_str::<serde_json::Value>(data)
                    {
                        if let Some(content) = json
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|v| v.as_str())
                        {
                            on_chunk(content);
                            full_response.push_str(content);
                        }
                    }
                }
            }
        }

        Ok(full_response)
    }

    /// Ollama 本地模型的流式请求
    ///
    /// Ollama 返回逐行 JSON，每行包含 `{"message":{"content":"..."},"done":false}`。
    async fn chat_ollama(
        &self,
        messages: &[ChatMessage],
        on_chunk: &impl Fn(&str),
    ) -> Result<String, CctError> {
        info!("开始 Ollama 流式请求");

        let base = self
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");
        let url = format!("{}/api/chat", base.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, url = %url, "Ollama HTTP 请求发送失败");
                CctError::AiConnectionFailed(format!(
                    "Ollama 连接失败 ({}): {}",
                    url, e
                ))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            error!(status = %status, body = %body_text, "Ollama API 返回错误");
            return Err(CctError::AiConnectionFailed(format!(
                "Ollama HTTP {}: {}",
                status, body_text
            )));
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            if STOP_FLAG.load(Ordering::SeqCst) {
                info!("用户请求停止生成");
                break;
            }

            let chunk = chunk_result.map_err(|e| {
                CctError::AiConnectionFailed(format!("Ollama 流式读取错误: {}", e))
            })?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line)
                {
                    if let Some(content) = json
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|v| v.as_str())
                    {
                        if !content.is_empty() {
                            on_chunk(content);
                            full_response.push_str(content);
                        }
                    }

                    if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
                        debug!("Ollama 流式响应完成");
                        break;
                    }
                }
            }
        }

        Ok(full_response)
    }

    /// Anthropic Claude API 的流式请求
    ///
    /// 使用 SSE 解析 `event: content_block_delta` 后的 `data` 行，
    /// 从 `delta.text` 中提取增量文本。
    async fn chat_anthropic(
        &self,
        messages: &[ChatMessage],
        on_chunk: &impl Fn(&str),
    ) -> Result<String, CctError> {
        info!("开始 Anthropic 流式请求");

        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));

        let api_key = self.api_key.as_deref().ok_or_else(|| {
            CctError::AiConfigInvalid("Anthropic 需要 API Key".to_string())
        })?;

        // Anthropic 要求 system 消息单独传递，不在 messages 数组中
        let system_text: String = messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let non_system: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": non_system,
            "max_tokens": 4096,
            "stream": true,
        });
        if !system_text.is_empty() {
            body.as_object_mut()
                .unwrap()
                .insert("system".to_string(), serde_json::Value::String(system_text));
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, url = %url, "Anthropic HTTP 请求发送失败");
                CctError::AiConnectionFailed(format!("Anthropic 请求失败: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            error!(status = %status, body = %body_text, "Anthropic API 返回错误");
            return Err(CctError::AiConnectionFailed(format!(
                "Anthropic HTTP {}: {}",
                status, body_text
            )));
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut current_event = String::new();

        while let Some(chunk_result) = stream.next().await {
            if STOP_FLAG.load(Ordering::SeqCst) {
                info!("用户请求停止生成");
                break;
            }

            let chunk = chunk_result.map_err(|e| {
                CctError::AiConnectionFailed(format!(
                    "Anthropic 流式读取错误: {}",
                    e
                ))
            })?;

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(event_type) = line.strip_prefix("event: ") {
                    current_event = event_type.trim().to_string();
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if current_event == "content_block_delta" {
                        if let Ok(json) =
                            serde_json::from_str::<serde_json::Value>(data)
                        {
                            if let Some(text) = json
                                .get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|v| v.as_str())
                            {
                                on_chunk(text);
                                full_response.push_str(text);
                            }
                        }
                    } else if current_event == "error" {
                        warn!(data = %data, "Anthropic 返回错误事件");
                    }
                }
            }
        }

        Ok(full_response)
    }
}
