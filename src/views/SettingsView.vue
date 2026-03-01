<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore, type ThemeMode } from "@/stores/settings";
import { shortcutList } from "@/composables/useKeyboardShortcuts";

const { t, locale } = useI18n();
const settings = useSettingsStore();

function onThemeChange(value: ThemeMode) {
  settings.setTheme(value);
}

function onLanguageChange(value: string) {
  settings.setLanguage(value);
  locale.value = value;
}

/* ── 编辑器设置 ── */
const fontFamily = ref("Menlo, Monaco, 'Courier New', monospace");
const tabSize = ref(4);
const showLineNumbers = ref(true);

/* ── 解析设置 ── */
const maxThreads = ref(4);
const fileExtensions = ref("c, cc, cpp, cxx, h, hh, hpp, hxx");

/* ── 快捷键检测 Mac ── */
const isMac = navigator.platform.toUpperCase().includes("MAC");

const shortcutColumns = [
  { title: t("shortcuts.shortcut"), dataIndex: "keys", width: 200 },
  { title: t("shortcuts.action"), dataIndex: "label" },
  { title: t("shortcuts.category"), dataIndex: "category", width: 140 },
];

const shortcutData = shortcutList.map((s, i) => ({
  key: String(i),
  keys: isMac ? s.keysMac : s.keys,
  label: t(s.labelKey),
  category: t(s.category),
}));
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

      <a-tab-pane key="editor" :title="t('settings.editor')">
        <a-form :model="{}" layout="vertical" style="max-width: 500px">
          <a-form-item :label="t('settings.fontFamily')">
            <a-input v-model="fontFamily" />
          </a-form-item>

          <a-form-item :label="t('settings.tabSize')">
            <a-input-number
              v-model="tabSize"
              :min="1"
              :max="16"
              style="width: 120px"
            />
          </a-form-item>

          <a-form-item :label="t('settings.lineNumbers')">
            <a-switch v-model="showLineNumbers" />
          </a-form-item>
        </a-form>
      </a-tab-pane>

      <a-tab-pane key="parse" :title="t('settings.parse')">
        <a-form :model="{}" layout="vertical" style="max-width: 500px">
          <a-form-item :label="t('settings.maxThreads')">
            <a-input-number
              v-model="maxThreads"
              :min="1"
              :max="64"
              style="width: 120px"
            />
          </a-form-item>

          <a-form-item :label="t('settings.fileExtensions')">
            <a-input
              v-model="fileExtensions"
              :placeholder="t('settings.fileExtensionsPlaceholder')"
            />
          </a-form-item>
        </a-form>
      </a-tab-pane>

      <a-tab-pane key="shortcuts" :title="t('settings.shortcuts')">
        <a-table
          :columns="shortcutColumns"
          :data="shortcutData"
          :pagination="false"
          :bordered="{ cell: true }"
          style="max-width: 700px"
        />
      </a-tab-pane>

      <a-tab-pane key="ai" :title="t('settings.ai')">
        <a-empty :description="t('settings.aiPlaceholder')" />
      </a-tab-pane>

      <a-tab-pane key="about" :title="t('settings.about')">
        <a-descriptions :column="1" bordered>
          <a-descriptions-item :label="t('settings.aboutAppName')">
            {{ t("app.fullName") }}
          </a-descriptions-item>
          <a-descriptions-item :label="t('settings.aboutVersion')">
            0.1.0
          </a-descriptions-item>
          <a-descriptions-item :label="t('settings.aboutFramework')">
            Tauri 2 + Vue 3
          </a-descriptions-item>
          <a-descriptions-item :label="t('settings.aboutEngine')">
            Clang LibTooling
          </a-descriptions-item>
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
