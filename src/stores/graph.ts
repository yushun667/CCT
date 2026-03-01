/**
 * 图视图 Store — 管理调用图与文件依赖图数据
 *
 * # 设计说明（观察者模式）
 * 图数据加载完成后，组件通过 reactive 引用自动更新渲染。
 * 支持调用图和文件依赖图两种模式。
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { GraphData, GraphNode } from "@/api/types";
import * as graphApi from "@/api/graph";

export type GraphMode = "call" | "file-dependency";

export const useGraphStore = defineStore("graph", () => {
  const graphData = ref<GraphData | null>(null);
  const graphMode = ref<GraphMode>("call");
  const rootSymbolId = ref<number | null>(null);
  const depth = ref(2);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const selectedNodeId = ref<string | null>(null);

  const nodeCount = computed(() => graphData.value?.nodes.length ?? 0);
  const edgeCount = computed(() => graphData.value?.edges.length ?? 0);

  const selectedNode = computed<GraphNode | null>(() => {
    if (!selectedNodeId.value || !graphData.value) return null;
    return (
      graphData.value.nodes.find((n) => n.id === selectedNodeId.value) ?? null
    );
  });

  async function loadCallGraph(projectId: string, symbolId: number) {
    loading.value = true;
    error.value = null;
    graphMode.value = "call";
    rootSymbolId.value = symbolId;

    try {
      graphData.value = await graphApi.getCallGraph(
        projectId,
        symbolId,
        depth.value,
      );
    } catch (e) {
      error.value = String(e);
      graphData.value = null;
    } finally {
      loading.value = false;
    }
  }

  async function loadFileDependencyGraph(projectId: string) {
    loading.value = true;
    error.value = null;
    graphMode.value = "file-dependency";
    rootSymbolId.value = null;

    try {
      graphData.value = await graphApi.getFileDependencyGraph(projectId);
    } catch (e) {
      error.value = String(e);
      graphData.value = null;
    } finally {
      loading.value = false;
    }
  }

  function setDepth(newDepth: number) {
    depth.value = Math.max(1, Math.min(newDepth, 10));
  }

  function selectNode(nodeId: string | null) {
    selectedNodeId.value = nodeId;
  }

  function clearGraph() {
    graphData.value = null;
    selectedNodeId.value = null;
    error.value = null;
  }

  return {
    graphData,
    graphMode,
    rootSymbolId,
    depth,
    loading,
    error,
    selectedNodeId,
    selectedNode,
    nodeCount,
    edgeCount,
    loadCallGraph,
    loadFileDependencyGraph,
    setDepth,
    selectNode,
    clearGraph,
  };
});
