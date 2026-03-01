<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import Sidebar from "./Sidebar.vue";
import Toolbar from "./Toolbar.vue";
import StatusBar from "./StatusBar.vue";
import AiPanel from "@/components/ai/AiPanel.vue";

const { t } = useI18n();
const settings = useSettingsStore();
</script>

<template>
  <a-layout class="main-layout">
    <Toolbar />

    <a-layout class="content-layout">
      <a-layout-sider
        :width="settings.sidebarWidth"
        :collapsed="settings.sidebarCollapsed"
        :collapsed-width="48"
        collapsible
        :trigger="null"
        class="sidebar-sider"
      >
        <Sidebar />
      </a-layout-sider>

      <a-layout class="work-area">
        <a-layout-content class="main-content">
          <router-view />
        </a-layout-content>

        <a-layout-sider
          v-if="settings.aiPanelVisible"
          :width="settings.aiPanelWidth"
          class="ai-panel-sider"
        >
          <AiPanel />
        </a-layout-sider>
      </a-layout>

      <div
        v-if="settings.bottomPanelVisible"
        class="bottom-panel"
        :style="{ height: settings.bottomPanelHeight + 'px' }"
      >
        <div class="bottom-panel-content">
          <a-tabs default-active-key="output" size="small">
            <a-tab-pane key="output" title="输出" />
            <a-tab-pane key="problems" title="问题" />
            <a-tab-pane key="parse" :title="t('parse.status')" />
          </a-tabs>
        </div>
      </div>
    </a-layout>

    <StatusBar />
  </a-layout>
</template>

<style scoped>
.main-layout {
  height: 100vh;
  overflow: hidden;
}

.content-layout {
  flex: 1;
  overflow: hidden;
}

.sidebar-sider {
  border-right: 1px solid var(--color-border);
}

.work-area {
  flex: 1;
  flex-direction: row !important;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow: auto;
  background: var(--color-bg-1);
}

.ai-panel-sider {
  border-left: 1px solid var(--color-border);
}

.bottom-panel {
  border-top: 1px solid var(--color-border);
  overflow: hidden;
}

.bottom-panel-content {
  height: 100%;
  padding: 0 8px;
}
</style>
