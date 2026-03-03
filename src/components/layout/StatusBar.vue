<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import { useProjectStore } from "@/stores/project";
import RemoteProjectWizard from "@/components/project/RemoteProjectWizard.vue";

import MdiSsh from "~icons/mdi/ssh";

const { t } = useI18n();
const settings = useSettingsStore();
const projectStore = useProjectStore();

const showRemoteWizard = ref(false);

function openRemoteWizard() {
  showRemoteWizard.value = true;
}

function onRemoteSuccess() {
  showRemoteWizard.value = false;
  projectStore.fetchProjects();
}

const isRunning = computed(
  () =>
    projectStore.parseStatus === "running" ||
    projectStore.parseStatus === "indexing",
);

const statusLabel = computed(() => {
  if (projectStore.parseStatus === "running") return t("parse.inProgress");
  if (projectStore.parseStatus === "indexing") return t("parse.indexing");
  if (projectStore.parseStatus === "completed") return t("parse.completed");
  if (projectStore.parseStatus === "error") return t("parse.failed");
  return t("status.ready");
});

const statusIcon = computed(() => {
  if (isRunning.value) return "loading";
  if (projectStore.parseStatus === "completed") return "success";
  if (projectStore.parseStatus === "error") return "error";
  return "ready";
});

const progressFile = computed(() => {
  const file = projectStore.parseProgress?.current_file;
  if (!file) return "";
  return file.split("/").pop() ?? "";
});

const progressPct = computed(() => {
  const p = projectStore.parseProgress;
  if (!p) return 0;
  return p.percentage;
});

const progressText = computed(() => {
  const p = projectStore.parseProgress;
  if (!p) return "";
  const pct = Math.round(progressPct.value * 100);
  return `${p.parsed_files}/${p.total_files} (${pct}%)`;
});
</script>

<template>
  <div class="status-bar">
    <div class="status-left">
      <!-- 远程连接（仅图标） -->
      <a-tooltip :content="t('project.newRemote')" position="top" mini>
        <span
          role="button"
          tabindex="0"
          class="status-icon-btn"
          @click="openRemoteWizard"
          @keydown.enter.prevent="openRemoteWizard"
        >
          <MdiSsh class="status-ssh-icon" />
        </span>
      </a-tooltip>

      <!-- 解析状态 -->
      <span v-if="isRunning" class="status-item parse-running">
        <icon-loading style="animation: spin 1s linear infinite" />
        <span class="parse-phase">{{ statusLabel }}</span>
        <span class="parse-count">{{ progressText }}</span>
        <span class="parse-file">{{ progressFile }}</span>
        <a-progress
          :percent="projectStore.parseProgress?.percentage ?? 0"
          :show-text="false"
          size="small"
          :status="projectStore.parseProgress?.phase === 'indexing' ? 'warning' : 'normal'"
          class="status-progress"
        />
      </span>

      <span v-else-if="projectStore.parseStatus === 'completed'" class="status-item">
        <icon-check-circle-fill style="color: var(--color-success-6)" />
        {{ statusLabel }}
      </span>

      <span v-else-if="projectStore.parseStatus === 'error'" class="status-item">
        <icon-close-circle-fill style="color: var(--color-danger-6)" />
        {{ statusLabel }}
      </span>

      <span v-else class="status-item">
        <icon-check-circle-fill style="color: var(--color-success-6)" />
        {{ statusLabel }}
      </span>
    </div>

    <div class="status-right">
      <!-- 当前项目 -->
      <span v-if="projectStore.currentProject" class="status-item">
        <icon-folder />
        {{ projectStore.currentProject.name }}
      </span>

      <span
        role="button"
        tabindex="0"
        class="status-icon-btn"
        @click="settings.toggleBottomPanel()"
        @keydown.enter.prevent="settings.toggleBottomPanel()"
      >
        <icon-up v-if="!settings.bottomPanelVisible" />
        <icon-down v-else />
      </span>
      <span class="status-item">v0.1.0</span>
    </div>
  </div>

  <RemoteProjectWizard
    :visible="showRemoteWizard"
    @update:visible="showRemoteWizard = $event"
    @success="onRemoteSuccess"
  />
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 8px;
  font-size: 12px;
  border-top: 1px solid var(--color-border);
  background: var(--color-bg-2);
  color: var(--color-text-3);
}

.status-left,
.status-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-3);
  padding: 0 4px;
  cursor: pointer;
  border-radius: 4px;
  transition: color 0.15s;
}
.status-icon-btn:hover {
  color: var(--color-text-1);
}
.status-ssh-icon {
  font-size: 14px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.parse-running {
  gap: 6px;
}

.parse-phase {
  color: var(--color-text-2);
  font-weight: 500;
}

.parse-count {
  color: var(--color-text-3);
}

.parse-file {
  color: var(--color-text-3);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-progress {
  width: 120px;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
