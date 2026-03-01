//! 对话管理 — 持久化存储和检索 AI 对话历史
//!
//! 每段对话保存为独立 JSON 文件，存储在 `{data_dir}/conversations/` 目录下。
//! 支持创建、保存、加载、列出和删除操作。

use cct_core::error::CctError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use super::llm_client::ChatMessage;

/// 完整对话记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 对话摘要（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// 创建新对话
    pub fn new(title: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 生成摘要信息
    pub fn summary(&self) -> ConversationSummary {
        ConversationSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            message_count: self.messages.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    /// 添加消息并更新时间戳
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
        self.updated_at = Utc::now();
    }

    /// 从首条用户消息生成标题
    pub fn auto_title(&mut self) {
        if let Some(first_user) = self.messages.iter().find(|m| m.role == "user") {
            let title: String = first_user.content.chars().take(30).collect();
            self.title = if first_user.content.len() > 30 {
                format!("{}...", title)
            } else {
                title
            };
        }
    }
}

/// 对话持久化存储
pub struct ConversationStore {
    dir: PathBuf,
}

impl ConversationStore {
    /// 创建存储实例，自动确保目录存在
    ///
    /// # 参数
    /// - `data_dir`: 应用数据根目录
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.join("conversations");
        info!(dir = %dir.display(), "初始化对话存储");
        Self { dir }
    }

    /// 从默认数据目录初始化
    pub fn from_default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cct");
        Self::new(&data_dir)
    }

    fn ensure_dir(&self) -> Result<(), CctError> {
        if !self.dir.exists() {
            std::fs::create_dir_all(&self.dir)?;
            debug!(dir = %self.dir.display(), "创建对话目录");
        }
        Ok(())
    }

    fn file_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", id))
    }

    /// 保存对话到文件
    ///
    /// # 参数
    /// - `conversation`: 要保存的对话
    pub fn save(&self, conversation: &Conversation) -> Result<(), CctError> {
        info!(id = %conversation.id, title = %conversation.title, "保存对话");
        self.ensure_dir()?;
        let path = self.file_path(&conversation.id);
        let content = serde_json::to_string_pretty(conversation)?;
        std::fs::write(&path, content)?;
        debug!(path = %path.display(), "对话已保存");
        Ok(())
    }

    /// 加载指定 ID 的对话
    ///
    /// # 参数
    /// - `id`: 对话 UUID 字符串
    pub fn load(&self, id: &str) -> Result<Conversation, CctError> {
        info!(id = %id, "加载对话");
        let path = self.file_path(id);
        if !path.exists() {
            warn!(id = %id, "对话不存在");
            return Err(CctError::Internal(format!("对话不存在: {}", id)));
        }
        let content = std::fs::read_to_string(&path)?;
        let conv: Conversation = serde_json::from_str(&content)?;
        debug!(id = %id, messages = conv.messages.len(), "对话加载完成");
        Ok(conv)
    }

    /// 列出所有对话摘要，按更新时间倒序
    pub fn list(&self) -> Result<Vec<ConversationSummary>, CctError> {
        info!("列出所有对话");
        self.ensure_dir()?;

        let mut summaries = Vec::new();
        let entries = std::fs::read_dir(&self.dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<Conversation>(&content) {
                        Ok(conv) => summaries.push(conv.summary()),
                        Err(e) => {
                            error!(path = %path.display(), error = %e, "对话文件解析失败");
                        }
                    },
                    Err(e) => {
                        error!(path = %path.display(), error = %e, "对话文件读取失败");
                    }
                }
            }
        }

        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug!(count = summaries.len(), "对话列表加载完成");
        Ok(summaries)
    }

    /// 删除指定对话
    ///
    /// # 参数
    /// - `id`: 对话 UUID 字符串
    pub fn delete(&self, id: &str) -> Result<(), CctError> {
        info!(id = %id, "删除对话");
        let path = self.file_path(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
            debug!(id = %id, "对话已删除");
        } else {
            warn!(id = %id, "要删除的对话不存在");
        }
        Ok(())
    }
}
