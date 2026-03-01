<script setup lang="ts">
import { ref, nextTick, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useAiStore } from "@/stores/ai";
import ChatMessage from "./ChatMessage.vue";
import AiConfigPanel from "./AiConfigPanel.vue";

const { t } = useI18n();
const aiStore = useAiStore();

const inputText = ref("");
const messageListRef = ref<HTMLElement | null>(null);
const showHistory = ref(false);
const showConfig = ref(false);

const currentProjectId = ref("default");

function scrollToBottom() {
  nextTick(() => {
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight;
    }
  });
}

watch(() => aiStore.messages.length, scrollToBottom);
watch(() => aiStore.streamingContent, scrollToBottom);

async function handleSend() {
  const text = inputText.value.trim();
  if (!text || aiStore.isStreaming) return;

  inputText.value = "";
  await aiStore.sendMessage(currentProjectId.value, text);

  if (showHistory.value) {
    await aiStore.loadConversations();
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    handleSend();
  }
}

function handleNewChat() {
  aiStore.newConversation();
  inputText.value = "";
}

async function handleToggleHistory() {
  showHistory.value = !showHistory.value;
  if (showHistory.value) {
    await aiStore.loadConversations();
  }
}

async function handleSelectConversation(id: string) {
  await aiStore.loadConversation(id);
  showHistory.value = false;
}

async function handleDeleteConversation(id: string) {
  await aiStore.deleteConversation(id);
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMin = Math.floor(diffMs / 60000);

  if (diffMin < 1) return "刚刚";
  if (diffMin < 60) return `${diffMin}分钟前`;

  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour}小时前`;

  const diffDay = Math.floor(diffHour / 24);
  if (diffDay < 7) return `${diffDay}天前`;

  return d.toLocaleDateString();
}

onMounted(() => {
  aiStore.loadConfig();
});
</script>

<template>
  <div class="ai-panel">
    <!-- 配置面板 -->
    <AiConfigPanel v-if="showConfig" @close="showConfig = false" />

    <!-- 历史面板 -->
    <div v-else-if="showHistory" class="ai-panel__history">
      <div class="ai-panel__history-header">
        <span class="ai-panel__history-title">{{ t("ai.history") }}</span>
        <a-button type="text" size="small" @click="showHistory = false">
          <template #icon><icon-close /></template>
        </a-button>
      </div>
      <div class="ai-panel__history-list">
        <div
          v-for="conv in aiStore.conversations"
          :key="conv.id"
          class="ai-panel__history-item"
          @click="handleSelectConversation(conv.id)"
        >
          <div class="ai-panel__history-item-title">{{ conv.title }}</div>
          <div class="ai-panel__history-item-meta">
            <span>{{ conv.message_count }} {{ t("ai.messages") }}</span>
            <span>{{ formatTime(conv.updated_at) }}</span>
          </div>
          <a-button
            type="text"
            size="mini"
            status="danger"
            class="ai-panel__history-item-delete"
            @click.stop="handleDeleteConversation(conv.id)"
          >
            <template #icon><icon-delete /></template>
          </a-button>
        </div>
        <a-empty v-if="aiStore.conversations.length === 0" :description="t('ai.noHistory')" />
      </div>
    </div>

    <!-- 主对话面板 -->
    <template v-else>
      <div class="ai-panel__header">
        <span class="ai-panel__title">{{ t("ai.title") }}</span>
        <a-space :size="2">
          <a-tooltip :content="t('ai.newChat')">
            <a-button type="text" size="small" @click="handleNewChat">
              <template #icon><icon-plus /></template>
            </a-button>
          </a-tooltip>
          <a-tooltip :content="t('ai.history')">
            <a-button type="text" size="small" @click="handleToggleHistory">
              <template #icon><icon-history /></template>
            </a-button>
          </a-tooltip>
          <a-tooltip :content="t('ai.config')">
            <a-button type="text" size="small" @click="showConfig = true">
              <template #icon><icon-settings /></template>
            </a-button>
          </a-tooltip>
        </a-space>
      </div>

      <div ref="messageListRef" class="ai-panel__messages">
        <div v-if="aiStore.messages.length === 0" class="ai-panel__welcome">
          <icon-robot :size="40" style="color: var(--color-text-4)" />
          <div class="ai-panel__welcome-text">{{ t("ai.welcomeText") }}</div>
          <div class="ai-panel__welcome-hint">{{ t("ai.welcomeHint") }}</div>
        </div>
        <ChatMessage
          v-for="(msg, idx) in aiStore.messages"
          :key="idx"
          :role="msg.role"
          :content="msg.content"
        />
        <ChatMessage
          v-if="aiStore.isStreaming && aiStore.streamingContent"
          role="assistant"
          :content="aiStore.streamingContent"
        />
        <div v-if="aiStore.isStreaming && !aiStore.streamingContent" class="ai-panel__typing">
          <a-spin :size="14" />
          <span>{{ t("ai.thinking") }}</span>
        </div>
      </div>

      <div class="ai-panel__input">
        <a-textarea
          v-model="inputText"
          :placeholder="t('ai.placeholder')"
          :auto-size="{ minRows: 1, maxRows: 6 }"
          :disabled="aiStore.isStreaming"
          @keydown="handleKeydown"
        />
        <div class="ai-panel__input-actions">
          <a-button
            v-if="aiStore.isStreaming"
            type="primary"
            status="danger"
            size="small"
            @click="aiStore.stopGeneration()"
          >
            <template #icon><icon-pause /></template>
            {{ t("ai.stop") }}
          </a-button>
          <a-button
            v-else
            type="primary"
            size="small"
            :disabled="!inputText.trim()"
            @click="handleSend"
          >
            <template #icon><icon-send /></template>
            {{ t("ai.send") }}
          </a-button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.ai-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-1);
}

.ai-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.ai-panel__title {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-text-1);
}

.ai-panel__messages {
  flex: 1;
  overflow-y: auto;
  padding: 8px 10px;
}

.ai-panel__welcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  text-align: center;
  padding: 20px;
}

.ai-panel__welcome-text {
  font-size: 15px;
  font-weight: 500;
  color: var(--color-text-2);
}

.ai-panel__welcome-hint {
  font-size: 12px;
  color: var(--color-text-4);
  max-width: 260px;
  line-height: 1.5;
}

.ai-panel__typing {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  color: var(--color-text-3);
  font-size: 13px;
}

.ai-panel__input {
  padding: 10px 14px 14px;
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}

.ai-panel__input-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
}

/* ── History panel ── */
.ai-panel__history {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.ai-panel__history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-border);
}

.ai-panel__history-title {
  font-weight: 600;
  font-size: 14px;
}

.ai-panel__history-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.ai-panel__history-item {
  position: relative;
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 4px;
  transition: background 0.15s;
}

.ai-panel__history-item:hover {
  background: var(--color-fill-2);
}

.ai-panel__history-item-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 24px;
}

.ai-panel__history-item-meta {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--color-text-4);
  margin-top: 3px;
}

.ai-panel__history-item-delete {
  position: absolute;
  top: 8px;
  right: 4px;
  opacity: 0;
  transition: opacity 0.15s;
}

.ai-panel__history-item:hover .ai-panel__history-item-delete {
  opacity: 1;
}
</style>
