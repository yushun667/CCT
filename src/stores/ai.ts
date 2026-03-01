/**
 * AI 助手 Pinia Store
 *
 * 管理 AI 对话状态、消息流、配置和技能列表。
 * 通过事件监听实现流式输出效果。
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  aiChat,
  aiStop,
  listConversations,
  getConversation,
  deleteConversation as apiDeleteConversation,
  getAiConfig,
  updateAiConfig as apiUpdateAiConfig,
  listAiSkills,
  type ChatMessage,
  type ConversationSummary,
  type AiConfig,
  type AiSkill,
} from "@/api/ai";

export const useAiStore = defineStore("ai", () => {
  const conversations = ref<ConversationSummary[]>([]);
  const currentConversationId = ref<string | null>(null);
  const messages = ref<ChatMessage[]>([]);
  const isStreaming = ref(false);
  const streamingContent = ref("");
  const aiConfig = ref<AiConfig>({
    provider: null,
    model: null,
    api_key_ref: null,
    base_url: null,
    privacy_mode: "Local",
  });
  const skills = ref<AiSkill[]>([]);

  const hasConversation = computed(() => messages.value.length > 0);

  let unlistenChunk: UnlistenFn | null = null;

  async function setupChunkListener() {
    if (unlistenChunk) return;
    unlistenChunk = await listen<string>("ai-chunk", (event) => {
      streamingContent.value += event.payload;
    });
  }

  function teardownChunkListener() {
    if (unlistenChunk) {
      unlistenChunk();
      unlistenChunk = null;
    }
  }

  async function sendMessage(projectId: string, content: string) {
    if (!content.trim() || isStreaming.value) return;

    messages.value.push({ role: "user", content });
    isStreaming.value = true;
    streamingContent.value = "";

    await setupChunkListener();

    try {
      const result = await aiChat(
        projectId,
        content,
        currentConversationId.value ?? undefined,
      );

      currentConversationId.value = result.conversation_id;

      messages.value.push({ role: "assistant", content: result.response });
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : String(err);
      messages.value.push({
        role: "assistant",
        content: `**错误**: ${errorMsg}`,
      });
    } finally {
      isStreaming.value = false;
      streamingContent.value = "";
      teardownChunkListener();
    }
  }

  async function stopGeneration() {
    try {
      await aiStop();
    } finally {
      isStreaming.value = false;
      streamingContent.value = "";
      teardownChunkListener();
    }
  }

  async function loadConversations() {
    try {
      conversations.value = await listConversations();
    } catch {
      conversations.value = [];
    }
  }

  async function loadConversation(id: string) {
    try {
      const conv = await getConversation(id);
      currentConversationId.value = conv.id;
      messages.value = conv.messages;
    } catch {
      messages.value = [];
    }
  }

  function newConversation() {
    currentConversationId.value = null;
    messages.value = [];
    streamingContent.value = "";
  }

  async function deleteConversation(id: string) {
    await apiDeleteConversation(id);
    conversations.value = conversations.value.filter((c) => c.id !== id);
    if (currentConversationId.value === id) {
      newConversation();
    }
  }

  async function loadConfig() {
    try {
      aiConfig.value = await getAiConfig();
    } catch {
      // keep defaults
    }
  }

  async function updateConfig(config: AiConfig) {
    await apiUpdateAiConfig(config);
    aiConfig.value = config;
  }

  async function loadSkills() {
    try {
      skills.value = await listAiSkills();
    } catch {
      skills.value = [];
    }
  }

  return {
    conversations,
    currentConversationId,
    messages,
    isStreaming,
    streamingContent,
    aiConfig,
    skills,
    hasConversation,
    sendMessage,
    stopGeneration,
    loadConversations,
    loadConversation,
    newConversation,
    deleteConversation,
    loadConfig,
    updateConfig,
    loadSkills,
  };
});
