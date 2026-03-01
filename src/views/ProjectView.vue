<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { Message, Modal } from "@arco-design/web-vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/stores/project";
import RemoteProjectWizard from "@/components/project/RemoteProjectWizard.vue";
import ProjectSettingsDialog from "@/components/project/ProjectSettingsDialog.vue";
import type { Project } from "@/api/types";

const { t } = useI18n();
const projectStore = useProjectStore();
const showRemoteWizard = ref(false);
const settingsProject = ref<Project | null>(null);
const showSettings = ref(false);

const isRunning = computed(
  () =>
    projectStore.parseStatus === "running" ||
    projectStore.parseStatus === "indexing",
);

onMounted(async () => {
  await projectStore.fetchProjects();
  await projectStore.listenParseProgress();
});

async function handleOpenDirectory() {
  const selected = await open({ directory: true, multiple: false });
  if (!selected) return;

  const dirPath = selected as string;
  const existing = projectStore.projects.find(
    (p) => p.source_root === dirPath && p.project_type === "Local",
  );
  if (existing) {
    projectStore.setCurrentProject(existing.id);
    Message.info(t("project.alreadyExists"));
    return;
  }

  try {
    await projectStore.openLocalDirectory(dirPath);
    Message.success(t("project.openSuccess"));
  } catch {
    // error handled in store
  }
}

function getStatusColor(status: string): string {
  const map: Record<string, string> = {
    NotStarted: "gray",
    InProgress: "blue",
    Completed: "green",
    Failed: "red",
  };
  return map[status] ?? "gray";
}

function getStatusLabel(project: Project): string {
  if (project.parse_status === "InProgress" && isRunning.value) {
    if (projectStore.parseProgress?.phase === "indexing") {
      return t("parse.indexing");
    }
    return t("parse.inProgress");
  }
  const map: Record<string, string> = {
    NotStarted: t("parse.notStarted"),
    InProgress: t("parse.inProgress"),
    Completed: t("parse.completed"),
    Failed: t("parse.failed"),
  };
  return map[project.parse_status] ?? project.parse_status;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return t("project.never");
  return new Date(dateStr).toLocaleString();
}

function handleRemoteSuccess() {
  showRemoteWizard.value = false;
  Message.success(t("project.createSuccess"));
  projectStore.fetchProjects();
}

function handleDelete(project: Project) {
  Modal.confirm({
    title: t("project.delete"),
    content: t("project.deleteConfirm", { name: project.name }),
    okButtonProps: { status: "danger" },
    onOk: async () => {
      await projectStore.deleteProject(project.id);
      Message.success(t("project.deleteSuccess"));
    },
  });
}

async function handleParse(project: Project) {
  await projectStore.startParse(project.id);
  Message.info(t("project.parseStarted"));
  await projectStore.fetchProjects();
}

function handleSettings(project: Project) {
  settingsProject.value = project;
  showSettings.value = true;
}

function handleSettingsSaved() {
  projectStore.fetchProjects();
}

function progressLabel(): string {
  const p = projectStore.parseProgress;
  if (!p) return "";
  if (p.phase === "scanning") return t("parse.scanning");
  if (p.phase === "indexing") {
    return `${t("parse.indexing")} ${p.parsed_files}/${p.total_files} — ${p.current_file}`;
  }
  return `${p.parsed_files}/${p.total_files} — ${p.current_file}`;
}
</script>

<template>
  <div class="project-view">
    <div class="project-header">
      <h3>{{ t("project.title") }}</h3>
      <a-space>
        <a-button type="primary" size="small" @click="handleOpenDirectory">
          <template #icon><icon-folder /></template>
          {{ t("project.openDirectory") }}
        </a-button>
        <a-button size="small" @click="showRemoteWizard = true">
          <template #icon><icon-cloud /></template>
          {{ t("project.newRemote") }}
        </a-button>
      </a-space>
    </div>

    <a-spin :loading="projectStore.loading" class="project-list-spin">
      <div v-if="projectStore.projects.length === 0" class="project-list-empty">
        <a-empty :description="t('project.noProjects')" />
      </div>

      <div v-else class="project-list">
        <a-card
          v-for="project in projectStore.projects"
          :key="project.id"
          class="project-card"
          hoverable
        >
          <template #title>
            <div class="card-title">
              <icon-folder style="margin-right: 8px; color: var(--color-primary-light-4)" />
              {{ project.name }}
            </div>
          </template>
          <template #extra>
            <a-tag :color="getStatusColor(project.parse_status)" size="small">
              {{ getStatusLabel(project) }}
            </a-tag>
          </template>

          <a-descriptions :column="1" size="small" layout="inline-horizontal">
            <a-descriptions-item :label="t('project.type')">
              <a-tag size="small">
                {{ project.project_type === "Local" ? t("project.local") : t("project.remote") }}
              </a-tag>
            </a-descriptions-item>
            <a-descriptions-item :label="t('project.sourceRoot')">
              <span class="path-text">{{ project.source_root }}</span>
            </a-descriptions-item>
            <a-descriptions-item :label="t('project.lastParse')">
              {{ formatDate(project.last_parse_at) }}
            </a-descriptions-item>
          </a-descriptions>

          <!-- 解析进度条 -->
          <div
            v-if="isRunning && projectStore.parseProgress"
            class="parse-progress"
          >
            <a-progress
              :percent="projectStore.parseProgress.percentage"
              :status="projectStore.parseProgress.phase === 'indexing' ? 'warning' : 'normal'"
              size="small"
            />
            <span class="progress-text">{{ progressLabel() }}</span>
          </div>

          <!-- 解析完成统计 -->
          <div
            v-if="projectStore.parseStatus === 'completed' && !isRunning"
            class="parse-done-banner"
          >
            <icon-check-circle style="color: rgb(var(--green-6)); margin-right: 6px" />
            {{ t("parse.completed") }}
          </div>

          <template #actions>
            <a-button
              type="text"
              size="small"
              @click="handleParse(project)"
              :disabled="isRunning"
            >
              <template #icon><icon-play-arrow /></template>
              {{ isRunning ? t("parse.inProgress") : t("project.parse") }}
            </a-button>
            <a-button
              type="text"
              size="small"
              @click="handleSettings(project)"
              :disabled="isRunning"
            >
              <template #icon><icon-settings /></template>
              {{ t("project.settings") }}
            </a-button>
            <a-button
              type="text"
              size="small"
              status="danger"
              @click="handleDelete(project)"
              :disabled="isRunning"
            >
              <template #icon><icon-delete /></template>
              {{ t("common.delete") }}
            </a-button>
          </template>
        </a-card>
      </div>
    </a-spin>

    <RemoteProjectWizard
      v-model:visible="showRemoteWizard"
      @success="handleRemoteSuccess"
    />

    <ProjectSettingsDialog
      v-if="settingsProject"
      v-model:visible="showSettings"
      :project="settingsProject"
      @saved="handleSettingsSaved"
    />
  </div>
</template>

<style scoped>
.project-view {
  padding: 16px;
}

.project-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.project-header h3 {
  margin: 0;
}

.project-list-spin {
  width: 100%;
}

.project-list-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
}

.project-list {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}

.project-card {
  border-radius: 8px;
}

.card-title {
  display: flex;
  align-items: center;
  font-weight: 600;
}

.path-text {
  font-family: monospace;
  font-size: 12px;
  color: var(--color-text-3);
  word-break: break-all;
}

.parse-progress {
  margin-top: 12px;
}

.progress-text {
  font-size: 12px;
  color: var(--color-text-3);
  margin-top: 4px;
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.parse-done-banner {
  margin-top: 12px;
  display: flex;
  align-items: center;
  font-size: 13px;
  color: rgb(var(--green-6));
}
</style>
