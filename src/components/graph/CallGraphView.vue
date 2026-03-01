<script setup lang="ts">
/**
 * 可视化调用图 — 使用 dagre 树状布局 + SVG 渲染
 *
 * 中心节点为当前选中的函数，上方为调用者（callers），下方为被调用者（callees）。
 * 每个节点显示函数名 + 文件名:行号，点击可跳转到代码。
 */
import { ref, computed, watch, onMounted, nextTick } from "vue";
import dagre from "@dagrejs/dagre";
import type { Symbol as CctSymbol } from "@/api/types";

const props = defineProps<{
  rootSymbol: CctSymbol;
  callers: CctSymbol[];
  callees: CctSymbol[];
}>();

const emit = defineEmits<{
  (e: "navigate", sym: CctSymbol): void;
  (e: "expand", sym: CctSymbol): void;
}>();

interface LayoutNode {
  id: string;
  sym: CctSymbol;
  x: number;
  y: number;
  width: number;
  height: number;
  isRoot: boolean;
  kind: "caller" | "root" | "callee";
}

interface LayoutEdge {
  from: string;
  to: string;
  points: { x: number; y: number }[];
}

const NODE_W = 220;
const NODE_H = 48;
const nodes = ref<LayoutNode[]>([]);
const edges = ref<LayoutEdge[]>([]);
const svgWidth = ref(800);
const svgHeight = ref(600);

const viewBox = computed(() => {
  const pad = 40;
  if (nodes.value.length === 0) return `0 0 ${svgWidth.value} ${svgHeight.value}`;
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const n of nodes.value) {
    minX = Math.min(minX, n.x - n.width / 2);
    minY = Math.min(minY, n.y - n.height / 2);
    maxX = Math.max(maxX, n.x + n.width / 2);
    maxY = Math.max(maxY, n.y + n.height / 2);
  }
  return `${minX - pad} ${minY - pad} ${maxX - minX + pad * 2} ${maxY - minY + pad * 2}`;
});

// 交互状态
const pan = ref({ x: 0, y: 0 });
const scale = ref(1);
const dragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const hoveredNode = ref<string | null>(null);
const svgRef = ref<SVGSVGElement | null>(null);

function buildLayout() {
  const g = new dagre.graphlib.Graph();
  g.setGraph({
    rankdir: "TB",
    nodesep: 30,
    ranksep: 60,
    marginx: 20,
    marginy: 20,
  });
  g.setDefaultEdgeLabel(() => ({}));

  const rootId = `root-${props.rootSymbol.id}`;
  g.setNode(rootId, { width: NODE_W, height: NODE_H });

  const seen = new Set<number>([props.rootSymbol.id]);

  for (const caller of props.callers) {
    if (seen.has(caller.id)) continue;
    seen.add(caller.id);
    const cid = `caller-${caller.id}`;
    g.setNode(cid, { width: NODE_W, height: NODE_H });
    g.setEdge(cid, rootId);
  }

  for (const callee of props.callees) {
    if (seen.has(callee.id)) continue;
    seen.add(callee.id);
    const cid = `callee-${callee.id}`;
    g.setNode(cid, { width: NODE_W, height: NODE_H });
    g.setEdge(rootId, cid);
  }

  dagre.layout(g);

  const layoutNodes: LayoutNode[] = [];

  const rootNodeData = g.node(rootId);
  if (rootNodeData) {
    layoutNodes.push({
      id: rootId,
      sym: props.rootSymbol,
      x: rootNodeData.x,
      y: rootNodeData.y,
      width: NODE_W,
      height: NODE_H,
      isRoot: true,
      kind: "root",
    });
  }

  const seenLayout = new Set<number>([props.rootSymbol.id]);

  for (const caller of props.callers) {
    if (seenLayout.has(caller.id)) continue;
    seenLayout.add(caller.id);
    const cid = `caller-${caller.id}`;
    const nd = g.node(cid);
    if (nd) {
      layoutNodes.push({
        id: cid,
        sym: caller,
        x: nd.x,
        y: nd.y,
        width: NODE_W,
        height: NODE_H,
        isRoot: false,
        kind: "caller",
      });
    }
  }

  for (const callee of props.callees) {
    if (seenLayout.has(callee.id)) continue;
    seenLayout.add(callee.id);
    const cid = `callee-${callee.id}`;
    const nd = g.node(cid);
    if (nd) {
      layoutNodes.push({
        id: cid,
        sym: callee,
        x: nd.x,
        y: nd.y,
        width: NODE_W,
        height: NODE_H,
        isRoot: false,
        kind: "callee",
      });
    }
  }

  const layoutEdges: LayoutEdge[] = [];
  for (const e of g.edges()) {
    const edgeData = g.edge(e);
    if (edgeData && edgeData.points) {
      layoutEdges.push({
        from: e.v,
        to: e.w,
        points: edgeData.points as { x: number; y: number }[],
      });
    }
  }

  nodes.value = layoutNodes;
  edges.value = layoutEdges;
}

function edgePath(pts: { x: number; y: number }[]): string {
  if (pts.length < 2) return "";
  let d = `M ${pts[0].x} ${pts[0].y}`;
  if (pts.length === 2) {
    d += ` L ${pts[1].x} ${pts[1].y}`;
  } else {
    for (let i = 1; i < pts.length; i++) {
      d += ` L ${pts[i].x} ${pts[i].y}`;
    }
  }
  return d;
}

function nodeColor(kind: string): string {
  if (kind === "root") return "#1890ff";
  if (kind === "caller") return "#52c41a";
  return "#fa8c16";
}

function nodeBorderColor(kind: string): string {
  if (kind === "root") return "#096dd9";
  if (kind === "caller") return "#389e0d";
  return "#d46b08";
}

function shortFile(filePath: string): string {
  return filePath.split("/").pop() ?? filePath;
}

function handleNodeClick(node: LayoutNode) {
  emit("navigate", node.sym);
}

function handleNodeDblClick(node: LayoutNode) {
  if (!node.isRoot) {
    emit("expand", node.sym);
  }
}

function handleWheel(e: WheelEvent) {
  e.preventDefault();
  const factor = e.deltaY > 0 ? 0.9 : 1.1;
  scale.value = Math.max(0.2, Math.min(3, scale.value * factor));
}

function handleMouseDown(e: MouseEvent) {
  if (e.button === 0) {
    dragging.value = true;
    dragStart.value = { x: e.clientX - pan.value.x, y: e.clientY - pan.value.y };
  }
}

function handleMouseMove(e: MouseEvent) {
  if (dragging.value) {
    pan.value = {
      x: e.clientX - dragStart.value.x,
      y: e.clientY - dragStart.value.y,
    };
  }
}

function handleMouseUp() {
  dragging.value = false;
}

function resetView() {
  pan.value = { x: 0, y: 0 };
  scale.value = 1;
}

watch(
  () => [props.rootSymbol, props.callers, props.callees],
  () => {
    nextTick(() => buildLayout());
  },
  { immediate: true, deep: true },
);
</script>

<template>
  <div
    class="call-graph-view"
    @wheel.prevent="handleWheel"
    @mousedown="handleMouseDown"
    @mousemove="handleMouseMove"
    @mouseup="handleMouseUp"
    @mouseleave="handleMouseUp"
  >
    <!-- 工具栏 -->
    <div class="graph-toolbar">
      <span class="graph-legend">
        <span class="legend-dot" style="background: #52c41a" /> 调用者
        <span class="legend-dot" style="background: #1890ff" /> 当前函数
        <span class="legend-dot" style="background: #fa8c16" /> 被调用
      </span>
      <span class="graph-hint">滚轮缩放 · 拖拽平移 · 单击跳转 · 双击展开</span>
      <a-button size="mini" type="text" @click="resetView">重置视图</a-button>
    </div>

    <svg
      ref="svgRef"
      class="graph-svg"
      :viewBox="viewBox"
      preserveAspectRatio="xMidYMid meet"
      :style="{
        transform: `translate(${pan.x}px, ${pan.y}px) scale(${scale})`,
        cursor: dragging ? 'grabbing' : 'grab',
      }"
    >
      <defs>
        <marker
          id="arrowhead"
          markerWidth="8"
          markerHeight="6"
          refX="8"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#666" />
        </marker>
      </defs>

      <!-- 边 -->
      <g class="edges">
        <path
          v-for="(edge, idx) in edges"
          :key="idx"
          :d="edgePath(edge.points)"
          fill="none"
          stroke="#555"
          stroke-width="1.5"
          marker-end="url(#arrowhead)"
          opacity="0.6"
        />
      </g>

      <!-- 节点 -->
      <g
        v-for="node in nodes"
        :key="node.id"
        class="graph-node"
        :transform="`translate(${node.x - node.width / 2}, ${node.y - node.height / 2})`"
        @click.stop="handleNodeClick(node)"
        @dblclick.stop="handleNodeDblClick(node)"
        @mouseenter="hoveredNode = node.id"
        @mouseleave="hoveredNode = null"
        style="cursor: pointer"
      >
        <rect
          :width="node.width"
          :height="node.height"
          :rx="6"
          :ry="6"
          :fill="nodeColor(node.kind)"
          :stroke="hoveredNode === node.id ? '#fff' : nodeBorderColor(node.kind)"
          :stroke-width="hoveredNode === node.id ? 2.5 : 1.5"
          :opacity="hoveredNode === node.id ? 1 : 0.9"
        />
        <!-- 函数名 -->
        <text
          :x="node.width / 2"
          :y="18"
          text-anchor="middle"
          fill="#fff"
          font-size="12"
          font-weight="600"
          font-family="Menlo, Monaco, monospace"
        >
          {{ node.sym.name.length > 28 ? node.sym.name.slice(0, 26) + "…" : node.sym.name }}
        </text>
        <!-- 文件名:行号 -->
        <text
          :x="node.width / 2"
          :y="36"
          text-anchor="middle"
          fill="rgba(255,255,255,0.75)"
          font-size="10"
          font-family="Menlo, Monaco, monospace"
        >
          {{ shortFile(node.sym.file_path) }}:{{ node.sym.line }}
        </text>
      </g>

      <!-- 空状态 -->
      <text
        v-if="nodes.length === 0"
        x="50%"
        y="50%"
        text-anchor="middle"
        fill="#999"
        font-size="14"
      >
        暂无调用关系数据
      </text>
    </svg>
  </div>
</template>

<style scoped>
.call-graph-view {
  width: 100%;
  height: 100%;
  background: var(--color-bg-1);
  position: relative;
  overflow: hidden;
  user-select: none;
}

.graph-toolbar {
  position: absolute;
  top: 8px;
  left: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  z-index: 10;
  font-size: 11px;
  color: var(--color-text-3);
  pointer-events: none;
}

.graph-toolbar > * {
  pointer-events: auto;
}

.graph-legend {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--color-bg-2);
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--color-border);
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.graph-hint {
  background: var(--color-bg-2);
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--color-border);
}

.graph-svg {
  width: 100%;
  height: 100%;
  transform-origin: center center;
}

.graph-node:hover rect {
  filter: brightness(1.15);
}
</style>
