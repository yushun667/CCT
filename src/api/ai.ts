/**
 * AI 助手 API — 封装 Tauri invoke 调用
 *
 * 为 AI Pinia store 和组件提供类型安全的后端交互接口。
 */
import { invoke } from "@tauri-apps/api/core";

// ── 类型定义 ──────────────────────────────────────────────────────

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface ConversationSummary {
  id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export interface Conversation {
  id: string;
  title: string;
  messages: ChatMessage[];
  created_at: string;
  updated_at: string;
}

export interface AiChatResult {
  conversation_id: string;
  response: string;
}

export interface AiConfig {
  provider: string | null;
  model: string | null;
  api_key_ref: string | null;
  base_url: string | null;
  privacy_mode: "Full" | "Anonymized" | "Local";
}

export interface SkillParameter {
  name: string;
  description: string;
  param_type: string;
  required: boolean;
}

export interface AiSkill {
  name: string;
  description: string;
  parameters: SkillParameter[];
}

// ── 对话 API ──────────────────────────────────────────────────────

export async function aiChat(
  projectId: string,
  message: string,
  conversationId?: string,
): Promise<AiChatResult> {
  const raw = await invoke<string>("ai_chat", {
    projectId,
    message,
    conversationId: conversationId ?? null,
  });
  return JSON.parse(raw) as AiChatResult;
}

export async function aiStop(): Promise<void> {
  return invoke<void>("ai_stop");
}

export async function listConversations(): Promise<ConversationSummary[]> {
  return invoke<ConversationSummary[]>("list_conversations");
}

export async function getConversation(
  conversationId: string,
): Promise<Conversation> {
  const raw = await invoke<Record<string, unknown>>("get_conversation", {
    conversationId,
  });
  return raw as unknown as Conversation;
}

export async function deleteConversation(
  conversationId: string,
): Promise<void> {
  return invoke<void>("delete_conversation", { conversationId });
}

// ── 配置 API ──────────────────────────────────────────────────────

export async function getAiConfig(): Promise<AiConfig> {
  const raw = await invoke<Record<string, unknown>>("get_ai_config");
  return raw as unknown as AiConfig;
}

export async function updateAiConfig(config: AiConfig): Promise<void> {
  return invoke<void>("update_ai_config", { config });
}

// ── 技能 API ──────────────────────────────────────────────────────

export async function listAiSkills(): Promise<AiSkill[]> {
  return invoke<AiSkill[]>("list_ai_skills");
}
