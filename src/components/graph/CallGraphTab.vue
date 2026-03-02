<script setup lang="ts">
/**
 * 调用图 Tab — 在编辑器窗格中显示调用关系图
 *
 * 接收 graphData prop（包含 symbol、callers、callees、extraEdges），
 * 内部管理增量查询（扩展调用者/被调用者），通过 CallGraphView 渲染。
 */
import { ref, computed, watch } from "vue";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { Message } from "@arco-design/web-vue";
import CallGraphView from "./CallGraphView.vue";
import * as queryApi from "@/api/query";
import type { Symbol as CctSymbol, CallGraphData, GraphEdgeData } from "@/api/types";

const props = defineProps<{
  tabId: string;
  graphData: CallGraphData;
}>();

const projectStore = useProjectStore();
const editorStore = useEditorStore();

const graphSymMap = new Map<number, CctSymbol>();
const graphLoading = ref(false);

const rootSymbol = computed(() => props.graphData.symbol);
const callers = computed(() => props.graphData.callers);
const callees = computed(() => props.graphData.callees);
const extraEdges = computed(() => props.graphData.extraEdges);

watch(
  () => props.graphData.symbol.id,
  () => {
    graphSymMap.clear();
    graphSymMap.set(props.graphData.symbol.id, props.graphData.symbol);
    for (const s of props.graphData.callers) graphSymMap.set(s.id, s);
    for (const s of props.graphData.callees) graphSymMap.set(s.id, s);
  },
  { immediate: true },
);

function deduplicateSymbols(syms: CctSymbol[]): CctSymbol[] {
  const seen = new Map<string, CctSymbol>();
  for (const s of syms) {
    const key = s.qualified_name;
    const existing = seen.get(key);
    if (!existing || (s.is_definition && !existing.is_definition)) {
      seen.set(key, s);
    }
  }
  return Array.from(seen.values());
}

async function handleQueryNodeCallers(sym: CctSymbol) {
  const projectId = projectStore.currentProjectId;
  if (!projectId) return;

  try {
    const rels = await queryApi.queryCallers(projectId, sym.id, 1);

    const missingIds = new Set<number>();
    for (const r of rels) {
      if (!graphSymMap.has(r.caller_id)) missingIds.add(r.caller_id);
      if (!graphSymMap.has(r.callee_id)) missingIds.add(r.callee_id);
    }
    if (missingIds.size > 0) {
      const fetched = await queryApi.getSymbolsByIds(projectId, Array.from(missingIds));
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    const existingNames = new Set([
      props.graphData.symbol.qualified_name,
      ...props.graphData.callers.map((s) => s.qualified_name),
      ...props.graphData.callees.map((s) => s.qualified_name),
    ]);

    const newEdges: GraphEdgeData[] = [];
    const newCallers: CctSymbol[] = [];

    for (const r of rels) {
      const callerSym = graphSymMap.get(r.caller_id);
      if (!callerSym) continue;
      newEdges.push({ sourceId: r.caller_id, targetId: r.callee_id });
      if (!existingNames.has(callerSym.qualified_name)) {
        newCallers.push(callerSym);
        existingNames.add(callerSym.qualified_name);
      }
    }

    if (newCallers.length > 0 || newEdges.length > 0) {
      const updated: CallGraphData = {
        symbol: props.graphData.symbol,
        callers: newCallers.length > 0
          ? [...props.graphData.callers, ...newCallers]
          : props.graphData.callers,
        callees: props.graphData.callees,
        extraEdges: newEdges.length > 0
          ? [...props.graphData.extraEdges, ...newEdges]
          : props.graphData.extraEdges,
      };
      editorStore.updateCallGraphData(props.tabId, updated);
    } else {
      Message.info("未发现更多调用者");
    }
  } catch {
    Message.error("查询调用者失败");
  }
}

async function handleQueryNodeCallees(sym: CctSymbol) {
  const projectId = projectStore.currentProjectId;
  if (!projectId) return;

  try {
    const rels = await queryApi.queryCallees(projectId, sym.id, 1);

    const missingIds = new Set<number>();
    for (const r of rels) {
      if (!graphSymMap.has(r.caller_id)) missingIds.add(r.caller_id);
      if (!graphSymMap.has(r.callee_id)) missingIds.add(r.callee_id);
    }
    if (missingIds.size > 0) {
      const fetched = await queryApi.getSymbolsByIds(projectId, Array.from(missingIds));
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    const existingNames = new Set([
      props.graphData.symbol.qualified_name,
      ...props.graphData.callers.map((s) => s.qualified_name),
      ...props.graphData.callees.map((s) => s.qualified_name),
    ]);

    const newEdges: GraphEdgeData[] = [];
    const newCallees: CctSymbol[] = [];

    for (const r of rels) {
      const calleeSym = graphSymMap.get(r.callee_id);
      if (!calleeSym) continue;
      newEdges.push({ sourceId: r.caller_id, targetId: r.callee_id });
      if (!existingNames.has(calleeSym.qualified_name)) {
        newCallees.push(calleeSym);
        existingNames.add(calleeSym.qualified_name);
      }
    }

    if (newCallees.length > 0 || newEdges.length > 0) {
      const updated: CallGraphData = {
        symbol: props.graphData.symbol,
        callers: props.graphData.callers,
        callees: newCallees.length > 0
          ? [...props.graphData.callees, ...newCallees]
          : props.graphData.callees,
        extraEdges: newEdges.length > 0
          ? [...props.graphData.extraEdges, ...newEdges]
          : props.graphData.extraEdges,
      };
      editorStore.updateCallGraphData(props.tabId, updated);
    } else {
      Message.info("未发现更多被调用者");
    }
  } catch {
    Message.error("查询被调用者失败");
  }
}

function navigateToSymbol(sym: CctSymbol) {
  const projectId = projectStore.currentProjectId ?? undefined;
  editorStore.openFile(sym.file_path, projectId, sym.line ?? undefined);
}
</script>

<template>
  <div class="call-graph-tab">
    <a-spin :loading="graphLoading" style="width: 100%; height: 100%">
      <CallGraphView
        :root-symbol="rootSymbol"
        :callers="callers"
        :callees="callees"
        :extra-edges="extraEdges"
        @navigate="navigateToSymbol"
        @query-callers="handleQueryNodeCallers"
        @query-callees="handleQueryNodeCallees"
      />
    </a-spin>
  </div>
</template>

<style scoped>
.call-graph-tab {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
