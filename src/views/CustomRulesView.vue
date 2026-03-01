<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

interface CustomRule {
  name: string;
  description: string;
  pattern: {
    symbol_kind: string | null;
    name_regex: string;
    file_pattern: string | null;
    has_attribute: string | null;
  };
  severity: string;
  action: string;
}

interface RuleMatch {
  rule_name: string;
  symbol_name: string;
  file_path: string;
  line: number;
  message: string;
}

const projectId = ref("");
const rulesPath = ref("");
const loading = ref(false);
const errorMsg = ref("");

const rules = ref<CustomRule[]>([]);
const matches = ref<RuleMatch[]>([]);

const yamlContent = ref(`# CCT 自定义分析规则示例
- name: "global_function_prefix"
  description: "全局函数应以模块名为前缀"
  pattern:
    symbol_kind: "function"
    name_regex: "^[a-z]+_.*"
    file_pattern: null
    has_attribute: null
  severity: "Info"
  action: "Report"

- name: "unsafe_strcpy_usage"
  description: "不应使用 strcpy，请使用 strncpy 或 strlcpy"
  pattern:
    symbol_kind: "function"
    name_regex: "^strcpy$"
    file_pattern: null
    has_attribute: null
  severity: "Warning"
  action: "Highlight"
`);

const hasRules = computed(() => rules.value.length > 0);
const hasMatches = computed(() => matches.value.length > 0);

const matchColumns = [
  { title: () => t("rules.ruleName"), dataIndex: "rule_name", width: 180 },
  { title: () => t("rules.symbolName"), dataIndex: "symbol_name", width: 200 },
  { title: () => t("rules.filePath"), dataIndex: "file_path", ellipsis: true },
  { title: () => t("rules.lineNumber"), dataIndex: "line", width: 80 },
  { title: () => t("rules.message"), dataIndex: "message", ellipsis: true },
];

function severityColor(severity: string): string {
  switch (severity) {
    case "Error":
      return "red";
    case "Warning":
      return "orange";
    default:
      return "blue";
  }
}

async function loadRulesFromFile() {
  if (!rulesPath.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    rules.value = await invoke<CustomRule[]>("load_custom_rules", {
      projectId: projectId.value || "default",
      rulesPath: rulesPath.value,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function applyRules() {
  if (!projectId.value || !rulesPath.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    matches.value = await invoke<RuleMatch[]>("apply_custom_rules", {
      projectId: projectId.value,
      rulesPath: rulesPath.value,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

function removeRule(index: number) {
  rules.value.splice(index, 1);
}
</script>

<template>
  <div class="custom-rules-view">
    <div class="header">
      <h2>{{ t("rules.title") }}</h2>
    </div>

    <a-alert v-if="errorMsg" type="error" :content="errorMsg" closable style="margin-bottom: 16px" />

    <div class="controls">
      <a-space direction="vertical" fill style="width: 100%">
        <a-space>
          <a-input
            v-model="projectId"
            :placeholder="t('rules.projectIdPlaceholder')"
            style="width: 280px"
          />
          <a-input
            v-model="rulesPath"
            :placeholder="t('rules.rulesPathPlaceholder')"
            style="width: 360px"
          />
          <a-button @click="loadRulesFromFile" :loading="loading">
            {{ t("rules.load") }}
          </a-button>
          <a-button type="primary" @click="applyRules" :loading="loading">
            {{ t("rules.apply") }}
          </a-button>
        </a-space>
      </a-space>
    </div>

    <div class="main-content">
      <div class="left-panel">
        <h3>{{ t("rules.editorTitle") }}</h3>
        <a-textarea
          v-model="yamlContent"
          :auto-size="{ minRows: 15, maxRows: 30 }"
          class="yaml-editor"
          :placeholder="t('rules.yamlPlaceholder')"
        />

        <div v-if="hasRules" class="rule-list">
          <h3>{{ t("rules.loadedRules") }}</h3>
          <div
            v-for="(rule, index) in rules"
            :key="rule.name"
            class="rule-card"
          >
            <div class="rule-header">
              <a-tag :color="severityColor(rule.severity)" size="small">
                {{ rule.severity }}
              </a-tag>
              <span class="rule-name">{{ rule.name }}</span>
              <a-button
                size="mini"
                type="text"
                status="danger"
                @click="removeRule(index)"
              >
                {{ t("common.delete") }}
              </a-button>
            </div>
            <div class="rule-desc">{{ rule.description }}</div>
            <div class="rule-detail">
              <code>{{ rule.pattern.name_regex }}</code>
              <span v-if="rule.pattern.symbol_kind" class="rule-kind">
                {{ rule.pattern.symbol_kind }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div class="right-panel">
        <h3>
          {{ t("rules.results") }}
          <a-badge v-if="hasMatches" :count="matches.length" />
        </h3>
        <a-table
          :columns="matchColumns"
          :data="matches"
          :loading="loading"
          :pagination="{ pageSize: 20 }"
          row-key="symbol_name"
          stripe
          size="small"
        >
          <template #empty>
            <a-empty :description="t('rules.noResults')" />
          </template>
        </a-table>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-rules-view {
  padding: 20px;
  height: 100%;
  overflow-y: auto;
}

.header {
  margin-bottom: 16px;
}

.header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.controls {
  margin-bottom: 20px;
}

.main-content {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.left-panel h3,
.right-panel h3 {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 8px;
}

.yaml-editor {
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 13px;
  margin-bottom: 16px;
}

.rule-list {
  margin-top: 16px;
}

.rule-card {
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  margin-bottom: 8px;
  background: var(--color-bg-2);
}

.rule-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rule-name {
  font-weight: 500;
  font-size: 14px;
  flex: 1;
}

.rule-desc {
  color: var(--color-text-2);
  font-size: 13px;
  margin-top: 4px;
}

.rule-detail {
  margin-top: 6px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.rule-detail code {
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 12px;
  padding: 2px 6px;
  background: var(--color-fill-2);
  border-radius: 3px;
}

.rule-kind {
  font-size: 12px;
  color: var(--color-text-3);
}
</style>
