<script setup lang="ts">
import { ref } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import Sidebar from "./Sidebar.vue";
import Toolbar from "./Toolbar.vue";
import StatusBar from "./StatusBar.vue";
import AiPanel from "@/components/ai/AiPanel.vue";
import EditorTabs from "@/components/editor/EditorTabs.vue";
import CodeEditor from "@/components/editor/CodeEditor.vue";
import WelcomeScreen from "@/components/welcome/WelcomeScreen.vue";
import TerminalPanel from "@/components/terminal/TerminalPanel.vue";
import CallGraphView from "@/components/graph/CallGraphView.vue";
import { Message } from "@arco-design/web-vue";
import * as editorApi from "@/api/editor";
import * as queryApi from "@/api/query";
import type { Symbol as CctSymbol } from "@/api/types";

const settings = useSettingsStore();
const editorStore = useEditorStore();
const projectStore = useProjectStore();

const graphVisible = ref(false);
const graphSymbol = ref<CctSymbol | null>(null);
const graphResults = ref<{ callers: CctSymbol[]; callees: CctSymbol[] }>({
  callers: [],
  callees: [],
});
const graphLoading = ref(false);

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

  const projectId = projectStore.currentProjectId!;
  try {
    const [callerRels, calleeRels] = await Promise.all([
      queryApi.queryCallers(projectId, sym.id, 5),
      queryApi.queryCallees(projectId, sym.id, 5),
    ]);

    // 收集所有关联的符号 ID
    const neededIds = new Set<number>();
    callerRels.forEach((r) => neededIds.add(r.caller_id));
    calleeRels.forEach((r) => neededIds.add(r.callee_id));

    // 获取当前文件符号 + 全局搜索来补充
    const fileSymbols = await editorApi.getFileSymbols(
      projectId,
      editorStore.activeFile?.filePath ?? "",
    );
    const globalSymbols = await queryApi.searchSymbols(projectId, "", undefined, 5000);
    const symMap = new Map<number, CctSymbol>();
    for (const s of fileSymbols) symMap.set(s.id, s);
    for (const s of globalSymbols) symMap.set(s.id, s);

    graphResults.value = {
      callers: callerRels
        .map((r) => symMap.get(r.caller_id))
        .filter((s): s is CctSymbol => s != null),
      callees: calleeRels
        .map((r) => symMap.get(r.callee_id))
        .filter((s): s is CctSymbol => s != null),
    };
  } catch {
    graphResults.value = { callers: [], callees: [] };
  } finally {
    graphLoading.value = false;
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

async function handleExpandNode(sym: CctSymbol) {
  await loadCallGraph(sym);
}
</script>

<template>
  <a-layout class="main-layout">
    <Toolbar />

    <a-layout class="content-layout">
      <a-layout-sider
        :width="settings.sidebarWidth"
        :collapsed="settings.sidebarCollapsed"
        :collapsed-width="48"
        collapsible
        :trigger="null"
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
                    @navigate="navigateToSymbol"
                    @expand="handleExpandNode"
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
