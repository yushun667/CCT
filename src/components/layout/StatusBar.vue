<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";

const { t } = useI18n();
const settings = useSettingsStore();
</script>

<template>
  <div class="status-bar">
    <div class="status-left">
      <span class="status-item">
        <icon-check-circle-fill style="color: var(--color-success-6)" />
        {{ t("status.ready") }}
      </span>
    </div>

    <div class="status-right">
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
</style>
