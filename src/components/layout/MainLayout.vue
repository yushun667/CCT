<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import { useWindowTitle } from "@/composables/useWindowTitle";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import Sidebar from "./Sidebar.vue";
import StatusBar from "./StatusBar.vue";
import AiPanel from "@/components/ai/AiPanel.vue";
import EditorTabs from "@/components/editor/EditorTabs.vue";
import CodeEditor from "@/components/editor/CodeEditor.vue";
import WelcomeScreen from "@/components/welcome/WelcomeScreen.vue";
import TerminalPanel from "@/components/terminal/TerminalPanel.vue";
import CallGraphView from "@/components/graph/CallGraphView.vue";
import SettingsDialog from "@/components/settings/SettingsDialog.vue";
import ProjectSettingsDialog from "@/components/project/ProjectSettingsDialog.vue";
import { Message } from "@arco-design/web-vue";
import { useI18n } from "vue-i18n";
import * as editorApi from "@/api/editor";
import * as queryApi from "@/api/query";
import type { Symbol as CctSymbol, Project } from "@/api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

const { t } = useI18n();
const settings = useSettingsStore();
const editorStore = useEditorStore();
const projectStore = useProjectStore();
useWindowTitle();

// ── 原 Toolbar 功能迁移 ─────────────────────────────────────
const showSettings = ref(false);
const showProjectSettings = ref(false);
const settingsProject = ref<Project | null>(null);

async function handleOpenDirectory() {
  const selected = await open({ directory: true, multiple: false });
  if (!selected) return;
  try {
    await projectStore.openLocalDirectory(selected as string);
    Message.success(t("project.openSuccess"));
  } catch {
    // handled in store
  }
}

function handleProjectSettings() {
  if (!projectStore.currentProject) return;
  settingsProject.value = projectStore.currentProject;
  showProjectSettings.value = true;
}

async function handleParse() {
  if (!projectStore.currentProject) return;
  await projectStore.startParse(projectStore.currentProject.id);
  Message.info(t("project.parseStarted"));
}

function handleSettingsSaved() {
  projectStore.fetchProjects();
}

const menuUnlisteners: UnlistenFn[] = [];

onMounted(async () => {
  menuUnlisteners.push(
    await listen("open_directory", () => handleOpenDirectory()),
    await listen("start_parse", () => handleParse()),
    await listen("project_settings", () => handleProjectSettings()),
    await listen("toggle_sidebar", () => settings.toggleSidebar()),
    await listen("toggle_terminal", () => settings.toggleBottomPanel()),
    await listen("toggle_ai", () => settings.toggleAiPanel()),
    await listen("app_settings", () => { showSettings.value = true; }),
  );
});

onUnmounted(() => {
  menuUnlisteners.forEach((fn) => fn());
});

interface GraphEdgeData {
  sourceId: number;
  targetId: number;
}

const graphVisible = ref(false);
const graphSymbol = ref<CctSymbol | null>(null);
const graphResults = ref<{ callers: CctSymbol[]; callees: CctSymbol[] }>({
  callers: [],
  callees: [],
});
const graphExtraEdges = ref<GraphEdgeData[]>([]);
const graphLoading = ref(false);

const graphSymMap = new Map<number, CctSymbol>();

async function findSymbolAtLine(line: number): Promise<CctSymbol | null> {
  const projectId = projectStore.currentProjectId;
  const file = editorStore.activeFile;
  if (!projectId || !file) return null;

  try {
    const symbols = await editorApi.getFileSymbols(projectId, file.filePath);
    if (symbols.length === 0) return null;

    // 优先精确匹配：光标行在符号定义范围内
    const exact = symbols.find(
      (s) => s.line <= line && (s.end_line ?? s.line) >= line,
    );
    if (exact) return exact;

    // 退而求其次：找最近且在光标之前定义的符号
    let best: CctSymbol | null = null;
    for (const s of symbols) {
      if (s.line <= line) {
        if (!best || s.line > best.line) best = s;
      }
    }
    return best;
  } catch {
    return null;
  }
}

async function loadCallGraph(sym: CctSymbol) {
  graphSymbol.value = sym;
  graphVisible.value = true;
  graphLoading.value = true;
  graphExtraEdges.value = [];
  graphSymMap.clear();

  const projectId = projectStore.currentProjectId!;
  try {
    const [callerRels, calleeRels] = await Promise.all([
      queryApi.queryCallers(projectId, sym.id, 5),
      queryApi.queryCallees(projectId, sym.id, 5),
    ]);

    graphSymMap.set(sym.id, sym);

    // 收集所有需要的符号 ID，按需精确查询
    const neededIds = new Set<number>();
    for (const r of callerRels) {
      neededIds.add(r.caller_id);
      neededIds.add(r.callee_id);
    }
    for (const r of calleeRels) {
      neededIds.add(r.caller_id);
      neededIds.add(r.callee_id);
    }
    neededIds.delete(sym.id);

    if (neededIds.size > 0) {
      const fetched = await queryApi.getSymbolsByIds(
        projectId,
        Array.from(neededIds),
      );
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    graphResults.value = {
      callers: callerRels
        .map((r) => graphSymMap.get(r.caller_id))
        .filter((s): s is CctSymbol => s != null),
      callees: calleeRels
        .map((r) => graphSymMap.get(r.callee_id))
        .filter((s): s is CctSymbol => s != null),
    };
  } catch {
    graphResults.value = { callers: [], callees: [] };
  } finally {
    graphLoading.value = false;
  }
}

async function handleQueryNodeCallers(sym: CctSymbol) {
  const projectId = projectStore.currentProjectId;
  if (!projectId) return;

  try {
    const rels = await queryApi.queryCallers(projectId, sym.id, 1);

    // 按需查询缺失的符号
    const missingIds = new Set<number>();
    for (const r of rels) {
      if (!graphSymMap.has(r.caller_id)) missingIds.add(r.caller_id);
      if (!graphSymMap.has(r.callee_id)) missingIds.add(r.callee_id);
    }
    if (missingIds.size > 0) {
      const fetched = await queryApi.getSymbolsByIds(
        projectId,
        Array.from(missingIds),
      );
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    const existingIds = new Set([
      graphSymbol.value!.id,
      ...graphResults.value.callers.map((s) => s.id),
      ...graphResults.value.callees.map((s) => s.id),
    ]);

    const newEdges: GraphEdgeData[] = [];
    const newCallers: CctSymbol[] = [];

    for (const r of rels) {
      const callerSym = graphSymMap.get(r.caller_id);
      if (!callerSym) continue;

      newEdges.push({ sourceId: r.caller_id, targetId: r.callee_id });

      if (!existingIds.has(callerSym.id)) {
        newCallers.push(callerSym);
        existingIds.add(callerSym.id);
      }
    }

    if (newCallers.length > 0) {
      graphResults.value = {
        callers: [...graphResults.value.callers, ...newCallers],
        callees: graphResults.value.callees,
      };
    }
    if (newEdges.length > 0) {
      graphExtraEdges.value = [...graphExtraEdges.value, ...newEdges];
    }

    if (newCallers.length === 0 && newEdges.length === 0) {
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

    // 按需查询缺失的符号
    const missingIds = new Set<number>();
    for (const r of rels) {
      if (!graphSymMap.has(r.caller_id)) missingIds.add(r.caller_id);
      if (!graphSymMap.has(r.callee_id)) missingIds.add(r.callee_id);
    }
    if (missingIds.size > 0) {
      const fetched = await queryApi.getSymbolsByIds(
        projectId,
        Array.from(missingIds),
      );
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    const existingIds = new Set([
      graphSymbol.value!.id,
      ...graphResults.value.callers.map((s) => s.id),
      ...graphResults.value.callees.map((s) => s.id),
    ]);

    const newEdges: GraphEdgeData[] = [];
    const newCallees: CctSymbol[] = [];

    for (const r of rels) {
      const calleeSym = graphSymMap.get(r.callee_id);
      if (!calleeSym) continue;

      newEdges.push({ sourceId: r.caller_id, targetId: r.callee_id });

      if (!existingIds.has(calleeSym.id)) {
        newCallees.push(calleeSym);
        existingIds.add(calleeSym.id);
      }
    }

    if (newCallees.length > 0) {
      graphResults.value = {
        callers: graphResults.value.callers,
        callees: [...graphResults.value.callees, ...newCallees],
      };
    }
    if (newEdges.length > 0) {
      graphExtraEdges.value = [...graphExtraEdges.value, ...newEdges];
    }

    if (newCallees.length === 0 && newEdges.length === 0) {
      Message.info("未发现更多被调用者");
    }
  } catch {
    Message.error("查询被调用者失败");
  }
}

async function handleShowCallGraph(line: number, _col: number) {
  const sym = await findSymbolAtLine(line);
  if (!sym) {
    Message.warning("未找到当前位置的符号");
    return;
  }
  await loadCallGraph(sym);
}

async function handleShowCallers(line: number, _col: number) {
  await handleShowCallGraph(line, _col);
}

function handleFindReferences(_line: number, _col: number) {
  Message.info("引用查询功能开发中");
}

function navigateToSymbol(sym: CctSymbol) {
  const projectId = projectStore.currentProjectId ?? undefined;
  editorStore.openFile(sym.file_path, projectId);
}

</script>

<template>
  <a-layout class="main-layout">
    <a-layout class="content-layout">
      <a-layout-sider
        :width="settings.sidebarWidth"
        :collapsed="settings.sidebarCollapsed"
        :collapsed-width="48"
        :hide-trigger="true"
        class="sidebar-sider"
      >
        <Sidebar />
      </a-layout-sider>

      <a-layout class="center-column">
        <a-layout class="work-area">
          <a-layout-content class="main-content">
            <template v-if="editorStore.hasOpenFiles">
              <EditorTabs />
              <div class="editor-wrapper">
                <CodeEditor
                  v-if="editorStore.activeFile"
                  :key="editorStore.activeFile.filePath"
                  :file-path="editorStore.activeFile.filePath"
                  :content="editorStore.activeFile.content"
                  :language="editorStore.activeFile.language"
                  @show-call-graph="handleShowCallGraph"
                  @show-callers="handleShowCallers"
                  @find-references="handleFindReferences"
                />
              </div>
            </template>
            <WelcomeScreen v-else />
          </a-layout-content>

          <!-- 调用图可视化面板 -->
          <a-layout-sider
            v-if="graphVisible"
            :width="480"
            class="graph-panel-sider"
          >
            <div class="graph-panel">
              <div class="graph-panel-header">
                <span class="graph-panel-title">
                  <icon-relation-one-to-many />
                  调用图 — {{ graphSymbol?.name ?? "" }}
                </span>
                <a-button size="mini" type="text" @click="graphVisible = false">
                  <template #icon><icon-close /></template>
                </a-button>
              </div>

              <a-spin :loading="graphLoading" style="width: 100%; flex: 1">
                <div v-if="!graphLoading && graphSymbol" class="graph-canvas-wrapper">
                  <CallGraphView
                    :root-symbol="graphSymbol"
                    :callers="graphResults.callers"
                    :callees="graphResults.callees"
                    :extra-edges="graphExtraEdges"
                    @navigate="navigateToSymbol"
                    @query-callers="handleQueryNodeCallers"
                    @query-callees="handleQueryNodeCallees"
                  />
                </div>
              </a-spin>
            </div>
          </a-layout-sider>

          <!-- AI 面板 -->
          <a-layout-sider
            v-if="settings.aiPanelVisible"
            :width="settings.aiPanelWidth"
            class="ai-panel-sider"
          >
            <AiPanel />
          </a-layout-sider>
        </a-layout>

        <!-- 底部面板 -->
        <div
          v-if="settings.bottomPanelVisible"
          class="bottom-panel"
          :style="{ height: settings.bottomPanelHeight + 'px' }"
        >
          <TerminalPanel />
        </div>
      </a-layout>
    </a-layout>

    <StatusBar />

    <SettingsDialog v-model:visible="showSettings" />
    <ProjectSettingsDialog
      v-if="settingsProject"
      v-model:visible="showProjectSettings"
      :project="settingsProject"
      @saved="handleSettingsSaved"
    />
  </a-layout>
</template>

<style scoped>
.main-layout {
  height: 100vh;
  overflow: hidden;
}

.content-layout {
  flex: 1;
  overflow: hidden;
}

.sidebar-sider {
  border-right: 1px solid var(--color-border);
}

.sidebar-sider :deep(.arco-layout-sider-trigger) {
  display: none;
}

.center-column {
  flex: 1;
  flex-direction: column !important;
  overflow: hidden;
}

.work-area {
  flex: 1;
  flex-direction: row !important;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-1);
}

.editor-wrapper {
  flex: 1;
  overflow: hidden;
}

.ai-panel-sider {
  border-left: 1px solid var(--color-border);
}

.graph-panel-sider {
  border-left: 1px solid var(--color-border);
}

.graph-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.graph-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.graph-panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.graph-canvas-wrapper {
  width: 100%;
  height: 100%;
  min-height: 300px;
}

.bottom-panel {
  border-top: 1px solid var(--color-border);
  overflow: hidden;
  flex-shrink: 0;
}
</style>
