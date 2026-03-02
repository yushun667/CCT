import { defineStore } from "pinia";
import { ref } from "vue";
import { useI18n } from "vue-i18n";

export type ThemeMode = "light" | "dark" | "system";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemeMode>("dark");
  const language = ref("zh-CN");
  const sidebarWidth = ref(280);
  const sidebarCollapsed = ref(false);
  const bottomPanelHeight = ref(200);
  const bottomPanelVisible = ref(false);
  const bottomPanelTab = ref<"terminal" | "references">("terminal");
  const aiPanelWidth = ref(360);
  const aiPanelVisible = ref(false);

  function setTheme(mode: ThemeMode) {
    theme.value = mode;
  }

  function setLanguage(lang: string) {
    language.value = lang;
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  function toggleBottomPanel() {
    bottomPanelVisible.value = !bottomPanelVisible.value;
  }

  function toggleAiPanel() {
    aiPanelVisible.value = !aiPanelVisible.value;
  }

  return {
    theme,
    language,
    sidebarWidth,
    sidebarCollapsed,
    bottomPanelHeight,
    bottomPanelVisible,
    bottomPanelTab,
    aiPanelWidth,
    aiPanelVisible,
    setTheme,
    setLanguage,
    toggleSidebar,
    toggleBottomPanel,
    toggleAiPanel,
  };
});
