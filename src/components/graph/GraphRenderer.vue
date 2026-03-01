<script setup lang="ts">
/**
 * 图渲染器 — 基于 PixiJS v8 WebGL 的调用图 / 文件依赖图可视化
 *
 * # 设计说明
 * 使用 dagre 进行层次化布局计算，PixiJS v8 Application 进行 WebGL 渲染。
 * 支持滚轮缩放、拖拽平移、节点点击选中。
 * 节点按 kind 着色：Function=#4080ff, File=#52c41a, Type=#fa8c16, Module=#722ed1
 */
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import { useGraphStore } from "@/stores/graph";
import { Application, Graphics, Text, TextStyle, Container } from "pixi.js";
import type { FederatedPointerEvent } from "pixi.js";
import dagre from "@dagrejs/dagre";
import type { GraphNode, GraphEdge } from "@/api/types";

const { t } = useI18n();
const graphStore = useGraphStore();

const props = defineProps<{
  graphData: { nodes: GraphNode[]; edges: GraphEdge[] } | null;
}>();

const emit = defineEmits<{
  (e: "node-click", node: GraphNode): void;
}>();

const containerRef = ref<HTMLDivElement | null>(null);

const NODE_WIDTH = 160;
const NODE_HEIGHT = 40;
const NODE_RADIUS = 8;
const ARROW_SIZE = 8;
const MIN_SCALE = 0.1;
const MAX_SCALE = 3;

const kindColorMap: Record<string, number> = {
  Function: 0x4080ff,
  File: 0x52c41a,
  Type: 0xfa8c16,
  Module: 0x722ed1,
};

function getNodeColor(kind: string): number {
  return kindColorMap[kind] ?? 0x999999;
}

let app: Application | null = null;
let worldContainer: Container | null = null;
let isPanning = false;
let panStart = { x: 0, y: 0 };

interface LayoutNode {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  node: GraphNode;
}

interface LayoutEdge {
  points: Array<{ x: number; y: number }>;
  edge: GraphEdge;
}

const nodeCount = computed(() => props.graphData?.nodes.length ?? 0);
const edgeCount = computed(() => props.graphData?.edges.length ?? 0);
const hasData = computed(() => props.graphData && props.graphData.nodes.length > 0);

function computeLayout(): { nodes: LayoutNode[]; edges: LayoutEdge[] } {
  if (!props.graphData) return { nodes: [], edges: [] };

  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", ranksep: 60, nodesep: 40, marginx: 40, marginy: 40 });
  g.setDefaultEdgeLabel(() => ({}));

  for (const node of props.graphData.nodes) {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }
  for (const edge of props.graphData.edges) {
    g.setEdge(edge.source, edge.target);
  }

  dagre.layout(g);

  const layoutNodes: LayoutNode[] = [];
  for (const node of props.graphData.nodes) {
    const dagreNode = g.node(node.id);
    if (dagreNode) {
      layoutNodes.push({
        id: node.id,
        x: dagreNode.x - NODE_WIDTH / 2,
        y: dagreNode.y - NODE_HEIGHT / 2,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        node,
      });
    }
  }

  const layoutEdges: LayoutEdge[] = [];
  for (const edge of props.graphData.edges) {
    const dagreEdge = g.edge(edge.source, edge.target);
    if (dagreEdge?.points) {
      layoutEdges.push({
        points: dagreEdge.points.map((p: { x: number; y: number }) => ({ x: p.x, y: p.y })),
        edge,
      });
    }
  }

  return { nodes: layoutNodes, edges: layoutEdges };
}

function drawRoundedRect(
  gfx: Graphics,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
  fillColor: number,
  isSelected: boolean,
) {
  if (isSelected) {
    gfx.roundRect(x - 2, y - 2, w + 4, h + 4, r + 2);
    gfx.fill({ color: 0xffffff, alpha: 0.9 });
    gfx.stroke({ color: fillColor, width: 3 });
  }
  gfx.roundRect(x, y, w, h, r);
  gfx.fill({ color: fillColor, alpha: 0.85 });
}

function drawArrow(gfx: Graphics, toX: number, toY: number, fromX: number, fromY: number) {
  const angle = Math.atan2(toY - fromY, toX - fromX);
  const x1 = toX - ARROW_SIZE * Math.cos(angle - Math.PI / 6);
  const y1 = toY - ARROW_SIZE * Math.sin(angle - Math.PI / 6);
  const x2 = toX - ARROW_SIZE * Math.cos(angle + Math.PI / 6);
  const y2 = toY - ARROW_SIZE * Math.sin(angle + Math.PI / 6);

  gfx.moveTo(toX, toY);
  gfx.lineTo(x1, y1);
  gfx.moveTo(toX, toY);
  gfx.lineTo(x2, y2);
  gfx.stroke({ color: 0x999999, width: 1.5 });
}

function renderGraph() {
  if (!app || !worldContainer) return;

  worldContainer.removeChildren();

  if (!hasData.value) return;

  const { nodes, edges } = computeLayout();

  const edgeGfx = new Graphics();
  for (const layoutEdge of edges) {
    const pts = layoutEdge.points;
    if (pts.length < 2) continue;

    edgeGfx.moveTo(pts[0].x, pts[0].y);
    for (let i = 1; i < pts.length; i++) {
      edgeGfx.lineTo(pts[i].x, pts[i].y);
    }
    edgeGfx.stroke({ color: 0x999999, width: 1.2, alpha: 0.6 });

    const last = pts[pts.length - 1];
    const prev = pts[pts.length - 2];
    drawArrow(edgeGfx, last.x, last.y, prev.x, prev.y);
  }
  worldContainer.addChild(edgeGfx);

  const nodeIdToGfx = new Map<string, Container>();
  for (const ln of nodes) {
    const nodeContainer = new Container();
    nodeContainer.position.set(ln.x, ln.y);
    nodeContainer.eventMode = "static";
    nodeContainer.cursor = "pointer";

    const color = getNodeColor(ln.node.kind);
    const isSelected = graphStore.selectedNodeId === ln.id;

    const bg = new Graphics();
    drawRoundedRect(bg, 0, 0, ln.width, ln.height, NODE_RADIUS, color, isSelected);
    nodeContainer.addChild(bg);

    const labelStyle = new TextStyle({
      fontSize: 12,
      fill: 0xffffff,
      fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      wordWrap: false,
    });
    const label = new Text({ text: truncateLabel(ln.node.label, 18), style: labelStyle });
    label.anchor.set(0.5, 0.5);
    label.position.set(ln.width / 2, ln.height / 2);
    nodeContainer.addChild(label);

    const graphNode = ln.node;
    nodeContainer.on("pointerdown", (ev: FederatedPointerEvent) => {
      ev.stopPropagation();
      graphStore.selectNode(graphNode.id);
      emit("node-click", graphNode);
      renderGraph();
    });

    worldContainer.addChild(nodeContainer);
    nodeIdToGfx.set(ln.id, nodeContainer);
  }
}

function truncateLabel(label: string, maxLen: number): string {
  return label.length > maxLen ? label.slice(0, maxLen - 1) + "…" : label;
}

function handleWheel(ev: WheelEvent) {
  if (!worldContainer) return;
  ev.preventDefault();

  const scaleFactor = ev.deltaY > 0 ? 0.9 : 1.1;
  const newScale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, worldContainer.scale.x * scaleFactor));

  const rect = containerRef.value?.getBoundingClientRect();
  if (!rect) return;

  const mouseX = ev.clientX - rect.left;
  const mouseY = ev.clientY - rect.top;

  const worldX = (mouseX - worldContainer.x) / worldContainer.scale.x;
  const worldY = (mouseY - worldContainer.y) / worldContainer.scale.y;

  worldContainer.scale.set(newScale, newScale);
  worldContainer.x = mouseX - worldX * newScale;
  worldContainer.y = mouseY - worldY * newScale;
}

function handlePointerDown(ev: FederatedPointerEvent) {
  isPanning = true;
  panStart = { x: ev.globalX, y: ev.globalY };
}

function handlePointerMove(ev: FederatedPointerEvent) {
  if (!isPanning || !worldContainer) return;
  const dx = ev.globalX - panStart.x;
  const dy = ev.globalY - panStart.y;
  worldContainer.x += dx;
  worldContainer.y += dy;
  panStart = { x: ev.globalX, y: ev.globalY };
}

function handlePointerUp() {
  isPanning = false;
}

async function initPixi() {
  if (!containerRef.value) return;

  const container = containerRef.value;
  const { clientWidth: width, clientHeight: height } = container;

  app = new Application();
  await app.init({
    width: width || 800,
    height: height || 600,
    backgroundColor: 0x1e1e2e,
    antialias: true,
    resolution: window.devicePixelRatio || 1,
    autoDensity: true,
  });

  container.appendChild(app.canvas);
  app.canvas.style.width = "100%";
  app.canvas.style.height = "100%";

  worldContainer = new Container();
  app.stage.addChild(worldContainer);

  app.stage.eventMode = "static";
  app.stage.hitArea = app.screen;
  app.stage.on("pointerdown", handlePointerDown);
  app.stage.on("pointermove", handlePointerMove);
  app.stage.on("pointerup", handlePointerUp);
  app.stage.on("pointerupoutside", handlePointerUp);

  container.addEventListener("wheel", handleWheel, { passive: false });

  renderGraph();
}

function destroyPixi() {
  if (containerRef.value) {
    containerRef.value.removeEventListener("wheel", handleWheel);
  }
  if (app) {
    app.destroy(true, { children: true });
    app = null;
    worldContainer = null;
  }
}

function resizeRenderer() {
  if (!app || !containerRef.value) return;
  const { clientWidth: w, clientHeight: h } = containerRef.value;
  if (w > 0 && h > 0) {
    app.renderer.resize(w, h);
    app.stage.hitArea = app.screen;
  }
}

let resizeObserver: ResizeObserver | null = null;

onMounted(async () => {
  await nextTick();
  if (hasData.value) {
    await initPixi();
  }

  if (containerRef.value) {
    resizeObserver = new ResizeObserver(() => resizeRenderer());
    resizeObserver.observe(containerRef.value);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  destroyPixi();
});

watch(
  () => props.graphData,
  async () => {
    if (hasData.value) {
      if (!app) {
        await nextTick();
        await initPixi();
      } else {
        renderGraph();
      }
    } else {
      destroyPixi();
    }
  },
  { deep: true },
);

watch(
  () => graphStore.selectedNodeId,
  () => {
    if (app && worldContainer) {
      renderGraph();
    }
  },
);
</script>

<template>
  <div class="graph-renderer">
    <div class="graph-toolbar">
      <a-space>
        <a-tag>{{ t("graph.title") }}</a-tag>
        <a-tag color="arcoblue">
          {{ nodeCount }} {{ t("parse.symbols") }}
        </a-tag>
        <a-tag color="green">
          {{ edgeCount }} {{ t("parse.relations") }}
        </a-tag>
      </a-space>
    </div>

    <div v-if="!hasData" class="empty-state">
      <a-empty :description="t('search.noResults')" />
    </div>
    <div v-else ref="containerRef" class="graph-canvas-container" />
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

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 48px;
}

.graph-canvas-container {
  flex: 1;
  overflow: hidden;
  position: relative;
}
</style>
