import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { Project, ParseProgress, ParseStatistics } from "@/api/types";
import * as projectApi from "@/api/project";

/**
 * 项目管理 Store — 集中管理项目列表、当前项目、解析状态
 *
 * # 设计说明（观察者模式）
 * 通过 Tauri 事件系统监听后端发射的 `parse-progress` 事件，
 * 自动更新前端解析进度状态，组件无需手动轮询。
 */
export const useProjectStore = defineStore("project", () => {
  const projects = ref<Project[]>([]);
  const currentProjectId = ref<string | null>(null);
  const parseProgress = ref<ParseProgress | null>(null);
  const parseStatus = ref<string>("idle");
  const loading = ref(false);
  const error = ref<string | null>(null);

  const currentProject = computed(() =>
    projects.value.find((p) => p.id === currentProjectId.value) ?? null,
  );

  // ── 项目 CRUD ────────────────────────────────────────────────

  async function fetchProjects() {
    loading.value = true;
    error.value = null;
    try {
      projects.value = await projectApi.listProjects();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function createLocalProject(name: string, sourceRoot: string) {
    error.value = null;
    try {
      const project = await projectApi.createLocalProject(name, sourceRoot);
      projects.value.unshift(project);
      return project;
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function openLocalDirectory(dirPath: string) {
    const dirName = dirPath.split("/").filter(Boolean).pop() ?? dirPath;
    const existing = projects.value.find(
      (p) => p.source_root === dirPath && p.project_type === "Local",
    );
    if (existing) {
      currentProjectId.value = existing.id;
      return existing;
    }
    const project = await createLocalProject(dirName, dirPath);
    currentProjectId.value = project.id;
    return project;
  }

  async function deleteProject(projectId: string) {
    error.value = null;
    try {
      await projectApi.deleteProject(projectId);
      projects.value = projects.value.filter((p) => p.id !== projectId);
      if (currentProjectId.value === projectId) {
        currentProjectId.value = null;
      }
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function updateProject(
    projectId: string,
    opts?: {
      name?: string;
      compileDbPath?: string;
      excludedDirs?: string[];
    },
  ) {
    error.value = null;
    try {
      const updated = await projectApi.updateProject(projectId, opts);
      const idx = projects.value.findIndex((p) => p.id === projectId);
      if (idx !== -1) {
        projects.value[idx] = updated;
      }
      return updated;
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  function setCurrentProject(projectId: string | null) {
    currentProjectId.value = projectId;
  }

  // ── 解析控制 ─────────────────────────────────────────────────

  async function startParse(projectId: string) {
    error.value = null;
    parseStatus.value = "running";
    parseProgress.value = null;
    try {
      await projectApi.startFullParse(projectId);
    } catch (e) {
      parseStatus.value = "error";
      error.value = String(e);
      throw e;
    }
  }

  async function cancelParse(projectId: string) {
    try {
      await projectApi.cancelParse(projectId);
      parseStatus.value = "idle";
      parseProgress.value = null;
    } catch (e) {
      error.value = String(e);
    }
  }

  async function fetchParseStatistics(
    projectId: string,
  ): Promise<ParseStatistics> {
    return projectApi.getParseStatistics(projectId);
  }

  // ── 事件监听 ─────────────────────────────────────────────────

  async function listenParseProgress() {
    await listen<ParseProgress>("parse-progress", (event) => {
      const p = { ...event.payload };
      // 防御性夹紧：后端浮点误差或并发竞态可能导致百分比溢出
      const raw = Number(p.percentage);
      p.percentage = Number.isFinite(raw) ? Math.min(100, Math.max(0, raw)) : 0;
      parseProgress.value = p;

      if (p.phase === "parsing") {
        parseStatus.value = "running";
      } else if (p.phase === "indexing") {
        parseStatus.value = "indexing";
      }
    });

    await listen<{ project_id: string; statistics?: unknown }>(
      "parse-complete",
      (_event) => {
        parseStatus.value = "completed";
        parseProgress.value = null;
        fetchProjects();
      },
    );

    await listen<{ project_id: string; error: string }>(
      "parse-error",
      (event) => {
        parseStatus.value = "error";
        error.value = event.payload.error;
        parseProgress.value = null;
        fetchProjects();
      },
    );
  }

  return {
    projects,
    currentProjectId,
    currentProject,
    parseProgress,
    parseStatus,
    loading,
    error,
    fetchProjects,
    createLocalProject,
    openLocalDirectory,
    deleteProject,
    updateProject,
    setCurrentProject,
    startParse,
    cancelParse,
    fetchParseStatistics,
    listenParseProgress,
  };
});
