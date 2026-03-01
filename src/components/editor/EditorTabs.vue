<script setup lang="ts">
/**
 * 编辑器 Tab 栏 — 多文件标签管理
 *
 * 显示已打开的文件标签，支持切换和关闭。
 * 活跃标签高亮显示，带关闭按钮。
 */
import { useEditorStore } from "@/stores/editor";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const editorStore = useEditorStore();

function onTabClick(index: number) {
  editorStore.setActiveFile(index);
}

function onTabClose(index: number, event: Event) {
  event.stopPropagation();
  editorStore.closeFile(index);
}

function getFileIcon(language: string): string {
  const iconMap: Record<string, string> = {
    c: "icon-file",
    cpp: "icon-file",
    json: "icon-file",
    markdown: "icon-file",
    plaintext: "icon-file",
  };
  return iconMap[language] ?? "icon-file";
}
</script>

<template>
  <div class="editor-tabs">
    <div
      v-if="editorStore.openFiles.length === 0"
      class="no-files"
    >
      <span>{{ t("editor.noFileOpen") }}</span>
    </div>
    <div
      v-for="(file, index) in editorStore.openFiles"
      :key="file.filePath"
      :class="['tab', { active: index === editorStore.activeFileIndex }]"
      @click="onTabClick(index)"
    >
      <component :is="getFileIcon(file.language)" class="tab-icon" />
      <span class="tab-name" :title="file.filePath">{{ file.fileName }}</span>
      <a-button
        type="text"
        size="mini"
        class="tab-close"
        @click="onTabClose(index, $event)"
      >
        <template #icon><icon-close /></template>
      </a-button>
    </div>
  </div>
</template>

<style scoped>
.editor-tabs {
  display: flex;
  align-items: center;
  height: 36px;
  background: var(--color-bg-2);
  border-bottom: 1px solid var(--color-border);
  overflow-x: auto;
  flex-shrink: 0;
}

.no-files {
  display: flex;
  align-items: center;
  padding: 0 12px;
  color: var(--color-text-3);
  font-size: 12px;
}

.tab {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 8px;
  height: 100%;
  cursor: pointer;
  border-right: 1px solid var(--color-border);
  font-size: 12px;
  color: var(--color-text-2);
  white-space: nowrap;
  transition: background-color 0.15s;
}

.tab:hover {
  background: var(--color-fill-2);
}

.tab.active {
  background: var(--color-bg-1);
  color: var(--color-text-1);
  border-bottom: 2px solid rgb(var(--primary-6));
}

.tab-name {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-close {
  opacity: 0;
  transition: opacity 0.15s;
}

.tab:hover .tab-close,
.tab.active .tab-close {
  opacity: 1;
}
</style>
