<script setup lang="ts">
/**
 * 查询结果面板 — 展示调用者/被调用者/引用列表
 *
 * 结果按文件分组显示，点击条目可跳转到对应文件和行。
 */
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import type { CallRelation, ReferenceRelation } from "@/api/types";
import * as queryApi from "@/api/query";

const { t } = useI18n();
const projectStore = useProjectStore();
const editorStore = useEditorStore();

const activeTab = ref<"callers" | "callees" | "references">("callers");
const callers = ref<CallRelation[]>([]);
const callees = ref<CallRelation[]>([]);
const references = ref<ReferenceRelation[]>([]);
const loading = ref(false);
const currentSymbolId = ref<number | null>(null);

const groupedCallers = computed(() => groupByFile(callers.value, "call_site_file"));
const groupedCallees = computed(() => groupByFile(callees.value, "call_site_file"));
const groupedReferences = computed(() =>
  groupByFile(references.value, "reference_file"),
);

function groupByFile<T extends Record<string, unknown>>(
  items: T[],
  fileKey: string,
): Map<string, T[]> {
  const groups = new Map<string, T[]>();
  for (const item of items) {
    const file = String(item[fileKey] ?? "unknown");
    const group = groups.get(file) ?? [];
    group.push(item);
    groups.set(file, group);
  }
  return groups;
}

async function querySymbol(symbolId: number) {
  const projectId = projectStore.currentProjectId;
  if (!projectId) return;

  currentSymbolId.value = symbolId;
  loading.value = true;

  try {
    const [callersResult, calleesResult, refsResult] = await Promise.all([
      queryApi.queryCallers(projectId, symbolId),
      queryApi.queryCallees(projectId, symbolId),
      queryApi.queryReferences(projectId, symbolId),
    ]);
    callers.value = callersResult;
    callees.value = calleesResult;
    references.value = refsResult;
  } catch {
    callers.value = [];
    callees.value = [];
    references.value = [];
  } finally {
    loading.value = false;
  }
}

function navigateToFile(filePath: string, line: number) {
  editorStore.openFile(filePath, projectStore.currentProjectId ?? undefined, line);
}

function shortFileName(path: string): string {
  return path.split("/").pop() ?? path.split("\\").pop() ?? path;
}

defineExpose({ querySymbol });
</script>

<template>
  <div class="result-panel">
    <a-tabs v-model:active-key="activeTab" size="small">
      <a-tab-pane key="callers" :title="t('search.callers')">
        <a-spin :loading="loading">
          <div v-if="callers.length === 0" class="empty">{{ t("search.noResults") }}</div>
          <div v-else class="grouped-results">
            <div
              v-for="[file, items] in groupedCallers"
              :key="file"
              class="file-group"
            >
              <div class="file-header">
                <icon-file />
                <span>{{ shortFileName(file) }}</span>
                <a-badge :count="items.length" />
              </div>
              <div
                v-for="item in items"
                :key="item.id"
                class="result-item"
                @click="navigateToFile(item.call_site_file, item.call_site_line)"
              >
                <span class="line-info">L{{ item.call_site_line }}</span>
                <span class="id-info">caller: {{ item.caller_id }}</span>
              </div>
            </div>
          </div>
        </a-spin>
      </a-tab-pane>

      <a-tab-pane key="callees" :title="t('search.callees')">
        <a-spin :loading="loading">
          <div v-if="callees.length === 0" class="empty">{{ t("search.noResults") }}</div>
          <div v-else class="grouped-results">
            <div
              v-for="[file, items] in groupedCallees"
              :key="file"
              class="file-group"
            >
              <div class="file-header">
                <icon-file />
                <span>{{ shortFileName(file) }}</span>
                <a-badge :count="items.length" />
              </div>
              <div
                v-for="item in items"
                :key="item.id"
                class="result-item"
                @click="navigateToFile(item.call_site_file, item.call_site_line)"
              >
                <span class="line-info">L{{ item.call_site_line }}</span>
                <span class="id-info">callee: {{ item.callee_id }}</span>
              </div>
            </div>
          </div>
        </a-spin>
      </a-tab-pane>

      <a-tab-pane key="references" :title="t('search.references')">
        <a-spin :loading="loading">
          <div v-if="references.length === 0" class="empty">{{ t("search.noResults") }}</div>
          <div v-else class="grouped-results">
            <div
              v-for="[file, items] in groupedReferences"
              :key="file"
              class="file-group"
            >
              <div class="file-header">
                <icon-file />
                <span>{{ shortFileName(file) }}</span>
                <a-badge :count="items.length" />
              </div>
              <div
                v-for="item in items"
                :key="item.id"
                class="result-item"
                @click="navigateToFile(item.reference_file, item.reference_line)"
              >
                <span class="line-info">L{{ item.reference_line }}</span>
                <span class="ref-kind">{{ item.reference_kind }}</span>
              </div>
            </div>
          </div>
        </a-spin>
      </a-tab-pane>
    </a-tabs>
  </div>
</template>

<style scoped>
.result-panel {
  height: 100%;
  overflow: hidden;
}

.empty {
  padding: 24px;
  text-align: center;
  color: var(--color-text-3);
}

.grouped-results {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 400px;
  overflow-y: auto;
  padding: 4px;
}

.file-group {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  overflow: hidden;
}

.file-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  background: var(--color-fill-1);
  font-size: 12px;
  font-weight: 500;
}

.result-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px 4px 24px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.result-item:hover {
  background: var(--color-fill-2);
}

.line-info {
  color: var(--color-text-3);
  min-width: 40px;
}

.id-info,
.ref-kind {
  color: var(--color-text-2);
}
</style>
