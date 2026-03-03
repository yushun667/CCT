<script lang="ts">
/**
 * Dockview 编辑区 — 使用 dockview-vue 管理 Tab 布局
 *
 * dockview 通过 findComponent 按名称查找已注册组件来渲染面板内容，
 * 因此必须通过 Options API 的 components 选项注册面板组件。
 */
import CodeEditorPanel from "./CodeEditorPanel.vue";
import CallGraphPanel from "./CallGraphPanel.vue";

export default {
  components: {
    codeEditor: CodeEditorPanel,
    callGraph: CallGraphPanel,
  },
};
</script>

<script setup lang="ts">
import { provide } from "vue";
import { DockviewVue } from "dockview-vue";
import type { DockviewReadyEvent } from "dockview-core";
import "dockview-vue/dist/styles/dockview.css";
import { useEditorStore } from "@/stores/editor";

const emit = defineEmits<{
  (e: "show-call-graph", line: number, column: number): void;
  (e: "show-callers", line: number, column: number): void;
  (e: "find-references", line: number, column: number): void;
}>();

const editorStore = useEditorStore();

provide("editorEmit", {
  showCallGraph: (line: number, col: number) => emit("show-call-graph", line, col),
  showCallers: (line: number, col: number) => emit("show-callers", line, col),
  findReferences: (line: number, col: number) => emit("find-references", line, col),
});

function onReady(event: DockviewReadyEvent) {
  editorStore.setDockApi(event.api);
}
</script>

<template>
  <div class="dockview-editor dockview-theme-dark">
    <DockviewVue @ready="onReady" />
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
