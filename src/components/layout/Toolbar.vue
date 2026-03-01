<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { Message } from "@arco-design/web-vue";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "@/stores/settings";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import SettingsDialog from "@/components/settings/SettingsDialog.vue";
import ProjectSettingsDialog from "@/components/project/ProjectSettingsDialog.vue";
import type { Project } from "@/api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

const { t } = useI18n();
const settings = useSettingsStore();
const projectStore = useProjectStore();
const editorStore = useEditorStore();

const showSettings = ref(false);
const showProjectSettings = ref(false);
const settingsProject = ref<Project | null>(null);

const currentProject = computed(() => projectStore.currentProject);

const titleInfo = computed(() => {
  const file = editorStore.activeFile;
  if (!file) {
    const proj = currentProject.value;
    return proj ? proj.name : "CCT";
  }
  return file.fileName;
});

const titleDetail = computed(() => {
  const file = editorStore.activeFile;
  if (!file) return "";
  const proj = currentProject.value;
  if (proj) {
    const rel = file.filePath.replace(proj.source_root, "").replace(/^\//, "");
    return rel || file.filePath;
  }
  return file.filePath;
});

const isRunning = computed(
  () =>
    projectStore.parseStatus === "running" ||
    projectStore.parseStatus === "indexing",
);

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

function handleProjectSettings() {
  if (!currentProject.value) return;
  settingsProject.value = currentProject.value;
  showProjectSettings.value = true;
}

async function handleParse() {
  if (!currentProject.value) return;
  await projectStore.startParse(currentProject.value.id);
  Message.info(t("project.parseStarted"));
}

function handleSettingsSaved() {
  projectStore.fetchProjects();
}

const unlisteners: UnlistenFn[] = [];

onMounted(async () => {
  unlisteners.push(
    await listen("open_directory", () => handleOpenDirectory()),
    await listen("start_parse", () => handleParse()),
    await listen("project_settings", () => handleProjectSettings()),
    await listen("toggle_sidebar", () => settings.toggleSidebar()),
    await listen("toggle_terminal", () => settings.toggleBottomPanel()),
    await listen("toggle_ai", () => settings.toggleAiPanel()),
  );
});

onUnmounted(() => {
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <div class="title-bar">
    <div class="title-left">
      <a-button size="mini" type="text" @click="settings.toggleSidebar()">
        <template #icon><icon-menu /></template>
      </a-button>
    </div>

    <div class="title-center">
      <span class="title-filename">{{ titleInfo }}</span>
      <span v-if="titleDetail" class="title-path">— {{ titleDetail }}</span>
    </div>

    <div class="title-right">
      <a-tooltip :content="t('settings.title')" position="bottom" mini>
        <a-button size="mini" type="text" @click="showSettings = true">
          <template #icon><icon-settings /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip :content="t('sidebar.ai')" position="bottom" mini>
        <a-button
          size="mini"
          type="text"
          :class="{ 'icon-active': settings.aiPanelVisible }"
          @click="settings.toggleAiPanel()"
        >
          <template #icon><icon-robot /></template>
        </a-button>
      </a-tooltip>

      <a-tooltip content="终端 (Ctrl+`)" position="bottom" mini>
        <a-button
          size="mini"
          type="text"
          :class="{ 'icon-active': settings.bottomPanelVisible }"
          @click="settings.toggleBottomPanel()"
        >
          <template #icon><icon-code-square /></template>
        </a-button>
      </a-tooltip>
    </div>

    <SettingsDialog v-model:visible="showSettings" />

    <ProjectSettingsDialog
      v-if="settingsProject"
      v-model:visible="showProjectSettings"
      :project="settingsProject"
      @saved="handleSettingsSaved"
    />
  </div>
</template>

<style scoped>
.title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 8px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-2);
  -webkit-app-region: drag;
  flex-shrink: 0;
}

.title-left,
.title-right {
  display: flex;
  align-items: center;
  gap: 2px;
  -webkit-app-region: no-drag;
}

.title-center {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  justify-content: center;
  overflow: hidden;
  -webkit-app-region: drag;
}

.title-filename {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-1);
  white-space: nowrap;
}

.title-path {
  font-size: 11px;
  color: var(--color-text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.icon-active {
  color: rgb(var(--primary-6));
}
</style>
