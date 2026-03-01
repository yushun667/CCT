<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Message, Modal } from "@arco-design/web-vue";
import { useProjectStore } from "@/stores/project";
import CreateProjectDialog from "@/components/project/CreateProjectDialog.vue";
import type { Project } from "@/api/types";

const { t } = useI18n();
const projectStore = useProjectStore();
const showCreateDialog = ref(false);

onMounted(async () => {
  await projectStore.fetchProjects();
  await projectStore.listenParseProgress();
});

function getStatusColor(status: string): string {
  const map: Record<string, string> = {
    NotStarted: "gray",
    InProgress: "blue",
    Completed: "green",
    Failed: "red",
  };
  return map[status] ?? "gray";
}

function getStatusLabel(status: string): string {
  const map: Record<string, string> = {
    NotStarted: t("parse.notStarted"),
    InProgress: t("parse.inProgress"),
    Completed: t("parse.completed"),
    Failed: t("parse.failed"),
  };
  return map[status] ?? status;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return t("project.never");
  return new Date(dateStr).toLocaleString();
}

function handleCreateSuccess() {
  showCreateDialog.value = false;
  Message.success(t("project.createSuccess"));
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
</script>

<template>
  <div class="project-view">
    <div class="project-header">
      <h3>{{ t("project.title") }}</h3>
      <a-space>
        <a-button type="primary" size="small" @click="showCreateDialog = true">
          <template #icon><icon-plus /></template>
          {{ t("project.newLocal") }}
        </a-button>
        <a-button size="small" disabled>
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
              {{ getStatusLabel(project.parse_status) }}
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
            v-if="
              project.parse_status === 'InProgress' &&
              projectStore.parseProgress
            "
            class="parse-progress"
          >
            <a-progress
              :percent="projectStore.parseProgress.percentage / 100"
              :status="'normal'"
              size="small"
            />
            <span class="progress-text">
              {{ projectStore.parseProgress.current_file }}
            </span>
          </div>

          <template #actions>
            <a-button
              type="text"
              size="small"
              @click="handleParse(project)"
              :disabled="project.parse_status === 'InProgress'"
            >
              <template #icon><icon-play-arrow /></template>
              {{ t("project.parse") }}
            </a-button>
            <a-button type="text" size="small" disabled>
              <template #icon><icon-settings /></template>
              {{ t("project.settings") }}
            </a-button>
            <a-button
              type="text"
              size="small"
              status="danger"
              @click="handleDelete(project)"
            >
              <template #icon><icon-delete /></template>
              {{ t("common.delete") }}
            </a-button>
          </template>
        </a-card>
      </div>
    </a-spin>

    <CreateProjectDialog
      v-model:visible="showCreateDialog"
      @success="handleCreateSuccess"
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
</style>
