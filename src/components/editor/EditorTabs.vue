<script setup lang="ts">
/**
 * 编辑器 Tab 栏 — 多文件/多类型标签管理
 *
 * 支持：
 *   右键菜单 — 关闭 / 关闭其他 / 关闭所有 / 拆分到右侧
 *   拖拽标签 — 同窗格排序 / 跨窗格移动
 */
import { ref, computed } from "vue";
import { useEditorStore } from "@/stores/editor";
import type { EditorFile } from "@/api/types";

import MdiClose from "~icons/mdi/close";
import MdiArrowSplitVertical from "~icons/mdi/arrow-split-vertical";
import MdiCloseCircleOutline from "~icons/mdi/close-circle-outline";
import MdiFileDocumentOutline from "~icons/mdi/file-document-outline";
import MdiGraphOutline from "~icons/mdi/graph-outline";

const props = withDefaults(defineProps<{
  paneIndex?: number;
}>(), {
  paneIndex: 0,
});

const editorStore = useEditorStore();

const files = computed(() => editorStore.getPaneFiles(props.paneIndex));
const activeIndex = computed(() => editorStore.getPaneActiveIndex(props.paneIndex));

/* ---------- Tab 基础操作 ---------- */

function onTabClick(index: number) {
  editorStore.setActiveFile(index, props.paneIndex);
}

function onTabClose(index: number, event: Event) {
  event.stopPropagation();
  editorStore.closeFile(index, props.paneIndex);
}

/* ---------- 右键上下文菜单 ---------- */

const ctxMenu = ref({ visible: false, x: 0, y: 0, index: -1 });

function onTabContext(index: number, event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  ctxMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    index,
  };
  document.addEventListener("click", closeCtxMenu, { once: true });
}

function closeCtxMenu() {
  ctxMenu.value.visible = false;
}

function ctxClose() {
  editorStore.closeFile(ctxMenu.value.index, props.paneIndex);
  closeCtxMenu();
}

function ctxCloseOthers() {
  editorStore.closeOtherFiles(ctxMenu.value.index, props.paneIndex);
  closeCtxMenu();
}

function ctxCloseAll() {
  editorStore.closeAllInPane(props.paneIndex);
  closeCtxMenu();
}

function ctxSplitRight() {
  editorStore.splitFileToRight(ctxMenu.value.index, props.paneIndex);
  closeCtxMenu();
}

/* ---------- 拖拽排序 / 跨窗格移动 ---------- */

const dragOverIndex = ref(-1);
const dragOverSide = ref<"left" | "right">("left");

function onDragStart(index: number, event: DragEvent) {
  editorStore.setDragTab(props.paneIndex, index);
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData(
      "application/x-editor-tab",
      JSON.stringify({ paneIndex: props.paneIndex, fileIndex: index }),
    );
    event.dataTransfer.setData("text/plain", JSON.stringify({ paneIndex: props.paneIndex, fileIndex: index }));
  }
}

function onDragOver(index: number, event: DragEvent) {
  event.preventDefault();
  if (!event.dataTransfer) return;
  event.dataTransfer.dropEffect = "move";
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  dragOverSide.value = event.clientX < rect.left + rect.width / 2 ? "left" : "right";
  dragOverIndex.value = index;
}

function onDragLeave() {
  dragOverIndex.value = -1;
}

function onDrop(index: number, event: DragEvent) {
  event.preventDefault();
  dragOverIndex.value = -1;
  const data = editorStore.getDragTabFromEvent(event);
  if (!data) return;

  const toIdx = dragOverSide.value === "right" ? index + 1 : index;

  if (data.paneIndex === props.paneIndex) {
    editorStore.reorderFile(props.paneIndex, data.fileIndex, toIdx);
  } else {
    editorStore.moveToPane(data.paneIndex, props.paneIndex, data.fileIndex, toIdx);
  }
}

function onBarDrop(event: DragEvent) {
  event.preventDefault();
  dragOverIndex.value = -1;
  const data = editorStore.getDragTabFromEvent(event);
  if (!data) return;

  const toIdx = files.value.length;

  if (data.paneIndex === props.paneIndex) {
    editorStore.reorderFile(props.paneIndex, data.fileIndex, toIdx);
  } else {
    editorStore.moveToPane(data.paneIndex, props.paneIndex, data.fileIndex, toIdx);
  }
}

function onDragEnd() {
  dragOverIndex.value = -1;
  editorStore.clearDragTab();
}

/* ---------- 辅助 ---------- */

function getTabIcon(file: EditorFile) {
  return file.type === "call-graph" ? MdiGraphOutline : MdiFileDocumentOutline;
}
</script>

<template>
  <div
    class="editor-tabs"
    @dragover.prevent
    @drop.prevent="onBarDrop"
  >
    <div
      v-if="files.length === 0"
      class="no-files"
    >
      <span>无打开的文件</span>
    </div>
    <div
      v-for="(file, index) in files"
      :key="file.filePath"
      :class="[
        'tab',
        { active: index === activeIndex },
        { 'drop-left': dragOverIndex === index && dragOverSide === 'left' },
        { 'drop-right': dragOverIndex === index && dragOverSide === 'right' },
      ]"
      draggable="true"
      @click="onTabClick(index)"
      @contextmenu="onTabContext(index, $event)"
      @dragstart="onDragStart(index, $event)"
      @dragover.prevent="onDragOver(index, $event)"
      @dragleave="onDragLeave"
      @drop.stop="onDrop(index, $event)"
      @dragend="onDragEnd"
    >
      <component :is="getTabIcon(file)" class="tab-icon" />
      <span
        class="tab-name"
        :title="file.type === 'call-graph' ? file.fileName : file.filePath"
      >
        {{ file.fileName }}
      </span>
      <a-button
        type="text"
        size="mini"
        class="tab-close"
        @click="onTabClose(index, $event)"
      >
        <template #icon><MdiClose /></template>
      </a-button>
    </div>

    <div class="tab-actions">
      <a-tooltip v-if="!editorStore.splitMode" content="拆分编辑器" position="bottom" mini>
        <a-button size="mini" type="text" @click="editorStore.splitRight()">
          <template #icon><MdiArrowSplitVertical /></template>
        </a-button>
      </a-tooltip>
      <a-tooltip v-if="editorStore.splitMode && paneIndex === 1" content="关闭拆分" position="bottom" mini>
        <a-button size="mini" type="text" @click="editorStore.closeSplit()">
          <template #icon><MdiCloseCircleOutline /></template>
        </a-button>
      </a-tooltip>
    </div>
  </div>

  <!-- 右键菜单 -->
  <Teleport to="body">
    <div
      v-if="ctxMenu.visible"
      class="tab-context-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
    >
      <div class="ctx-item" @click="ctxClose">关闭</div>
      <div class="ctx-item" @click="ctxCloseOthers">关闭其他</div>
      <div class="ctx-item" @click="ctxCloseAll">关闭所有</div>
      <div class="ctx-divider" />
      <div class="ctx-item" @click="ctxSplitRight">
        向右拆分
      </div>
    </div>
  </Teleport>
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
  position: relative;
}

.tab:hover {
  background: var(--color-fill-2);
}

.tab.active {
  background: var(--color-bg-1);
  color: var(--color-text-1);
  border-bottom: 2px solid rgb(var(--primary-6));
}

.tab.drop-left::before,
.tab.drop-right::after {
  content: "";
  position: absolute;
  top: 4px;
  bottom: 4px;
  width: 2px;
  background: rgb(var(--primary-6));
  border-radius: 1px;
}

.tab.drop-left::before {
  left: -1px;
}

.tab.drop-right::after {
  right: -1px;
}

.tab-icon {
  font-size: 14px;
  flex-shrink: 0;
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

<style>
.tab-context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 140px;
  background: var(--color-bg-2, #fff);
  border: 1px solid var(--color-border, #e5e6eb);
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  padding: 4px 0;
  font-size: 13px;
}

.tab-context-menu .ctx-item {
  padding: 6px 14px;
  cursor: pointer;
  color: var(--color-text-1, #1d2129);
  transition: background 0.15s;
}

.tab-context-menu .ctx-item:hover {
  background: var(--color-fill-2, #f2f3f5);
}

.tab-context-menu .ctx-divider {
  height: 1px;
  margin: 4px 0;
  background: var(--color-border, #e5e6eb);
}
</style>
