/**
 * 编辑器 Store — 通过 dockview-vue 管理编辑区布局与 Tab 生命周期
 *
 * 使用 DockviewApi 进行面板增删、激活；panelDataMap 维护每个面板的业务数据。
 * 所有窗格拆分、Tab 拖拽、右键菜单等 UI 交互由 dockview 内置处理。
 */
import { defineStore } from "pinia";
import { ref, computed, shallowRef } from "vue";
import type { DockviewApi } from "dockview-core";
import type { EditorFile, Symbol, CallGraphData } from "@/api/types";
import * as editorApi from "@/api/editor";

export const useEditorStore = defineStore("editor", () => {
  const dockApi = shallowRef<DockviewApi | null>(null);
  const panelDataMap = ref(new Map<string, EditorFile>());
  const activePanelId = ref<string | null>(null);

  /** 导航目标面板 ID，仅在 openFile 传入 line 时更新 */
  const targetPanelId = ref<string | null>(null);
  const targetLine = ref<number | null>(null);
  const targetLineSeq = ref(0);

  const fileSymbols = ref<Symbol[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const activeFile = computed(() => {
    if (!activePanelId.value) return null;
    return panelDataMap.value.get(activePanelId.value) ?? null;
  });

  const hasOpenFiles = computed(() => panelDataMap.value.size > 0);

  /**
   * 绑定 DockviewApi，监听面板激活和移除事件以同步响应式状态。
   */
  function setDockApi(api: DockviewApi) {
    dockApi.value = api;
    api.onDidActivePanelChange((panel) => {
      activePanelId.value = panel?.id ?? null;
    });
    api.onDidRemovePanel((panel) => {
      panelDataMap.value.delete(panel.id);
    });
  }

  function detectLanguage(filePath: string): string {
    const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
    const langMap: Record<string, string> = {
      c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp",
      hpp: "cpp", hxx: "cpp", hh: "cpp", json: "json",
      xml: "xml", md: "markdown", txt: "plaintext", py: "python",
      rs: "rust", ts: "typescript", js: "javascript",
    };
    return langMap[ext] ?? "plaintext";
  }

  async function openFile(filePath: string, projectId?: string, line?: number) {
    error.value = null;
    const panelId = `file:${filePath}`;

    if (line != null) {
      targetPanelId.value = panelId;
      targetLine.value = line;
      targetLineSeq.value++;
    }

    const existing = dockApi.value?.getPanel(panelId);
    if (existing) {
      existing.api.setActive();
      if (projectId) await loadFileSymbols(projectId, filePath);
      return;
    }

    loading.value = true;
    try {
      const content = await editorApi.readFileContent(filePath);
      const fileName =
        filePath.split("/").pop() ??
        filePath.split("\\").pop() ??
        filePath;
      const language = detectLanguage(filePath);

      const file: EditorFile = {
        filePath,
        fileName,
        content,
        language,
        type: "file",
      };

      panelDataMap.value.set(panelId, file);

      dockApi.value?.addPanel({
        id: panelId,
        component: "codeEditor",
        title: fileName,
        params: { panelId },
      });

      if (projectId) await loadFileSymbols(projectId, filePath);
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function openCallGraph(graphData: CallGraphData) {
    const panelId = `call-graph:${graphData.symbol.id}`;

    const existing = dockApi.value?.getPanel(panelId);
    if (existing) {
      const file = panelDataMap.value.get(panelId);
      if (file) file.graphData = graphData;
      existing.api.setActive();
      return;
    }

    const file: EditorFile = {
      filePath: panelId,
      fileName: `调用图 — ${graphData.symbol.name}`,
      content: "",
      language: "",
      type: "call-graph",
      graphData,
    };

    panelDataMap.value.set(panelId, file);

    dockApi.value?.addPanel({
      id: panelId,
      component: "callGraph",
      title: `调用图 — ${graphData.symbol.name}`,
      params: { panelId },
    });
  }

  function updateCallGraphData(tabId: string, graphData: CallGraphData) {
    const file = panelDataMap.value.get(tabId);
    if (file) {
      file.graphData = graphData;
    }
  }

  function closeAllFiles() {
    dockApi.value?.clear();
    panelDataMap.value.clear();
    fileSymbols.value = [];
  }

  async function loadFileSymbols(projectId: string, filePath: string) {
    try {
      fileSymbols.value = await editorApi.getFileSymbols(projectId, filePath);
    } catch {
      fileSymbols.value = [];
    }
  }

  return {
    dockApi,
    panelDataMap,
    activePanelId,
    targetPanelId,
    targetLine,
    targetLineSeq,
    activeFile,
    hasOpenFiles,
    fileSymbols,
    loading,
    error,
    setDockApi,
    detectLanguage,
    openFile,
    openCallGraph,
    updateCallGraphData,
    closeAllFiles,
    loadFileSymbols,
  };
});
