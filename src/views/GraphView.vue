<script setup lang="ts">
/**
 * 图视图 — 展示调用图和文件依赖图
 */
import { useI18n } from "vue-i18n";
import { useGraphStore } from "@/stores/graph";
import { useProjectStore } from "@/stores/project";
import GraphRenderer from "@/components/graph/GraphRenderer.vue";

const { t } = useI18n();
const graphStore = useGraphStore();
const projectStore = useProjectStore();

async function loadFileGraph() {
  if (!projectStore.currentProjectId) return;
  await graphStore.loadFileDependencyGraph(projectStore.currentProjectId);
}
</script>

<template>
  <div class="graph-view">
    <div class="graph-header">
      <a-space>
        <a-button
          type="primary"
          size="small"
          :loading="graphStore.loading"
          @click="loadFileGraph"
        >
          {{ t("graph.fileGraph") }}
        </a-button>
        <a-input-number
          v-model="graphStore.depth"
          :min="1"
          :max="10"
          size="small"
          style="width: 100px"
        />
      </a-space>
    </div>
    <div class="graph-body">
      <GraphRenderer :graph-data="graphStore.graphData" />
    </div>
  </div>
</template>

<style scoped>
.graph-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.graph-header {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.graph-body {
  flex: 1;
  overflow: hidden;
}
</style>
