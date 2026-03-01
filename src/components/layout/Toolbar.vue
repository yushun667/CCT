<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import { useTheme } from "@/composables/useTheme";

const { t } = useI18n();
const settings = useSettingsStore();
const { isDark, toggleTheme } = useTheme();
</script>

<template>
  <div class="toolbar">
    <div class="toolbar-left">
      <a-button
        size="small"
        type="text"
        @click="settings.toggleSidebar()"
      >
        <template #icon>
          <icon-menu />
        </template>
      </a-button>
      <span class="toolbar-title">CCT</span>
    </div>

    <div class="toolbar-center">
      <a-button-group size="small">
        <a-button type="text" @click="$router.push('/')">
          <template #icon><icon-apps /></template>
          {{ t("sidebar.analysis") }}
        </a-button>
        <a-button type="text" @click="$router.push('/projects')">
          <template #icon><icon-folder /></template>
          {{ t("sidebar.projects") }}
        </a-button>
      </a-button-group>
    </div>

    <div class="toolbar-right">
      <a-button size="small" type="text" @click="toggleTheme()">
        <template #icon>
          <icon-moon-fill v-if="isDark" />
          <icon-sun-fill v-else />
        </template>
      </a-button>
      <a-button
        size="small"
        type="text"
        @click="settings.toggleAiPanel()"
      >
        <template #icon><icon-robot /></template>
      </a-button>
      <a-button
        size="small"
        type="text"
        @click="$router.push('/settings')"
      >
        <template #icon><icon-settings /></template>
      </a-button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 8px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-2);
  -webkit-app-region: drag;
}

.toolbar-left,
.toolbar-center,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
  -webkit-app-region: no-drag;
}

.toolbar-title {
  font-weight: 600;
  font-size: 14px;
  margin-left: 8px;
  color: var(--color-text-1);
}
</style>
