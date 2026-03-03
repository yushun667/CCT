<script setup lang="ts">
/**
 * CodeEditor 的 dockview 面板包装器
 *
 * dockview 通过 findComponent 查找并挂载本组件，
 * 接收 dockview 的 params 格式，从 store 查询文件数据后传递给 CodeEditor。
 * 通过 inject 获取由 DockviewEditor 提供的事件发射函数。
 */
import { computed, inject } from "vue";
import { useEditorStore } from "@/stores/editor";
import CodeEditor from "./CodeEditor.vue";

const props = defineProps<{
  params: {
    params: { panelId: string };
    api: any;
    containerApi: any;
  };
}>();

const editorStore = useEditorStore();

const panelId = computed(() => props.params.params.panelId);
const fileData = computed(() => editorStore.panelDataMap.get(panelId.value));

const editorEmit = inject<{
  showCallGraph: (line: number, col: number) => void;
  showCallers: (line: number, col: number) => void;
  findReferences: (line: number, col: number) => void;
}>("editorEmit")!;
</script>

<template>
  <CodeEditor
    v-if="fileData"
    :file-path="fileData.filePath"
    :content="fileData.content"
    :language="fileData.language"
    :line="editorStore.targetPanelId === panelId ? editorStore.targetLine : null"
    :line-seq="editorStore.targetPanelId === panelId ? editorStore.targetLineSeq : 0"
    @show-call-graph="(l: number, c: number) => editorEmit.showCallGraph(l, c)"
    @show-callers="(l: number, c: number) => editorEmit.showCallers(l, c)"
    @find-references="(l: number, c: number) => editorEmit.findReferences(l, c)"
  />
</template>
