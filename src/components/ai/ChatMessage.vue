<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useEditorStore } from "@/stores/editor";

const props = defineProps<{
  role: "user" | "assistant" | "system";
  content: string;
}>();

const router = useRouter();
const editorStore = useEditorStore();

const isUser = computed(() => props.role === "user");

/**
 * 将 markdown 文本转换为简易 HTML。
 * 支持行内代码、代码块、加粗、标题和文件引用链接。
 */
function renderMarkdown(text: string): string {
  let html = escapeHtml(text);

  // 代码块 ```lang\n...\n```
  html = html.replace(
    /```(\w*)\n([\s\S]*?)```/g,
    (_match, lang, code) =>
      `<pre class="code-block" data-lang="${lang}"><code>${code.trim()}</code></pre>`,
  );

  // 行内代码
  html = html.replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>');

  // 加粗
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");

  // 标题 (##)
  html = html.replace(/^### (.+)$/gm, '<h4 class="md-heading">$1</h4>');
  html = html.replace(/^## (.+)$/gm, '<h3 class="md-heading">$1</h3>');

  // 文件:行 引用 → 可点击
  html = html.replace(
    /(\S+\.\w+):(\d+)/g,
    '<a class="file-ref" href="#" data-file="$1" data-line="$2">$1:$2</a>',
  );

  // 引用块 > ...
  html = html.replace(
    /^&gt; (.+)$/gm,
    '<blockquote class="md-quote">$1</blockquote>',
  );

  // 水平线
  html = html.replace(/^---$/gm, '<hr class="md-hr" />');

  // 无序列表
  html = html.replace(/^- (.+)$/gm, '<li class="md-li">$1</li>');

  // 段落（连续换行）
  html = html.replace(/\n\n/g, "</p><p>");
  html = `<p>${html}</p>`;
  html = html.replace(/<p><\/p>/g, "");

  return html;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

async function handleClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (target.classList.contains("file-ref")) {
    event.preventDefault();
    const file = target.dataset.file;
    const line = target.dataset.line;
    if (file) {
      await editorStore.openFile(file);
      await router.push("/editor");
      if (line) {
        console.log(`[ChatMessage] 打开文件 ${file}:${line}`);
      }
    }
  }
}

const renderedContent = computed(() => renderMarkdown(props.content));
</script>

<template>
  <div
    :class="['chat-message', isUser ? 'chat-message--user' : 'chat-message--assistant']"
    @click="handleClick"
  >
    <div class="chat-message__avatar">
      <a-avatar :size="28" :style="{ backgroundColor: isUser ? 'rgb(var(--primary-6))' : 'rgb(var(--success-6))' }">
        <template #icon>
          <icon-user v-if="isUser" />
          <icon-robot v-else />
        </template>
      </a-avatar>
    </div>
    <div class="chat-message__body">
      <div class="chat-message__role">{{ isUser ? "You" : "AI" }}</div>
      <!-- eslint-disable-next-line vue/no-v-html -->
      <div class="chat-message__content" v-html="renderedContent" />
    </div>
  </div>
</template>

<style scoped>
.chat-message {
  display: flex;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 8px;
  margin-bottom: 8px;
}

.chat-message--user {
  background: var(--color-fill-2);
}

.chat-message--assistant {
  background: var(--color-bg-2);
}

.chat-message__avatar {
  flex-shrink: 0;
  padding-top: 2px;
}

.chat-message__body {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.chat-message__role {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-2);
  margin-bottom: 4px;
}

.chat-message__content {
  font-size: 13px;
  line-height: 1.6;
  color: var(--color-text-1);
  word-wrap: break-word;
  overflow-wrap: break-word;
}

.chat-message__content :deep(.code-block) {
  background: var(--color-fill-3);
  border-radius: 6px;
  padding: 10px 12px;
  margin: 8px 0;
  overflow-x: auto;
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 12px;
  line-height: 1.5;
}

.chat-message__content :deep(.inline-code) {
  background: var(--color-fill-3);
  border-radius: 3px;
  padding: 1px 5px;
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 12px;
}

.chat-message__content :deep(.file-ref) {
  color: rgb(var(--primary-6));
  text-decoration: none;
  cursor: pointer;
}

.chat-message__content :deep(.file-ref:hover) {
  text-decoration: underline;
}

.chat-message__content :deep(.md-heading) {
  margin: 8px 0 4px;
  font-weight: 600;
}

.chat-message__content :deep(.md-quote) {
  border-left: 3px solid var(--color-border-3);
  padding-left: 10px;
  margin: 6px 0;
  color: var(--color-text-3);
}

.chat-message__content :deep(.md-hr) {
  border: none;
  border-top: 1px solid var(--color-border-2);
  margin: 8px 0;
}

.chat-message__content :deep(strong) {
  font-weight: 600;
}
</style>
