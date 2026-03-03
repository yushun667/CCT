<script setup lang="ts">
/**
 * CallGraphTab 的 dockview 面板包装器
 *
 * 接收 dockview 的 params 格式，从 store 查询调用图数据后传递给 CallGraphTab。
 */
import { computed } from "vue";
import { useEditorStore } from "@/stores/editor";
import CallGraphTab from "@/components/graph/CallGraphTab.vue";

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
</script>

<template>
  <CallGraphTab
    v-if="fileData?.graphData"
    :tab-id="panelId"
    :graph-data="fileData.graphData!"
  />
</template>
