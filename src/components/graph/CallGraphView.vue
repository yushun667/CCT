<script setup lang="ts">
/**
 * 可视化调用图 — 基于 @antv/x6 框架 + dagre 横向布局
 *
 * 中心节点为当前选中的函数，左侧为调用者（callers），右侧为被调用者（callees）。
 * 每个节点带有上下左右四个固定连接桩，边使用正交路由 + 圆弧转角。
 *
 * 交互：
 *   单击节点 — 选中       Ctrl/Meta+点击 — 多选切换
 *   框选拖拽 — 批量选中    双击节点 — 跳转代码
 *   右键节点 — 查询菜单    Delete/Backspace — 删除选中
 *   拖拽节点 — 移动位置（带对齐辅助线）
 *   滚轮 — 缩放            Shift+拖拽空白 — 平移画布
 */
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from "vue";
import { Graph } from "@antv/x6";
import { Snapline } from "@antv/x6-plugin-snapline";
import { Selection } from "@antv/x6-plugin-selection";
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

const NODE_W = 280;
const NODE_H = 48;

const containerRef = ref<HTMLDivElement | null>(null);
let graph: Graph | null = null;

const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  sym: null as CctSymbol | null,
});

/* ---------- 连接桩配置 ---------- */

const portAttrs = {
  circle: {
    r: 3,
    fill: "rgba(255,255,255,0.9)",
    stroke: "rgba(0,0,0,0.25)",
    strokeWidth: 1,
    magnet: false,
  },
};

const portGroups: Record<string, object> = {
  left: { position: "left", attrs: portAttrs },
  right: { position: "right", attrs: portAttrs },
  top: { position: "top", attrs: portAttrs },
  bottom: { position: "bottom", attrs: portAttrs },
};

const portItems = [
  { group: "left", id: "port-left" },
  { group: "right", id: "port-right" },
  { group: "top", id: "port-top" },
  { group: "bottom", id: "port-bottom" },
];

/* ---------- 辅助函数 ---------- */

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

/**
 * 从 qualified_name 提取简短的限定名用于节点显示。
 * 例如 "clang::CodeGen::CodeGenTypes::getLLVMContext" → "CodeGenTypes::getLLVMContext"
 * 保留最后两级以区分同名但不同类的方法。
 */
function shortQualifiedName(sym: CctSymbol): string {
  const parts = sym.qualified_name.split("::");
  if (parts.length <= 2) return sym.qualified_name;
  return parts.slice(-2).join("::");
}

function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max - 2) + "\u2026" : text;
}

/* ---------- 上下文菜单 ---------- */

function closeContextMenu() {
  contextMenu.value.visible = false;
  contextMenu.value.sym = null;
}

function onCtxQueryCallers() {
  if (contextMenu.value.sym) emit("query-callers", contextMenu.value.sym);
  closeContextMenu();
}

function onCtxQueryCallees() {
  if (contextMenu.value.sym) emit("query-callees", contextMenu.value.sym);
  closeContextMenu();
}

function onCtxNavigate() {
  if (contextMenu.value.sym) emit("navigate", contextMenu.value.sym);
  closeContextMenu();
}

/* ---------- 删除选中节点及其关联边 ---------- */

function deleteSelected() {
  if (!graph) return;
  const selected = graph.getSelectedCells();
  if (selected.length === 0) return;

  const nodeIds = new Set(
    selected.filter((c) => c.isNode()).map((c) => c.id),
  );

  const toRemove = [...selected];
  for (const edge of graph.getEdges()) {
    const src =
      typeof edge.getSourceCellId === "function"
        ? edge.getSourceCellId()
        : (edge.getSource() as { cell?: string })?.cell;
    const tgt =
      typeof edge.getTargetCellId === "function"
        ? edge.getTargetCellId()
        : (edge.getTarget() as { cell?: string })?.cell;
    if ((src && nodeIds.has(src)) || (tgt && nodeIds.has(tgt))) {
      if (!toRemove.includes(edge)) toRemove.push(edge);
    }
  }

  graph.removeCells(toRemove);
}

function onCtxDelete() {
  deleteSelected();
  closeContextMenu();
}

/* ---------- 键盘事件 ---------- */

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === "Delete" || e.key === "Backspace") {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    e.preventDefault();
    deleteSelected();
  }
}

/* ---------- 初始化 X6 图实例 ---------- */

function initGraph() {
  if (!containerRef.value) return;

  graph = new Graph({
    container: containerRef.value,
    autoResize: true,
    panning: {
      enabled: true,
      modifiers: "shift",
    },
    mousewheel: {
      enabled: true,
      zoomAtMousePosition: true,
      minScale: 0.1,
      maxScale: 10,
    },
    interacting: { nodeMovable: true },
  });

  graph.use(
    new Snapline({
      enabled: true,
      tolerance: 10,
    }),
  );

  graph.use(
    new Selection({
      enabled: true,
      rubberband: true,
      rubberNode: true,
      rubberEdge: false,
      multiple: true,
      multipleSelectionModifiers: ["ctrl", "meta"],
      movable: true,
      strict: false,
      showNodeSelectionBox: true,
    }),
  );

  graph.on("node:dblclick", ({ node }) => {
    const sym = node.getData()?.sym as CctSymbol | undefined;
    if (sym) emit("navigate", sym);
  });

  graph.on("node:contextmenu", ({ e, node }) => {
    e.preventDefault();
    graph!.select(node);
    contextMenu.value = {
      visible: true,
      x: e.clientX,
      y: e.clientY,
      sym: (node.getData()?.sym as CctSymbol) ?? null,
    };
  });

  graph.on("blank:click", () => {
    closeContextMenu();
  });

  graph.on("blank:contextmenu", ({ e }) => {
    e.preventDefault();
    closeContextMenu();
  });

  document.addEventListener("keydown", handleKeyDown);
}

/* ---------- 使用 dagre 布局并渲染到 X6 ---------- */

function buildGraph() {
  if (!graph) return;

  const g = new dagre.graphlib.Graph();
  g.setGraph({
    rankdir: "LR",
    nodesep: 30,
    ranksep: 80,
    marginx: 20,
    marginy: 20,
  });
  g.setDefaultEdgeLabel(() => ({}));

  const allSymbols = new Map<number, CctSymbol>();
  allSymbols.set(props.rootSymbol.id, props.rootSymbol);
  for (const s of props.callers) allSymbols.set(s.id, s);
  for (const s of props.callees) allSymbols.set(s.id, s);

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

  if (props.extraEdges) {
    for (const edge of props.extraEdges) {
      addEdge(edge.sourceId, edge.targetId);
    }
  }

  dagre.layout(g);

  graph.clearCells();

  const callerIds = new Set(props.callers.map((s) => s.id));

  for (const [id, sym] of allSymbols) {
    const nd = g.node(nodeKey(id));
    if (!nd) continue;

    let kind: "caller" | "root" | "callee" = "callee";
    if (id === props.rootSymbol.id) kind = "root";
    else if (callerIds.has(id)) kind = "caller";

    graph.addNode({
      id: nodeKey(id),
      x: nd.x - NODE_W / 2,
      y: nd.y - NODE_H / 2,
      width: NODE_W,
      height: NODE_H,
      data: { sym, kind },
      markup: [
        { tagName: "rect", selector: "body" },
        { tagName: "text", selector: "label" },
        { tagName: "text", selector: "sublabel" },
      ],
      attrs: {
        body: {
          width: NODE_W,
          height: NODE_H,
          rx: 6,
          ry: 6,
          fill: nodeColor(kind),
          stroke: nodeBorderColor(kind),
          strokeWidth: 1.5,
          opacity: 0.9,
          cursor: "pointer",
        },
        label: {
          text: truncate(shortQualifiedName(sym), 32),
          x: NODE_W / 2,
          y: 18,
          fill: "#fff",
          fontSize: 12,
          fontWeight: "600",
          fontFamily: "Menlo, Monaco, monospace",
          textAnchor: "middle",
        },
        sublabel: {
          text: `${shortFile(sym.file_path)}:${sym.line}`,
          x: NODE_W / 2,
          y: 36,
          fill: "rgba(255,255,255,0.75)",
          fontSize: 10,
          fontFamily: "Menlo, Monaco, monospace",
          textAnchor: "middle",
        },
      },
      ports: { groups: portGroups, items: portItems },
    });
  }

  for (const e of g.edges()) {
    graph.addEdge({
      source: { cell: e.v, port: "port-right" },
      target: { cell: e.w, port: "port-left" },
      router: { name: "orth" },
      connector: { name: "rounded", args: { radius: 8 } },
      attrs: {
        line: {
          stroke: "#555",
          strokeWidth: 1.5,
          opacity: 0.6,
          targetMarker: { name: "block", width: 8, height: 6 },
        },
      },
    });
  }

  graph.centerContent();
}

function resetView() {
  if (!graph) return;
  graph.zoomTo(1);
  graph.centerContent();
}

onMounted(() => {
  initGraph();
  nextTick(() => buildGraph());
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", handleKeyDown);
  graph?.dispose();
  graph = null;
});

watch(
  () => [props.rootSymbol, props.callers, props.callees, props.extraEdges],
  () => {
    nextTick(() => buildGraph());
  },
  { deep: true },
);
</script>

<template>
  <div class="call-graph-view" @contextmenu.prevent>
    <div class="graph-toolbar">
      <span class="graph-legend">
        <span class="legend-dot" style="background: #52c41a" /> 调用者
        <span class="legend-dot" style="background: #1890ff" /> 当前函数
        <span class="legend-dot" style="background: #fa8c16" /> 被调用
      </span>
      <span class="graph-hint">
        框选/点击选中 · 双击跳转 · 右键菜单 · 滚轮缩放 · Shift+拖拽平移 ·
        Delete 删除
      </span>
      <a-button size="mini" type="text" @click.stop="resetView">
        重置视图
      </a-button>
    </div>

    <div ref="containerRef" class="graph-container" />

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
        <div class="ctx-divider" />
        <div class="ctx-item ctx-danger" @click="onCtxDelete">
          <icon-delete />
          删除选中节点
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

.graph-container {
  width: 100%;
  height: 100%;
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

.ctx-danger {
  color: #f53f3f;
}

.ctx-danger:hover {
  background: #fff1f0;
}

.ctx-divider {
  height: 1px;
  margin: 4px 0;
  background: var(--color-border, #e5e6eb);
}
</style>
