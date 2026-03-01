<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { Message } from "@arco-design/web-vue";
import { useProjectStore } from "@/stores/project";

const { t } = useI18n();
const projectStore = useProjectStore();

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

function handleSelectProject(projectId: string) {
  projectStore.setCurrentProject(projectId);
}

async function handleParse() {
  if (!projectStore.currentProject) return;
  await projectStore.startParse(projectStore.currentProject.id);
}
</script>

<template>
  <div class="welcome-screen">
    <div class="welcome-center">
      <div class="welcome-logo">
        <icon-code-block style="font-size: 64px; color: var(--color-primary-light-4)" />
      </div>

      <h1 class="welcome-title">{{ t("welcome.title") }}</h1>
      <p class="welcome-subtitle">{{ t("welcome.subtitle") }}</p>

      <div class="welcome-actions">
        <a-button type="primary" size="large" @click="handleOpenDirectory">
          <template #icon><icon-folder-add /></template>
          {{ t("welcome.openProject") }}
        </a-button>

        <a-button
          v-if="projectStore.currentProject"
          size="large"
          @click="handleParse"
          :disabled="projectStore.parseStatus === 'running'"
        >
          <template #icon><icon-play-arrow /></template>
          {{ t("welcome.startParse") }}
        </a-button>
      </div>

      <!-- 解析进度 -->
      <div
        v-if="projectStore.parseStatus === 'running' || projectStore.parseStatus === 'indexing'"
        class="welcome-progress"
      >
        <a-progress
          :percent="projectStore.parseProgress?.percentage ?? 0"
          :status="projectStore.parseProgress?.phase === 'indexing' ? 'warning' : 'normal'"
        />
        <span class="progress-file">
          {{ projectStore.parseProgress?.current_file?.split("/").pop() ?? "" }}
        </span>
      </div>

      <div v-if="projectStore.parseStatus === 'completed'" class="welcome-done">
        <a-tag color="green"><icon-check-circle /> {{ t("parse.completed") }}</a-tag>
      </div>

      <!-- 最近项目 -->
      <div v-if="projectStore.projects.length > 0" class="recent-projects">
        <h3>{{ t("welcome.recentProjects") }}</h3>
        <div class="project-list">
          <div
            v-for="project in projectStore.projects"
            :key="project.id"
            class="project-item"
            :class="{ active: project.id === projectStore.currentProjectId }"
            @click="handleSelectProject(project.id)"
          >
            <icon-folder style="flex-shrink: 0; color: var(--color-primary-light-4)" />
            <div class="project-info">
              <span class="project-name">{{ project.name }}</span>
              <span class="project-path">{{ project.source_root }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome-screen {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 40px;
  overflow: auto;
}

.welcome-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 560px;
  width: 100%;
}

.welcome-logo {
  margin-bottom: 16px;
}

.welcome-title {
  font-size: 28px;
  font-weight: 600;
  margin: 0 0 8px;
  color: var(--color-text-1);
}

.welcome-subtitle {
  font-size: 14px;
  color: var(--color-text-3);
  margin: 0 0 32px;
}

.welcome-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.welcome-progress {
  width: 100%;
  margin-bottom: 16px;
}

.progress-file {
  font-size: 12px;
  color: var(--color-text-3);
  display: block;
  text-align: center;
  margin-top: 4px;
}

.welcome-done {
  margin-bottom: 16px;
}

.recent-projects {
  width: 100%;
  margin-top: 16px;
}

.recent-projects h3 {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-2);
  margin: 0 0 12px;
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.project-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.project-item:hover {
  background: var(--color-fill-2);
}

.project-item.active {
  background: var(--color-primary-light-1);
}

.project-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.project-name {
  font-size: 13px;
  color: var(--color-text-1);
  font-weight: 500;
}

.project-path {
  font-size: 11px;
  color: var(--color-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
