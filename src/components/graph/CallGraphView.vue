<script setup lang="ts">
/**
 * 可视化调用图 — 基于 @antv/x6 框架 + dagre 横向布局
 *
 * 中心节点为当前选中的函数，左侧为调用者（callers），右侧为被调用者（callees）。
 * 边使用 manhattan 路由自动避障 + 圆弧转角，连接点由框架根据 boundary 策略自动选择。
 *
 * 交互：
 *   左键框选/单击选中       Ctrl/Meta+点击 — 多选
 *   右键拖拽空白 — 平移      右键节点 — 查询菜单
 *   双击节点 — 跳转代码      Delete/Backspace — 删除选中
 *   拖拽节点 — 移动（对齐辅助线）
 *   滚轮 — 缩放              Ctrl+Z/Y — 撤销/重做查询
 *   右下角操作按钮
 */
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from "vue";
import { Graph } from "@antv/x6";
import { Snapline } from "@antv/x6-plugin-snapline";
import { Selection } from "@antv/x6-plugin-selection";
import dagre from "@dagrejs/dagre";
import type { Symbol as CctSymbol } from "@/api/types";
import MdiUndoVariant from "~icons/mdi/undo-variant";
import MdiRedoVariant from "~icons/mdi/redo-variant";
import MdiFitToScreenOutline from "~icons/mdi/fit-to-screen-outline";

interface GraphEdgeData {
  sourceId: number;
  targetId: number;
}

const props = defineProps<{
  rootSymbol: CctSymbol;
  callers: CctSymbol[];
  callees: CctSymbol[];
  extraEdges?: GraphEdgeData[];
  canUndo?: boolean;
  canRedo?: boolean;
}>();

const emit = defineEmits<{
  (e: "navigate", sym: CctSymbol): void;
  (e: "query-callers", sym: CctSymbol): void;
  (e: "query-callees", sym: CctSymbol): void;
  (e: "undo"): void;
  (e: "redo"): void;
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
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return;

  if (e.key === "Delete" || e.key === "Backspace") {
    e.preventDefault();
    deleteSelected();
    return;
  }

  const mod = e.ctrlKey || e.metaKey;
  if (mod && e.key === "z" && !e.shiftKey) {
    e.preventDefault();
    emit("undo");
  } else if (mod && (e.key === "Z" || (e.key === "z" && e.shiftKey))) {
    e.preventDefault();
    emit("redo");
  } else if (mod && e.key === "y") {
    e.preventDefault();
    emit("redo");
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
      eventTypes: ["rightMouseDown"],
    },
    mousewheel: {
      enabled: true,
      zoomAtMousePosition: true,
      minScale: 0.1,
      maxScale: 10,
    },
    interacting: { nodeMovable: true },
    connecting: {
      anchor: "center",
      connectionPoint: "boundary",
      router: { name: "manhattan", args: { padding: 20 } },
      connector: { name: "rounded", args: { radius: 8 } },
    },
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
          refX: 0.5,
          refY: 0.35,
          fill: "#fff",
          fontSize: 12,
          fontWeight: "600",
          fontFamily: "Menlo, Monaco, monospace",
          textAnchor: "middle",
          textVerticalAnchor: "middle",
        },
        sublabel: {
          text: `${shortFile(sym.file_path)}:${sym.line}`,
          refX: 0.5,
          refY: 0.72,
          fill: "rgba(255,255,255,0.75)",
          fontSize: 10,
          fontFamily: "Menlo, Monaco, monospace",
          textAnchor: "middle",
          textVerticalAnchor: "middle",
        },
      },
    });
  }

  for (const e of g.edges()) {
    graph.addEdge({
      source: e.v,
      target: e.w,
      router: { name: "manhattan", args: { padding: 20 } },
      connector: { name: "rounded", args: { radius: 8 } },
      attrs: {
        line: {
          stroke: "#a0a0a0",
          strokeWidth: 1.5,
          targetMarker: {
            name: "block",
            width: 8,
            height: 6,
            fill: "#a0a0a0",
          },
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
    <div ref="containerRef" class="graph-container" />

    <!-- 右下角操作按钮 -->
    <div class="graph-actions">
      <a-tooltip content="撤销查询 (Ctrl+Z)" position="left" mini>
        <a-button
          size="small"
          shape="circle"
          :disabled="!canUndo"
          @click.stop="emit('undo')"
        >
          <MdiUndoVariant />
        </a-button>
      </a-tooltip>
      <a-tooltip content="重做查询 (Ctrl+Y)" position="left" mini>
        <a-button
          size="small"
          shape="circle"
          :disabled="!canRedo"
          @click.stop="emit('redo')"
        >
          <MdiRedoVariant />
        </a-button>
      </a-tooltip>
      <a-tooltip content="重置视图" position="left" mini>
        <a-button size="small" shape="circle" @click.stop="resetView">
          <MdiFitToScreenOutline />
        </a-button>
      </a-tooltip>
    </div>

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

.graph-container {
  width: 100%;
  height: 100%;
}

.graph-actions {
  position: absolute;
  right: 12px;
  bottom: 12px;
  z-index: 10;
  display: flex;
  flex-direction: column;
  gap: 6px;
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
