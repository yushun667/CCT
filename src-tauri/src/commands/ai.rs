//! AI 助手 Tauri 命令 — 对应 M6 模块
//!
//! 提供对话交互、配置管理和技能查询等前端 IPC 接口。
//! 流式输出通过 Tauri 事件系统推送 `ai-chunk` 事件。

use cct_core::config::{AiConfig, AppConfig};
use cct_core::error::CctError;
use tracing::{debug, info, warn};

use crate::ai::context;
use crate::ai::conversation::{Conversation, ConversationStore};
use crate::ai::llm_client::LlmClient;
use crate::ai::skills;

fn load_app_config() -> AppConfig {
    let config_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cct")
        .join("config.json");
    AppConfig::load(&config_path)
}

/// 发送 AI 聊天消息
///
/// # 参数
/// - `app`: Tauri 应用句柄，用于发送事件
/// - `project_id`: 当前项目 ID
/// - `message`: 用户消息内容
/// - `conversation_id`: 对话 ID（可选，为空则创建新对话）
///
/// # 返回
/// 对话 ID（JSON 字符串），前端可用于后续消息
///
/// # 事件
/// 流式输出通过 `ai-chunk` 事件推送，payload 为文本片段
#[tauri::command]
pub fn ai_chat(
    app: tauri::AppHandle,
    project_id: String,
    message: String,
    conversation_id: Option<String>,
) -> Result<String, CctError> {
    info!(
        project_id = %project_id,
        conversation_id = ?conversation_id,
        message_len = message.len(),
        "Tauri Command: ai_chat"
    );

    let config = load_app_config();
    let store = ConversationStore::from_default();

    let mut conversation = match &conversation_id {
        Some(id) => {
            debug!(id = %id, "加载已有对话");
            store.load(id)?
        }
        None => {
            let conv = Conversation::new("新对话");
            debug!(id = %conv.id, "创建新对话");
            conv
        }
    };

    let _ctx = context::collect_context(&project_id, None, None);

    conversation.add_message("user", &message);

    let client = LlmClient::new(&config.ai)?;

    let app_handle = app.clone();
    let response = client.chat(&conversation.messages, |chunk| {
        use tauri::Emitter;
        let _ = app_handle.emit("ai-chunk", chunk);
    })?;

    conversation.add_message("assistant", &response);

    if conversation.title == "新对话" {
        conversation.auto_title();
    }

    store.save(&conversation)?;

    let result = serde_json::json!({
        "conversation_id": conversation.id,
        "response": response,
    });

    debug!(conversation_id = %conversation.id, "AI 对话完成");
    Ok(serde_json::to_string(&result)?)
}

/// 停止 AI 生成（占位实现）
#[tauri::command]
pub fn ai_stop() -> Result<(), CctError> {
    info!("Tauri Command: ai_stop");
    warn!("ai_stop 当前为占位实现");
    Ok(())
}

/// 列出所有对话
///
/// # 返回
/// 对话摘要列表的 JSON 数组
#[tauri::command]
pub fn list_conversations() -> Result<Vec<serde_json::Value>, CctError> {
    info!("Tauri Command: list_conversations");
    let store = ConversationStore::from_default();
    let summaries = store.list()?;
    let result: Vec<serde_json::Value> = summaries
        .iter()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();
    debug!(count = result.len(), "对话列表返回");
    Ok(result)
}

/// 获取单个对话详情
///
/// # 参数
/// - `conversation_id`: 对话 UUID
#[tauri::command]
pub fn get_conversation(conversation_id: String) -> Result<serde_json::Value, CctError> {
    info!(id = %conversation_id, "Tauri Command: get_conversation");
    let store = ConversationStore::from_default();
    let conv = store.load(&conversation_id)?;
    let result = serde_json::to_value(&conv)?;
    debug!(id = %conversation_id, "对话详情返回");
    Ok(result)
}

/// 删除对话
///
/// # 参数
/// - `conversation_id`: 要删除的对话 UUID
#[tauri::command]
pub fn delete_conversation(conversation_id: String) -> Result<(), CctError> {
    info!(id = %conversation_id, "Tauri Command: delete_conversation");
    let store = ConversationStore::from_default();
    store.delete(&conversation_id)?;
    debug!(id = %conversation_id, "对话已删除");
    Ok(())
}

/// 获取当前 AI 配置
#[tauri::command]
pub fn get_ai_config() -> Result<serde_json::Value, CctError> {
    info!("Tauri Command: get_ai_config");
    let config = load_app_config();
    let result = serde_json::to_value(&config.ai)?;
    debug!("AI 配置返回");
    Ok(result)
}

/// 更新 AI 配置
///
/// # 参数
/// - `config`: 新的 AI 配置（JSON 对象）
#[tauri::command]
pub fn update_ai_config(config: serde_json::Value) -> Result<(), CctError> {
    info!("Tauri Command: update_ai_config");
    let ai_config: AiConfig = serde_json::from_value(config)?;

    let config_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cct")
        .join("config.json");

    let mut app_config = AppConfig::load(&config_path);
    app_config.ai = ai_config;
    app_config.save(&config_path)?;
    debug!("AI 配置已更新");
    Ok(())
}

/// 获取所有 AI 技能列表
#[tauri::command]
pub fn list_ai_skills() -> Result<Vec<serde_json::Value>, CctError> {
    info!("Tauri Command: list_ai_skills");
    let skill_list = skills::list_skills();
    let result: Vec<serde_json::Value> = skill_list
        .iter()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();
    debug!(count = result.len(), "技能列表返回");
    Ok(result)
}
