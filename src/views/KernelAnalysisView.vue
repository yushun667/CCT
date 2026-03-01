<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

interface SyscallInfo {
  name: string;
  number: number;
  params: string | null;
  definition_file: string;
  definition_line: number;
}

interface IoctlCommand {
  name: string;
  cmd_number: number | null;
  direction: string | null;
  type_char: string | null;
  function_handler: string;
  file: string;
}

interface SymbolInfo {
  id: number;
  name: string;
  qualified_name: string;
  kind: string;
  file_path: string;
  line: number;
}

const activeTab = ref("syscalls");
const projectId = ref("");
const loading = ref(false);
const errorMsg = ref("");

const syscalls = ref<SyscallInfo[]>([]);
const ioctlCommands = ref<IoctlCommand[]>([]);
const callChain = ref<SymbolInfo[]>([]);
const selectedSyscall = ref("");

const syscallColumns = [
  { title: () => t("kernel.syscallName"), dataIndex: "name" },
  { title: () => t("kernel.paramCount"), dataIndex: "number", width: 100 },
  { title: () => t("kernel.params"), dataIndex: "params", ellipsis: true },
  {
    title: () => t("kernel.file"),
    dataIndex: "definition_file",
    ellipsis: true,
  },
  { title: () => t("kernel.line"), dataIndex: "definition_line", width: 80 },
];

const ioctlColumns = [
  { title: () => t("kernel.commandName"), dataIndex: "name" },
  { title: () => t("kernel.handler"), dataIndex: "function_handler" },
  { title: () => t("kernel.direction"), dataIndex: "direction", width: 100 },
  { title: () => t("kernel.file"), dataIndex: "file", ellipsis: true },
];

const hasSyscalls = computed(() => syscalls.value.length > 0);
const hasIoctlCommands = computed(() => ioctlCommands.value.length > 0);
const hasCallChain = computed(() => callChain.value.length > 0);

async function loadSyscalls() {
  if (!projectId.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    syscalls.value = await invoke<SyscallInfo[]>("list_syscalls", {
      projectId: projectId.value,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadIoctlCommands() {
  if (!projectId.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    ioctlCommands.value = await invoke<IoctlCommand[]>(
      "list_ioctl_commands",
      { projectId: projectId.value }
    );
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function traceSyscall() {
  if (!projectId.value || !selectedSyscall.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    callChain.value = await invoke<SymbolInfo[]>("get_syscall_path", {
      projectId: projectId.value,
      syscallName: selectedSyscall.value,
      depth: 5,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

function onSyscallRowClick(record: SyscallInfo) {
  selectedSyscall.value = record.name;
  activeTab.value = "chain";
  traceSyscall();
}

function onTabChange(key: string) {
  activeTab.value = key;
  if (key === "syscalls" && !hasSyscalls.value) loadSyscalls();
  if (key === "ioctl" && !hasIoctlCommands.value) loadIoctlCommands();
}
</script>

<template>
  <div class="kernel-analysis-view">
    <div class="header">
      <h2>{{ t("kernel.title") }}</h2>
      <a-space>
        <a-input
          v-model="projectId"
          :placeholder="t('kernel.projectIdPlaceholder')"
          style="width: 280px"
        />
        <a-button type="primary" :loading="loading" @click="loadSyscalls">
          {{ t("kernel.analyze") }}
        </a-button>
      </a-space>
    </div>

    <a-alert v-if="errorMsg" type="error" :content="errorMsg" closable style="margin-bottom: 16px" />

    <a-tabs :active-key="activeTab" @change="onTabChange">
      <a-tab-pane key="syscalls" :title="t('kernel.tabSyscalls')">
        <a-table
          :columns="syscallColumns"
          :data="syscalls"
          :loading="loading"
          :pagination="{ pageSize: 20 }"
          row-key="name"
          @row-click="onSyscallRowClick"
          stripe
        >
          <template #empty>
            <a-empty :description="t('kernel.noSyscalls')" />
          </template>
        </a-table>
      </a-tab-pane>

      <a-tab-pane key="ioctl" :title="t('kernel.tabIoctl')">
        <a-table
          :columns="ioctlColumns"
          :data="ioctlCommands"
          :loading="loading"
          :pagination="{ pageSize: 20 }"
          row-key="name"
          stripe
        >
          <template #empty>
            <a-empty :description="t('kernel.noIoctl')" />
          </template>
        </a-table>
      </a-tab-pane>

      <a-tab-pane key="chain" :title="t('kernel.tabCallChain')">
        <div class="chain-controls">
          <a-input
            v-model="selectedSyscall"
            :placeholder="t('kernel.syscallNamePlaceholder')"
            style="width: 240px"
          />
          <a-button type="primary" :loading="loading" @click="traceSyscall">
            {{ t("kernel.trace") }}
          </a-button>
        </div>

        <div v-if="hasCallChain" class="call-chain">
          <div
            v-for="(sym, index) in callChain"
            :key="sym.id"
            class="chain-node"
            :style="{ paddingLeft: `${index * 24}px` }"
          >
            <span class="chain-connector" v-if="index > 0">└─</span>
            <a-tag :color="sym.kind === 'Function' ? 'blue' : sym.kind === 'Macro' ? 'green' : 'gray'" size="small">
              {{ sym.kind }}
            </a-tag>
            <span class="chain-name">{{ sym.qualified_name || sym.name }}</span>
            <span class="chain-file">{{ sym.file_path }}:{{ sym.line }}</span>
          </div>
        </div>
        <a-empty v-else :description="t('kernel.noChain')" />
      </a-tab-pane>
    </a-tabs>
  </div>
</template>

<style scoped>
.kernel-analysis-view {
  padding: 20px;
  height: 100%;
  overflow-y: auto;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.chain-controls {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.call-chain {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 12px;
  background: var(--color-bg-2);
}

.chain-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 13px;
}

.chain-connector {
  color: var(--color-text-3);
  user-select: none;
}

.chain-name {
  font-weight: 500;
  color: var(--color-text-1);
}

.chain-file {
  color: var(--color-text-3);
  font-size: 12px;
  margin-left: auto;
}
</style>
