<script setup lang="ts">
/**
 * 编辑器 Tab 栏 — 多文件/多类型标签管理
 *
 * 通过 paneIndex prop 感知所属窗格，支持文件和调用图两种 Tab 类型。
 * 调用图 Tab 显示专用图标和标题。
 */
import { computed } from "vue";
import { useEditorStore } from "@/stores/editor";
import { useI18n } from "vue-i18n";
import type { EditorFile } from "@/api/types";

const props = withDefaults(defineProps<{
  paneIndex?: number;
}>(), {
  paneIndex: 0,
});

const { t } = useI18n();
const editorStore = useEditorStore();

const files = computed(() => editorStore.getPaneFiles(props.paneIndex));
const activeIndex = computed(() => editorStore.getPaneActiveIndex(props.paneIndex));

function onTabClick(index: number) {
  editorStore.setActiveFile(index, props.paneIndex);
}

function onTabClose(index: number, event: Event) {
  event.stopPropagation();
  editorStore.closeFile(index, props.paneIndex);
}

function getTabIcon(file: EditorFile): string {
  if (file.type === "call-graph") return "icon-relation-one-to-many";
  return "icon-file";
}

function getTabLabel(file: EditorFile): string {
  if (file.type === "call-graph") return file.fileName;
  return file.fileName;
}

function onSplitRight() {
  editorStore.splitRight();
}

function onCloseSplit() {
  editorStore.closeSplit();
}
</script>

<template>
  <div class="editor-tabs">
    <div
      v-if="files.length === 0"
      class="no-files"
    >
      <span>{{ t("editor.noFileOpen") }}</span>
    </div>
    <div
      v-for="(file, index) in files"
      :key="file.filePath"
      :class="['tab', { active: index === activeIndex }]"
      @click="onTabClick(index)"
    >
      <component :is="getTabIcon(file)" class="tab-icon" />
      <span class="tab-name" :title="file.type === 'call-graph' ? file.fileName : file.filePath">
        {{ getTabLabel(file) }}
      </span>
      <a-button
        type="text"
        size="mini"
        class="tab-close"
        @click="onTabClose(index, $event)"
      >
        <template #icon><icon-close /></template>
      </a-button>
    </div>

    <div class="tab-actions">
      <a-button
        v-if="!editorStore.splitMode"
        size="mini"
        type="text"
        title="拆分编辑器"
        @click="onSplitRight"
      >
        <template #icon><icon-swap /></template>
      </a-button>
      <a-button
        v-if="editorStore.splitMode && paneIndex === 1"
        size="mini"
        type="text"
        title="关闭拆分"
        @click="onCloseSplit"
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

.tab-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  padding: 0 4px;
  flex-shrink: 0;
}
</style>
