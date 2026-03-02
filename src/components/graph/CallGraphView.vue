<script setup lang="ts">
/**
 * 可视化调用图 — 使用 dagre 树状布局 + SVG 渲染
 *
 * 中心节点为当前选中的函数，上方为调用者（callers），下方为被调用者（callees）。
 * 每个节点显示函数名 + 文件名:行号。
 *
 * 交互：单击选中节点，双击跳转代码，右键菜单查询调用者/被调用者（1层，增量追加）。
 */
import { ref, computed, watch, nextTick } from "vue";
import dagre from "@dagrejs/dagre";
import type { Symbol as CctSymbol } from "@/api/types";

interface GraphEdgeData {
  sourceId: number;
  targetId: number;
}

const props = defineProps<{
  rootSymbol: CctSymbol;
  callers: CctSymbol[];
  callees: CctSymbol[];
  extraEdges?: GraphEdgeData[];
}>();

const emit = defineEmits<{
  (e: "navigate", sym: CctSymbol): void;
  (e: "query-callers", sym: CctSymbol): void;
  (e: "query-callees", sym: CctSymbol): void;
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
  if (nodes.value.length === 0)
    return `0 0 ${svgWidth.value} ${svgHeight.value}`;
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const n of nodes.value) {
    minX = Math.min(minX, n.x - n.width / 2);
    minY = Math.min(minY, n.y - n.height / 2);
    maxX = Math.max(maxX, n.x + n.width / 2);
    maxY = Math.max(maxY, n.y + n.height / 2);
  }
  return `${minX - pad} ${minY - pad} ${maxX - minX + pad * 2} ${maxY - minY + pad * 2}`;
});

const pan = ref({ x: 0, y: 0 });
const scale = ref(1);
const dragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const hoveredNode = ref<string | null>(null);
const selectedNode = ref<string | null>(null);
const svgRef = ref<SVGSVGElement | null>(null);

const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  node: null as LayoutNode | null,
});

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

  // 按 qualified_name 去重，避免声明+定义产生重复节点；优先保留定义
  const allSymbols = new Map<number, CctSymbol>();
  const nameToId = new Map<string, number>();

  function addSymbol(s: CctSymbol) {
    const existId = nameToId.get(s.qualified_name);
    if (existId != null) {
      const exist = allSymbols.get(existId);
      if (exist && !exist.is_definition && s.is_definition) {
        allSymbols.delete(existId);
        allSymbols.set(s.id, s);
        nameToId.set(s.qualified_name, s.id);
      }
      return;
    }
    allSymbols.set(s.id, s);
    nameToId.set(s.qualified_name, s.id);
  }

  addSymbol(props.rootSymbol);
  for (const s of props.callers) addSymbol(s);
  for (const s of props.callees) addSymbol(s);

  // 构建全量 qualified_name → 保留 ID 的映射，用于将边指向正确节点
  const allNameToCanonId = new Map<string, number>();
  for (const [id, s] of allSymbols) allNameToCanonId.set(s.qualified_name, id);

  // 将来源的 id 映射到去重后保留的 id（通过 graphSymMap 查 qualified_name）
  function canonId(rawId: number): number {
    // 先从当前 allSymbols 找
    if (allSymbols.has(rawId)) return rawId;
    // 否则从 props 中的完整符号列表找对应的 qualified_name
    const all = [props.rootSymbol, ...props.callers, ...props.callees];
    const sym = all.find((s) => s.id === rawId);
    if (sym) {
      const cid = allNameToCanonId.get(sym.qualified_name);
      if (cid != null) return cid;
    }
    return rawId;
  }

  const nodeKey = (id: number) => `n-${id}`;

  for (const [id] of allSymbols) {
    g.setNode(nodeKey(id), { width: NODE_W, height: NODE_H });
  }

  const edgeSet = new Set<string>();
  const addEdge = (from: number, to: number) => {
    const key = `${from}->${to}`;
    if (edgeSet.has(key)) return;
    edgeSet.add(key);
    if (allSymbols.has(from) && allSymbols.has(to)) {
      g.setEdge(nodeKey(from), nodeKey(to));
    }
  };

  const rootId = canonId(props.rootSymbol.id);
  for (const caller of props.callers) {
    addEdge(canonId(caller.id), rootId);
  }
  for (const callee of props.callees) {
    addEdge(rootId, canonId(callee.id));
  }
  if (props.extraEdges) {
    for (const edge of props.extraEdges) {
      addEdge(canonId(edge.sourceId), canonId(edge.targetId));
    }
  }

  dagre.layout(g);

  const layoutNodes: LayoutNode[] = [];
  const callerIds = new Set(props.callers.map((s) => s.id));
  const calleeIds = new Set(props.callees.map((s) => s.id));

  for (const [id, sym] of allSymbols) {
    const nd = g.node(nodeKey(id));
    if (!nd) continue;

    let kind: "caller" | "root" | "callee" = "callee";
    if (id === props.rootSymbol.id) kind = "root";
    else if (callerIds.has(id)) kind = "caller";

    layoutNodes.push({
      id: nodeKey(id),
      sym,
      x: nd.x,
      y: nd.y,
      width: NODE_W,
      height: NODE_H,
      isRoot: id === props.rootSymbol.id,
      kind,
    });
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
  for (let i = 1; i < pts.length; i++) {
    d += ` L ${pts[i].x} ${pts[i].y}`;
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

function isSelected(nodeId: string): boolean {
  return selectedNode.value === nodeId;
}

function handleNodeClick(node: LayoutNode, e: MouseEvent) {
  e.stopPropagation();
  selectedNode.value = node.id;
  closeContextMenu();
}

function handleNodeDblClick(node: LayoutNode) {
  emit("navigate", node.sym);
}

function handleNodeContextMenu(node: LayoutNode, e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
  selectedNode.value = node.id;
  contextMenu.value = {
    visible: true,
    x: e.clientX,
    y: e.clientY,
    node,
  };
}

function closeContextMenu() {
  contextMenu.value.visible = false;
  contextMenu.value.node = null;
}

function onCtxQueryCallers() {
  const node = contextMenu.value.node;
  if (node) emit("query-callers", node.sym);
  closeContextMenu();
}

function onCtxQueryCallees() {
  const node = contextMenu.value.node;
  if (node) emit("query-callees", node.sym);
  closeContextMenu();
}

function onCtxNavigate() {
  const node = contextMenu.value.node;
  if (node) emit("navigate", node.sym);
  closeContextMenu();
}

function handleBgClick() {
  selectedNode.value = null;
  closeContextMenu();
}

function handleWheel(e: WheelEvent) {
  e.preventDefault();
  const factor = e.deltaY > 0 ? 0.9 : 1.1;
  scale.value = Math.max(0.2, Math.min(3, scale.value * factor));
}

function handleMouseDown(e: MouseEvent) {
  if (e.button === 0) {
    dragging.value = true;
    dragStart.value = {
      x: e.clientX - pan.value.x,
      y: e.clientY - pan.value.y,
    };
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
  () => [props.rootSymbol, props.callers, props.callees, props.extraEdges],
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
    @click="handleBgClick"
    @contextmenu.prevent="closeContextMenu"
  >
    <div class="graph-toolbar">
      <span class="graph-legend">
        <span class="legend-dot" style="background: #52c41a" /> 调用者
        <span class="legend-dot" style="background: #1890ff" /> 当前函数
        <span class="legend-dot" style="background: #fa8c16" /> 被调用
      </span>
      <span class="graph-hint">
        单击选中 · 双击跳转 · 右键查询 · 滚轮缩放 · 拖拽平移
      </span>
      <a-button size="mini" type="text" @click.stop="resetView">
        重置视图
      </a-button>
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
        @click="handleNodeClick(node, $event)"
        @dblclick.stop="handleNodeDblClick(node)"
        @contextmenu="handleNodeContextMenu(node, $event)"
        @mouseenter="hoveredNode = node.id"
        @mouseleave="hoveredNode = null"
        style="cursor: pointer"
      >
        <!-- 选中高亮光圈 -->
        <rect
          v-if="isSelected(node.id)"
          :x="-3"
          :y="-3"
          :width="node.width + 6"
          :height="node.height + 6"
          :rx="8"
          :ry="8"
          fill="none"
          stroke="#fff"
          stroke-width="2"
          opacity="0.8"
        />
        <rect
          :width="node.width"
          :height="node.height"
          :rx="6"
          :ry="6"
          :fill="nodeColor(node.kind)"
          :stroke="
            isSelected(node.id)
              ? '#fff'
              : hoveredNode === node.id
                ? '#ddd'
                : nodeBorderColor(node.kind)
          "
          :stroke-width="isSelected(node.id) ? 2.5 : hoveredNode === node.id ? 2 : 1.5"
          :opacity="isSelected(node.id) || hoveredNode === node.id ? 1 : 0.9"
        />
        <text
          :x="node.width / 2"
          :y="18"
          text-anchor="middle"
          fill="#fff"
          font-size="12"
          font-weight="600"
          font-family="Menlo, Monaco, monospace"
        >
          {{
            node.sym.name.length > 28
              ? node.sym.name.slice(0, 26) + "\u2026"
              : node.sym.name
          }}
        </text>
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

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="graph-context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      >
        <div class="ctx-item" @click="onCtxQueryCallers">
          <icon-import style="color: #52c41a" />
          查询调用者
        </div>
        <div class="ctx-item" @click="onCtxQueryCallees">
          <icon-export style="color: #fa8c16" />
          查询被调用者
        </div>
        <div class="ctx-divider" />
        <div class="ctx-item" @click="onCtxNavigate">
          <icon-code />
          跳转到代码
        </div>
      </div>
    </Teleport>
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

<style>
.graph-context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 160px;
  background: var(--color-bg-2, #fff);
  border: 1px solid var(--color-border, #e5e6eb);
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  padding: 4px 0;
  font-size: 13px;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  cursor: pointer;
  color: var(--color-text-1, #1d2129);
  transition: background 0.15s;
}

.ctx-item:hover {
  background: var(--color-fill-2, #f2f3f5);
}

.ctx-divider {
  height: 1px;
  margin: 4px 0;
  background: var(--color-border, #e5e6eb);
}
</style>
