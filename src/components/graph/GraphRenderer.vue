<script setup lang="ts">
/**
 * 图渲染器 — 调用图 / 文件依赖图可视化
 *
 * # 设计说明
 * 当前版本使用 Arco Design 的树/列表组件渲染图结构，
 * 后续迭代将替换为 PixiJS/WebGL 实现高性能力导向布局。
 * Canvas 元素已预留，用于未来 PixiJS 集成。
 */
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useGraphStore } from "@/stores/graph";
import type { GraphNode, GraphEdge } from "@/api/types";

const { t } = useI18n();
const graphStore = useGraphStore();

const props = defineProps<{
  graphData: { nodes: GraphNode[]; edges: GraphEdge[] } | null;
}>();

const emit = defineEmits<{
  (e: "node-click", node: GraphNode): void;
}>();

interface TreeItem {
  key: string;
  title: string;
  kind: string;
  node: GraphNode;
  children: TreeItem[];
}

const adjacencyList = computed(() => {
  if (!props.graphData) return new Map<string, string[]>();
  const adj = new Map<string, string[]>();
  for (const edge of props.graphData.edges) {
    const children = adj.get(edge.source) ?? [];
    children.push(edge.target);
    adj.set(edge.source, children);
  }
  return adj;
});

const nodeMap = computed(() => {
  if (!props.graphData) return new Map<string, GraphNode>();
  const map = new Map<string, GraphNode>();
  for (const node of props.graphData.nodes) {
    map.set(node.id, node);
  }
  return map;
});

const rootNodes = computed(() => {
  if (!props.graphData) return [];
  const targetIds = new Set(props.graphData.edges.map((e) => e.target));
  return props.graphData.nodes.filter((n) => !targetIds.has(n.id));
});

function buildTree(nodeId: string, visited: Set<string>): TreeItem | null {
  if (visited.has(nodeId)) return null;
  visited.add(nodeId);

  const node = nodeMap.value.get(nodeId);
  if (!node) return null;

  const childIds = adjacencyList.value.get(nodeId) ?? [];
  const children: TreeItem[] = [];
  for (const childId of childIds) {
    const child = buildTree(childId, visited);
    if (child) children.push(child);
  }

  return {
    key: node.id,
    title: node.label,
    kind: node.kind,
    node,
    children,
  };
}

const treeData = computed(() => {
  const visited = new Set<string>();
  const trees: TreeItem[] = [];
  for (const root of rootNodes.value) {
    const tree = buildTree(root.id, visited);
    if (tree) trees.push(tree);
  }
  return trees;
});

function flattenTree(items: TreeItem[], depth: number): Array<{ item: TreeItem; depth: number }> {
  const result: Array<{ item: TreeItem; depth: number }> = [];
  for (const item of items) {
    result.push({ item, depth });
    if (item.children.length > 0) {
      result.push(...flattenTree(item.children, depth + 1));
    }
  }
  return result;
}

const flatNodes = computed(() => flattenTree(treeData.value, 0));

function onNodeClick(node: GraphNode) {
  graphStore.selectNode(node.id);
  emit("node-click", node);
}

function dotColor(kind: string): string {
  switch (kind) {
    case "Function":
      return "rgb(var(--primary-6, 22, 93, 255))";
    case "File":
      return "rgb(var(--success-6, 0, 180, 42))";
    case "Type":
      return "rgb(var(--warning-6, 255, 125, 0))";
    default:
      return "#999";
  }
}
</script>

<template>
  <div class="graph-renderer">
    <div class="graph-toolbar">
      <a-space>
        <a-tag>{{ t("graph.title") }}</a-tag>
        <a-tag color="arcoblue">
          {{ graphStore.nodeCount }} {{ t("parse.symbols") }}
        </a-tag>
        <a-tag color="green">
          {{ graphStore.edgeCount }} {{ t("parse.relations") }}
        </a-tag>
      </a-space>
    </div>

    <!-- 预留 Canvas（未来 PixiJS 集成） -->
    <canvas class="graph-canvas" style="display: none" />

    <div v-if="!props.graphData" class="empty-state">
      <a-empty :description="t('search.noResults')" />
    </div>
    <div v-else class="tree-container">
      <div v-if="flatNodes.length === 0" class="empty-state">
        <a-empty :description="t('search.noResults')" />
      </div>
      <div v-else class="tree-list">
        <div
          v-for="({ item, depth }, idx) in flatNodes"
          :key="`${item.key}-${idx}`"
          class="tree-node-content"
          :style="{ paddingLeft: `${depth * 20 + 8}px` }"
          @click="onNodeClick(item.node)"
        >
          <span
            class="node-dot"
            :style="{ background: dotColor(item.kind) }"
          />
          <span class="node-label">{{ item.title }}</span>
          <span class="node-kind">{{ item.kind }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.graph-renderer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.graph-toolbar {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.graph-canvas {
  width: 100%;
  height: 100%;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 48px;
}

.tree-container {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

.tree-list {
  display: flex;
  flex-direction: column;
}

.tree-node-content {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
  font-size: 13px;
}

.tree-node-content:hover {
  background: var(--color-fill-2);
}

.node-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.node-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-kind {
  font-size: 11px;
  color: var(--color-text-3);
  flex-shrink: 0;
}
</style>
