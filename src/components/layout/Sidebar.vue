<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";

const { t } = useI18n();
const settings = useSettingsStore();
const activeTab = ref("files");
</script>

<template>
  <div class="sidebar">
    <div v-if="!settings.sidebarCollapsed" class="sidebar-content">
      <a-menu
        :selected-keys="[activeTab]"
        mode="vertical"
        @menu-item-click="(key: string) => activeTab = key"
      >
        <a-menu-item key="files">
          <template #icon><icon-folder /></template>
          {{ t("sidebar.files") }}
        </a-menu-item>
        <a-menu-item key="search">
          <template #icon><icon-search /></template>
          {{ t("sidebar.search") }}
        </a-menu-item>
        <a-menu-item key="analysis">
          <template #icon><icon-mind-mapping /></template>
          {{ t("sidebar.analysis") }}
        </a-menu-item>
      </a-menu>

      <div class="sidebar-panel">
        <div v-if="activeTab === 'files'" class="panel-content">
          <a-empty :description="t('sidebar.files')" />
        </div>
        <div v-else-if="activeTab === 'search'" class="panel-content">
          <a-input
            :placeholder="t('search.placeholder')"
            allow-clear
            size="small"
          >
            <template #prefix><icon-search /></template>
          </a-input>
        </div>
        <div v-else-if="activeTab === 'analysis'" class="panel-content">
          <a-empty :description="t('sidebar.analysis')" />
        </div>
      </div>
    </div>

    <div v-else class="sidebar-collapsed">
      <a-tooltip :content="t('sidebar.files')" position="right">
        <a-button type="text" @click="activeTab = 'files'; settings.toggleSidebar()">
          <template #icon><icon-folder /></template>
        </a-button>
      </a-tooltip>
      <a-tooltip :content="t('sidebar.search')" position="right">
        <a-button type="text" @click="activeTab = 'search'; settings.toggleSidebar()">
          <template #icon><icon-search /></template>
        </a-button>
      </a-tooltip>
      <a-tooltip :content="t('sidebar.analysis')" position="right">
        <a-button type="text" @click="activeTab = 'analysis'; settings.toggleSidebar()">
          <template #icon><icon-mind-mapping /></template>
        </a-button>
      </a-tooltip>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.sidebar-content {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sidebar-panel {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

.panel-content {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sidebar-collapsed {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 8px;
  gap: 4px;
}
</style>
