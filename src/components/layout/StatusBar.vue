<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import { useProjectStore } from "@/stores/project";

const { t } = useI18n();
const settings = useSettingsStore();
const projectStore = useProjectStore();

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

const progressText = computed(() => {
  const p = projectStore.parseProgress;
  if (!p) return "";
  return `${p.parsed_files}/${p.total_files}`;
});
</script>

<template>
  <div class="status-bar">
    <div class="status-left">
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

      <a-button
        size="mini"
        type="text"
        @click="settings.toggleBottomPanel()"
      >
        <template #icon>
          <icon-up v-if="!settings.bottomPanelVisible" />
          <icon-down v-else />
        </template>
      </a-button>
      <span class="status-item">v0.1.0</span>
    </div>
  </div>
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
