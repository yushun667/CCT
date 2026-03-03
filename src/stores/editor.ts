/**
 * 编辑器 Store — 管理双窗格编辑区、Tab 生命周期和调用图 Tab
 *
 * 支持左右两个窗格（pane），每个窗格独立管理 Tab 列表和活跃索引。
 * 窗格 0 始终存在，窗格 1 为可选的右侧拆分。
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { EditorFile, Symbol, CallGraphData } from "@/api/types";
import * as editorApi from "@/api/editor";

export interface EditorPane {
  openFiles: EditorFile[];
  activeFileIndex: number;
}

export const useEditorStore = defineStore("editor", () => {
  const panes = ref<EditorPane[]>([{ openFiles: [], activeFileIndex: -1 }]);
  const activePaneIndex = ref(0);
  const splitMode = ref(false);
  const targetLine = ref<number | null>(null);
  const targetLineSeq = ref(0);
  const fileSymbols = ref<Symbol[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const activePane = computed(() => panes.value[activePaneIndex.value]);

  const openFiles = computed(() => activePane.value.openFiles);
  const activeFileIndex = computed(() => activePane.value.activeFileIndex);

  const activeFile = computed(() => {
    const pane = activePane.value;
    return pane.activeFileIndex >= 0 && pane.activeFileIndex < pane.openFiles.length
      ? pane.openFiles[pane.activeFileIndex]
      : null;
  });

  const hasOpenFiles = computed(() =>
    panes.value.some((p) => p.openFiles.length > 0),
  );

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

  function getPaneFiles(paneIdx: number) {
    return panes.value[paneIdx]?.openFiles ?? [];
  }

  function getPaneActiveIndex(paneIdx: number) {
    return panes.value[paneIdx]?.activeFileIndex ?? -1;
  }

  function getPaneActiveFile(paneIdx: number) {
    const pane = panes.value[paneIdx];
    if (!pane) return null;
    return pane.activeFileIndex >= 0 && pane.activeFileIndex < pane.openFiles.length
      ? pane.openFiles[pane.activeFileIndex]
      : null;
  }

  async function openFile(filePath: string, projectId?: string, line?: number, targetPaneIdx?: number) {
    error.value = null;
    targetLine.value = line ?? null;
    targetLineSeq.value++;

    const paneIdx = targetPaneIdx ?? activePaneIndex.value;
    const pane = panes.value[paneIdx];
    if (!pane) return;

    const existingIdx = pane.openFiles.findIndex(
      (f) => f.filePath === filePath && f.type === "file",
    );
    if (existingIdx >= 0) {
      pane.activeFileIndex = existingIdx;
      activePaneIndex.value = paneIdx;
      if (projectId) {
        await loadFileSymbols(projectId, filePath);
      }
      return;
    }

    loading.value = true;
    try {
      const content = await editorApi.readFileContent(filePath);
      const fileName =
        filePath.split("/").pop() ??
        filePath.split("\\").pop() ??
        filePath;

      const file: EditorFile = {
        filePath,
        fileName,
        content,
        language: detectLanguage(filePath),
        type: "file",
      };

      pane.openFiles.push(file);
      pane.activeFileIndex = pane.openFiles.length - 1;
      activePaneIndex.value = paneIdx;

      if (projectId) {
        await loadFileSymbols(projectId, filePath);
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function openCallGraph(graphData: CallGraphData, targetPaneIdx?: number) {
    const paneIdx = targetPaneIdx ?? activePaneIndex.value;
    const pane = panes.value[paneIdx];
    if (!pane) return;

    const tabId = `call-graph:${graphData.symbol.id}`;

    const existingIdx = pane.openFiles.findIndex((f) => f.filePath === tabId);
    if (existingIdx >= 0) {
      pane.openFiles[existingIdx].graphData = graphData;
      pane.activeFileIndex = existingIdx;
      activePaneIndex.value = paneIdx;
      return;
    }

    const file: EditorFile = {
      filePath: tabId,
      fileName: `调用图 — ${graphData.symbol.name}`,
      content: "",
      language: "",
      type: "call-graph",
      graphData,
    };

    pane.openFiles.push(file);
    pane.activeFileIndex = pane.openFiles.length - 1;
    activePaneIndex.value = paneIdx;
  }

  function updateCallGraphData(tabId: string, graphData: CallGraphData) {
    for (const pane of panes.value) {
      const idx = pane.openFiles.findIndex((f) => f.filePath === tabId);
      if (idx >= 0) {
        pane.openFiles[idx].graphData = graphData;
        return;
      }
    }
  }

  function closeFile(index: number, paneIdx?: number) {
    const pi = paneIdx ?? activePaneIndex.value;
    const pane = panes.value[pi];
    if (!pane) return;
    if (index < 0 || index >= pane.openFiles.length) return;

    pane.openFiles.splice(index, 1);
    if (pane.openFiles.length === 0) {
      pane.activeFileIndex = -1;
      if (splitMode.value && pi === 1) {
        closeSplit();
      }
    } else if (pane.activeFileIndex >= pane.openFiles.length) {
      pane.activeFileIndex = pane.openFiles.length - 1;
    } else if (pane.activeFileIndex > index) {
      pane.activeFileIndex--;
    }
  }

  function setActiveFile(index: number, paneIdx?: number) {
    const pi = paneIdx ?? activePaneIndex.value;
    const pane = panes.value[pi];
    if (!pane) return;
    if (index >= 0 && index < pane.openFiles.length) {
      pane.activeFileIndex = index;
      activePaneIndex.value = pi;
    }
  }

  function setActivePane(paneIdx: number) {
    if (paneIdx >= 0 && paneIdx < panes.value.length) {
      activePaneIndex.value = paneIdx;
    }
  }

  function splitRight() {
    if (splitMode.value) return;
    splitMode.value = true;
    panes.value.push({ openFiles: [], activeFileIndex: -1 });
  }

  function closeSplit() {
    if (!splitMode.value) return;
    const rightPane = panes.value[1];
    if (rightPane) {
      for (const f of rightPane.openFiles) {
        const leftPane = panes.value[0];
        if (!leftPane.openFiles.find((lf) => lf.filePath === f.filePath)) {
          leftPane.openFiles.push(f);
        }
      }
      if (panes.value[0].activeFileIndex < 0 && panes.value[0].openFiles.length > 0) {
        panes.value[0].activeFileIndex = 0;
      }
    }
    panes.value.splice(1, 1);
    splitMode.value = false;
    activePaneIndex.value = 0;
  }

  function moveToPane(fromPaneIdx: number, toPaneIdx: number, fileIdx: number, insertIdx?: number) {
    if (toPaneIdx === 1 && !splitMode.value) {
      splitRight();
    }
    const from = panes.value[fromPaneIdx];
    const to = panes.value[toPaneIdx];
    if (!from || !to) return;
    if (fileIdx < 0 || fileIdx >= from.openFiles.length) return;

    const [file] = from.openFiles.splice(fileIdx, 1);
    const idx = insertIdx ?? to.openFiles.length;
    const newFiles = [...to.openFiles];
    newFiles.splice(idx, 0, file);
    to.openFiles = newFiles;
    to.activeFileIndex = idx;

    if (from.openFiles.length === 0) {
      from.activeFileIndex = -1;
    } else if (from.activeFileIndex >= from.openFiles.length) {
      from.activeFileIndex = from.openFiles.length - 1;
    }

    activePaneIndex.value = toPaneIdx;
  }

  function closeOtherFiles(index: number, paneIdx?: number) {
    const pi = paneIdx ?? activePaneIndex.value;
    const pane = panes.value[pi];
    if (!pane) return;
    if (index < 0 || index >= pane.openFiles.length) return;
    pane.openFiles = [pane.openFiles[index]];
    pane.activeFileIndex = 0;
  }

  function closeAllInPane(paneIdx?: number) {
    const pi = paneIdx ?? activePaneIndex.value;
    const pane = panes.value[pi];
    if (!pane) return;
    pane.openFiles = [];
    pane.activeFileIndex = -1;
    if (splitMode.value && pi === 1) {
      closeSplit();
    }
  }

  function reorderFile(paneIdx: number, fromIdx: number, toIdx: number) {
    const pane = panes.value[paneIdx];
    if (!pane) return;
    if (fromIdx < 0 || fromIdx >= pane.openFiles.length) return;
    if (toIdx < 0 || toIdx > pane.openFiles.length) return;
    if (fromIdx === toIdx) return;

    const [file] = pane.openFiles.splice(fromIdx, 1);
    const dest = toIdx > fromIdx ? toIdx - 1 : toIdx;
    pane.openFiles.splice(dest, 0, file);
    pane.activeFileIndex = dest;
  }

  function splitFileToRight(fileIdx: number, fromPaneIdx?: number) {
    const pi = fromPaneIdx ?? activePaneIndex.value;
    if (!splitMode.value) {
      splitRight();
    }
    const targetPane = pi === 0 ? 1 : 0;
    moveToPane(pi, targetPane, fileIdx);
  }

  async function loadFileSymbols(projectId: string, filePath: string) {
    try {
      fileSymbols.value = await editorApi.getFileSymbols(projectId, filePath);
    } catch {
      fileSymbols.value = [];
    }
  }

  function closeAllFiles() {
    for (const pane of panes.value) {
      pane.openFiles = [];
      pane.activeFileIndex = -1;
    }
    fileSymbols.value = [];
    if (splitMode.value) {
      panes.value.splice(1);
      splitMode.value = false;
      activePaneIndex.value = 0;
    }
  }

  return {
    panes,
    activePaneIndex,
    splitMode,
    targetLine,
    targetLineSeq,
    activePane,
    openFiles,
    activeFileIndex,
    activeFile,
    hasOpenFiles,
    fileSymbols,
    loading,
    error,
    getPaneFiles,
    getPaneActiveIndex,
    getPaneActiveFile,
    openFile,
    openCallGraph,
    updateCallGraphData,
    closeFile,
    setActiveFile,
    setActivePane,
    splitRight,
    closeSplit,
    moveToPane,
    closeOtherFiles,
    closeAllInPane,
    reorderFile,
    splitFileToRight,
    loadFileSymbols,
    closeAllFiles,
  };
});
