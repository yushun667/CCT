<script setup lang="ts">
/**
 * 全局搜索对话框 — Ctrl+P 快速搜索符号
 *
 * # 设计说明（命令模式）
 * 键盘快捷键触发搜索对话框，用户输入关键词后实时查询后端，
 * 选择结果后触发导航事件。使用防抖避免频繁请求。
 */
import { ref, watch, onMounted, onBeforeUnmount } from "vue";
import { useI18n } from "vue-i18n";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import type { Symbol } from "@/api/types";
import * as queryApi from "@/api/query";

const { t } = useI18n();
const projectStore = useProjectStore();
const editorStore = useEditorStore();

const visible = ref(false);
const searchQuery = ref("");
const results = ref<Symbol[]>([]);
const selectedIndex = ref(0);
const loading = ref(false);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function open() {
  visible.value = true;
  searchQuery.value = "";
  results.value = [];
  selectedIndex.value = 0;
}

function close() {
  visible.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "p") {
    e.preventDefault();
    open();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  if (debounceTimer) clearTimeout(debounceTimer);
});

watch(searchQuery, (query) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  if (!query.trim()) {
    results.value = [];
    return;
  }
  debounceTimer = setTimeout(() => doSearch(query), 200);
});

async function doSearch(query: string) {
  const projectId = projectStore.currentProjectId;
  if (!projectId) return;

  loading.value = true;
  try {
    results.value = await queryApi.searchSymbols(projectId, query, undefined, 30);
    selectedIndex.value = 0;
  } catch {
    results.value = [];
  } finally {
    loading.value = false;
  }
}

function onSelect(symbol: Symbol) {
  editorStore.openFile(symbol.file_path, projectStore.currentProjectId ?? undefined);
  close();
}

function onInputKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    if (selectedIndex.value < results.value.length - 1) {
      selectedIndex.value++;
    }
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (selectedIndex.value > 0) {
      selectedIndex.value--;
    }
  } else if (e.key === "Enter") {
    e.preventDefault();
    if (results.value[selectedIndex.value]) {
      onSelect(results.value[selectedIndex.value]);
    }
  } else if (e.key === "Escape") {
    close();
  }
}

function kindIcon(kind: string): string {
  switch (kind) {
    case "Function":
      return "icon-code";
    case "Variable":
      return "icon-bookmark";
    case "Type":
      return "icon-layers";
    case "Macro":
      return "icon-command";
    default:
      return "icon-file";
  }
}

defineExpose({ open, close });
</script>

<template>
  <a-modal
    v-model:visible="visible"
    :footer="false"
    :closable="false"
    :mask-closable="true"
    unmount-on-close
    class="global-search-modal"
  >
    <div class="search-container">
      <a-input
        v-model="searchQuery"
        :placeholder="t('search.placeholder')"
        size="large"
        allow-clear
        autofocus
        @keydown="onInputKeydown"
      >
        <template #prefix><icon-search /></template>
      </a-input>

      <a-spin :loading="loading" class="results-wrapper">
        <div v-if="results.length === 0 && searchQuery.trim()" class="no-results">
          {{ t("search.noResults") }}
        </div>
        <div v-else class="results-list">
          <div
            v-for="(symbol, idx) in results"
            :key="symbol.id"
            :class="['result-item', { selected: idx === selectedIndex }]"
            @click="onSelect(symbol)"
            @mouseenter="selectedIndex = idx"
          >
            <component :is="kindIcon(symbol.kind)" class="result-icon" />
            <div class="result-info">
              <span class="result-name">{{ symbol.name }}</span>
              <span class="result-qualified">{{ symbol.qualified_name }}</span>
            </div>
            <span class="result-file">
              {{ symbol.file_path.split("/").pop() }}:{{ symbol.line }}
            </span>
          </div>
        </div>
      </a-spin>
    </div>
  </a-modal>
</template>

<style scoped>
.search-container {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.results-wrapper {
  max-height: 400px;
  overflow-y: auto;
}

.no-results {
  padding: 24px;
  text-align: center;
  color: var(--color-text-3);
}

.results-list {
  display: flex;
  flex-direction: column;
}

.result-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.15s;
}

.result-item:hover,
.result-item.selected {
  background: var(--color-fill-2);
}

.result-icon {
  font-size: 16px;
  color: var(--color-text-3);
  flex-shrink: 0;
}

.result-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.result-name {
  font-weight: 500;
  font-size: 14px;
  color: var(--color-text-1);
}

.result-qualified {
  font-size: 12px;
  color: var(--color-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-file {
  font-size: 12px;
  color: var(--color-text-3);
  flex-shrink: 0;
}
</style>
