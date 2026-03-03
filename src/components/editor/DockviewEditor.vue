<script setup lang="ts">
/**
 * Dockview 编辑区 — 使用 dockview-vue 管理 Tab 布局
 *
 * 替代原有 SplitEditor + EditorTabs，由 dockview 接管
 * Tab 渲染、拖拽排序、分栏分组、右键菜单等窗口管理职责。
 */
import { DockviewVue } from "dockview-vue";
import type { DockviewReadyEvent } from "dockview-core";
import "dockview-vue/dist/styles/dockview.css";
import { useEditorStore } from "@/stores/editor";
import CodeEditor from "./CodeEditor.vue";
import CallGraphTab from "@/components/graph/CallGraphTab.vue";

const emit = defineEmits<{
  (e: "show-call-graph", line: number, column: number): void;
  (e: "show-callers", line: number, column: number): void;
  (e: "find-references", line: number, column: number): void;
}>();

const editorStore = useEditorStore();

function onReady(event: DockviewReadyEvent) {
  editorStore.setDockApi(event.api);
}
</script>

<template>
  <div class="dockview-editor dockview-theme-dark">
    <DockviewVue @ready="onReady">
      <template #codeEditor="{ params }">
        <CodeEditor
          v-if="editorStore.panelDataMap.get(params.panelId)"
          :file-path="editorStore.panelDataMap.get(params.panelId)!.filePath"
          :content="editorStore.panelDataMap.get(params.panelId)!.content"
          :language="editorStore.panelDataMap.get(params.panelId)!.language"
          :line="editorStore.targetPanelId === params.panelId ? editorStore.targetLine : null"
          :line-seq="editorStore.targetPanelId === params.panelId ? editorStore.targetLineSeq : 0"
          @show-call-graph="(l: number, c: number) => emit('show-call-graph', l, c)"
          @show-callers="(l: number, c: number) => emit('show-callers', l, c)"
          @find-references="(l: number, c: number) => emit('find-references', l, c)"
        />
      </template>
      <template #callGraph="{ params }">
        <CallGraphTab
          v-if="editorStore.panelDataMap.get(params.panelId)?.graphData"
          :tab-id="params.panelId"
          :graph-data="editorStore.panelDataMap.get(params.panelId)!.graphData!"
        />
      </template>
    </DockviewVue>
  </div>
</template>

<style scoped>
.dockview-editor {
  position: absolute;
  inset: 0;
}
</style>

<style>
.dockview-theme-dark {
  --dv-tabs-and-actions-container-font-size: 12px;
  --dv-tabs-and-actions-container-height: 36px;
  --dv-activegroup-visibilitypanel-tab-background-color: var(--color-bg-1, #1e1e1e);
  --dv-activegroup-hiddenpanel-tab-background-color: var(--color-bg-2, #2b2b2b);
  --dv-inactivegroup-visibilitypanel-tab-background-color: var(--color-bg-1, #1e1e1e);
  --dv-inactivegroup-hiddenpanel-tab-background-color: var(--color-bg-2, #2b2b2b);
  --dv-tabs-container-scrollbar-color: var(--color-fill-3, #484849);
  --dv-group-view-background-color: var(--color-bg-1, #1e1e1e);
}
</style>
