<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useSettingsStore, type ThemeMode } from "@/stores/settings";

const { t, locale } = useI18n();
const settings = useSettingsStore();

function onThemeChange(value: ThemeMode) {
  settings.setTheme(value);
}

function onLanguageChange(value: string) {
  settings.setLanguage(value);
  locale.value = value;
}
</script>

<template>
  <div class="settings-view">
    <h3>{{ t("settings.title") }}</h3>

    <a-tabs default-active-key="general">
      <a-tab-pane key="general" :title="t('settings.general')">
        <a-form :model="{}" layout="vertical" style="max-width: 500px">
          <a-form-item :label="t('settings.theme')">
            <a-radio-group
              :model-value="settings.theme"
              @change="onThemeChange($event as ThemeMode)"
            >
              <a-radio value="light">{{ t("settings.themeLight") }}</a-radio>
              <a-radio value="dark">{{ t("settings.themeDark") }}</a-radio>
              <a-radio value="system">{{ t("settings.themeSystem") }}</a-radio>
            </a-radio-group>
          </a-form-item>

          <a-form-item :label="t('settings.language')">
            <a-select
              :model-value="settings.language"
              @change="onLanguageChange($event as string)"
              style="width: 200px"
            >
              <a-option value="zh-CN">中文</a-option>
              <a-option value="en-US">English</a-option>
            </a-select>
          </a-form-item>
        </a-form>
      </a-tab-pane>

      <a-tab-pane key="parse" :title="t('settings.parse')">
        <a-empty description="解析配置（M2 阶段实现）" />
      </a-tab-pane>

      <a-tab-pane key="ai" :title="t('settings.ai')">
        <a-empty description="AI 配置（M6 阶段实现）" />
      </a-tab-pane>

      <a-tab-pane key="about" :title="t('settings.about')">
        <a-descriptions :column="1" bordered>
          <a-descriptions-item label="应用名称">
            {{ t("app.fullName") }}
          </a-descriptions-item>
          <a-descriptions-item label="版本">0.1.0</a-descriptions-item>
          <a-descriptions-item label="框架">Tauri 2 + Vue 3</a-descriptions-item>
          <a-descriptions-item label="解析引擎">Clang LibTooling</a-descriptions-item>
        </a-descriptions>
      </a-tab-pane>
    </a-tabs>
  </div>
</template>

<style scoped>
.settings-view {
  padding: 16px;
}

.settings-view h3 {
  margin-bottom: 16px;
}
</style>
