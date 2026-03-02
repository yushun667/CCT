/**
 * 编辑器 Store — 管理打开的文件 Tab 和活跃文件状态
 *
 * # 设计说明（状态模式）
 * 集中管理编辑器 Tab 的生命周期：打开、关闭、切换。
 * 通过 Tauri IPC 从后端读取文件内容，缓存在前端避免重复请求。
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { EditorFile, Symbol } from "@/api/types";
import * as editorApi from "@/api/editor";

export const useEditorStore = defineStore("editor", () => {
  const openFiles = ref<EditorFile[]>([]);
  const activeFileIndex = ref(-1);
  const targetLine = ref<number | null>(null);
  const targetLineSeq = ref(0);
  const fileSymbols = ref<Symbol[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const activeFile = computed(() =>
    activeFileIndex.value >= 0 && activeFileIndex.value < openFiles.value.length
      ? openFiles.value[activeFileIndex.value]
      : null,
  );

  const hasOpenFiles = computed(() => openFiles.value.length > 0);

  function detectLanguage(filePath: string): string {
    const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
    const langMap: Record<string, string> = {
      c: "c",
      h: "c",
      cpp: "cpp",
      cc: "cpp",
      cxx: "cpp",
      hpp: "cpp",
      hxx: "cpp",
      hh: "cpp",
      json: "json",
      xml: "xml",
      md: "markdown",
      txt: "plaintext",
      py: "python",
      rs: "rust",
      ts: "typescript",
      js: "javascript",
    };
    return langMap[ext] ?? "plaintext";
  }

  async function openFile(filePath: string, projectId?: string, line?: number) {
    error.value = null;
    targetLine.value = line ?? null;
    targetLineSeq.value++;

    const existingIdx = openFiles.value.findIndex(
      (f) => f.filePath === filePath,
    );
    if (existingIdx >= 0) {
      activeFileIndex.value = existingIdx;
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
      };

      openFiles.value.push(file);
      activeFileIndex.value = openFiles.value.length - 1;

      if (projectId) {
        await loadFileSymbols(projectId, filePath);
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function closeFile(index: number) {
    if (index < 0 || index >= openFiles.value.length) return;

    openFiles.value.splice(index, 1);
    if (openFiles.value.length === 0) {
      activeFileIndex.value = -1;
    } else if (activeFileIndex.value >= openFiles.value.length) {
      activeFileIndex.value = openFiles.value.length - 1;
    } else if (activeFileIndex.value > index) {
      activeFileIndex.value--;
    }
  }

  function setActiveFile(index: number) {
    if (index >= 0 && index < openFiles.value.length) {
      activeFileIndex.value = index;
    }
  }

  async function loadFileSymbols(projectId: string, filePath: string) {
    try {
      fileSymbols.value = await editorApi.getFileSymbols(projectId, filePath);
    } catch {
      fileSymbols.value = [];
    }
  }

  function closeAllFiles() {
    openFiles.value = [];
    activeFileIndex.value = -1;
    fileSymbols.value = [];
  }

  return {
    openFiles,
    activeFileIndex,
    targetLine,
    targetLineSeq,
    activeFile,
    hasOpenFiles,
    fileSymbols,
    loading,
    error,
    openFile,
    closeFile,
    setActiveFile,
    loadFileSymbols,
    closeAllFiles,
  };
});
