<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import { useWindowTitle } from "@/composables/useWindowTitle";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import Sidebar from "./Sidebar.vue";
import StatusBar from "./StatusBar.vue";
import AiPanel from "@/components/ai/AiPanel.vue";
import SplitEditor from "@/components/editor/SplitEditor.vue";
import WelcomeScreen from "@/components/welcome/WelcomeScreen.vue";
import TerminalPanel from "@/components/terminal/TerminalPanel.vue";
import ResultPanel from "@/components/search/ResultPanel.vue";
import GlobalSearch from "@/components/search/GlobalSearch.vue";
import SettingsDialog from "@/components/settings/SettingsDialog.vue";
import ProjectSettingsDialog from "@/components/project/ProjectSettingsDialog.vue";
import { Message } from "@arco-design/web-vue";
import { useI18n } from "vue-i18n";
import * as editorApi from "@/api/editor";
import * as queryApi from "@/api/query";
import type { Symbol as CctSymbol, Project, CallGraphData, GraphEdgeData } from "@/api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

const { t } = useI18n();
const settings = useSettingsStore();
const editorStore = useEditorStore();
const projectStore = useProjectStore();
useWindowTitle();

const resultPanelRef = ref<InstanceType<typeof ResultPanel> | null>(null);

// ── 侧边栏拖拽调整宽度 ──────────────────────────────────────
function onSidebarResizeStart(e: MouseEvent) {
  const startX = e.clientX;
  const startWidth = settings.sidebarWidth;
  const onMove = (ev: MouseEvent) => {
    const newWidth = startWidth + (ev.clientX - startX);
    settings.sidebarWidth = Math.max(120, Math.min(600, newWidth));
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}

// ── 底部面板拖拽调整高度 ─────────────────────────────────────
function onBottomResizeStart(e: MouseEvent) {
  const startY = e.clientY;
  const startHeight = settings.bottomPanelHeight;
  const onMove = (ev: MouseEvent) => {
    const newHeight = startHeight - (ev.clientY - startY);
    settings.bottomPanelHeight = Math.max(100, Math.min(600, newHeight));
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
  document.body.style.cursor = "row-resize";
  document.body.style.userSelect = "none";
}

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

// ── 调用图：加载并作为编辑器 Tab 打开 ────────────────────────
/**
 * 统一符号去重：按 qualified_name 合并，优先保留 is_definition=true 的版本。
 */
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

async function findSymbolAtLine(line: number): Promise<CctSymbol | null> {
  const projectId = projectStore.currentProjectId;
  const file = editorStore.activeFile;
  if (!projectId || !file) return null;

  try {
    const symbols = await editorApi.getFileSymbols(projectId, file.filePath);
    if (symbols.length === 0) return null;

    const exact = symbols.find(
      (s) => s.line <= line && (s.end_line ?? s.line) >= line,
    );
    if (exact) return exact;

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
  const projectId = projectStore.currentProjectId!;
  const graphSymMap = new Map<number, CctSymbol>();

  try {
    const [callerRels, calleeRels] = await Promise.all([
      queryApi.queryCallers(projectId, sym.id, 1),
      queryApi.queryCallees(projectId, sym.id, 1),
    ]);

    graphSymMap.set(sym.id, sym);

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
      const fetched = await queryApi.getSymbolsByIds(projectId, Array.from(neededIds));
      for (const s of fetched) graphSymMap.set(s.id, s);
    }

    const callers = deduplicateSymbols(
      callerRels
        .map((r) => graphSymMap.get(r.caller_id))
        .filter((s): s is CctSymbol => s != null),
    );
    const callees = deduplicateSymbols(
      calleeRels
        .map((r) => graphSymMap.get(r.callee_id))
        .filter((s): s is CctSymbol => s != null),
    );

    const initialEdges: GraphEdgeData[] = [];
    for (const c of callers) {
      initialEdges.push({ sourceId: c.id, targetId: sym.id });
    }
    for (const c of callees) {
      initialEdges.push({ sourceId: sym.id, targetId: c.id });
    }

    const graphData: CallGraphData = {
      symbol: sym,
      callers,
      callees,
      extraEdges: initialEdges,
    };

    editorStore.openCallGraph(graphData);
  } catch {
    Message.error("加载调用图失败");
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

async function handleFindReferences(line: number, _col: number) {
  const sym = await findSymbolAtLine(line);
  if (!sym) {
    Message.warning("未找到当前位置的符号");
    return;
  }
  settings.bottomPanelVisible = true;
  settings.bottomPanelTab = "references";
  await nextTick();
  resultPanelRef.value?.querySymbol(sym.id);
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
        <div
          v-if="!settings.sidebarCollapsed"
          class="sidebar-resize-handle"
          @mousedown.prevent="onSidebarResizeStart"
        />
      </a-layout-sider>

      <a-layout class="center-column">
        <a-layout class="work-area">
          <a-layout-content class="main-content">
            <template v-if="editorStore.hasOpenFiles">
              <SplitEditor
                @show-call-graph="handleShowCallGraph"
                @show-callers="handleShowCallers"
                @find-references="handleFindReferences"
              />
            </template>
            <WelcomeScreen v-else />
          </a-layout-content>

          <!-- AI 面板 -->
          <a-layout-sider
            v-if="settings.aiPanelVisible"
            :width="settings.aiPanelWidth"
            class="ai-panel-sider"
          >
            <AiPanel />
          </a-layout-sider>
        </a-layout>

        <!-- 底部面板（终端 + 引用查询） -->
        <div
          v-if="settings.bottomPanelVisible"
          class="bottom-panel"
          :style="{ height: settings.bottomPanelHeight + 'px' }"
        >
          <div class="bottom-resize-handle" @mousedown.prevent="onBottomResizeStart" />
          <div class="bottom-panel-header">
            <div class="bottom-panel-tabs">
              <div
                :class="['bp-tab', { active: settings.bottomPanelTab === 'terminal' }]"
                @click="settings.bottomPanelTab = 'terminal'"
              >
                <icon-code-block /> 终端
              </div>
              <div
                :class="['bp-tab', { active: settings.bottomPanelTab === 'references' }]"
                @click="settings.bottomPanelTab = 'references'"
              >
                <icon-find-replace /> 引用
              </div>
            </div>
            <a-button size="mini" type="text" @click="settings.bottomPanelVisible = false">
              <template #icon><icon-close /></template>
            </a-button>
          </div>
          <div class="bottom-panel-content">
            <TerminalPanel v-show="settings.bottomPanelTab === 'terminal'" />
            <ResultPanel v-show="settings.bottomPanelTab === 'references'" ref="resultPanelRef" />
          </div>
        </div>
      </a-layout>
    </a-layout>

    <StatusBar />

    <GlobalSearch />
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
  position: relative;
}

.sidebar-sider :deep(.arco-layout-sider-trigger) {
  display: none;
}

.sidebar-resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  width: 4px;
  height: 100%;
  cursor: col-resize;
  z-index: 20;
  transition: background 0.15s;
}

.sidebar-resize-handle:hover,
.sidebar-resize-handle:active {
  background: rgb(var(--primary-6));
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

.ai-panel-sider {
  border-left: 1px solid var(--color-border);
}

.bottom-panel {
  border-top: 1px solid var(--color-border);
  overflow: hidden;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  position: relative;
}

.bottom-resize-handle {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 4px;
  cursor: row-resize;
  z-index: 20;
  transition: background 0.15s;
}

.bottom-resize-handle:hover,
.bottom-resize-handle:active {
  background: rgb(var(--primary-6));
}

.bottom-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px;
  background: var(--color-bg-2);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.bottom-panel-tabs {
  display: flex;
  align-items: center;
  gap: 2px;
}

.bp-tab {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 10px;
  font-size: 12px;
  cursor: pointer;
  border-radius: 4px;
  color: var(--color-text-3);
  transition: all 0.15s;
}

.bp-tab:hover {
  color: var(--color-text-1);
  background: var(--color-fill-2);
}

.bp-tab.active {
  color: var(--color-text-1);
  background: var(--color-bg-1);
  font-weight: 500;
}

.bottom-panel-content {
  flex: 1;
  overflow: hidden;
}
</style>
