<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useProjectStore } from "@/stores/project";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Command } from "@tauri-apps/plugin-shell";
import "@xterm/xterm/css/xterm.css";

const { t } = useI18n();
const projectStore = useProjectStore();

const activeTab = ref<string>("terminal");
const terminalRef = ref<HTMLElement | null>(null);
const logContent = ref<string[]>([]);

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let shellProcess: Awaited<ReturnType<typeof Command.create>> extends { spawn(): infer R } ? Awaited<R> : unknown;

const MAX_LOG_LINES = 2000;

function appendLog(line: string) {
  logContent.value.push(line);
  if (logContent.value.length > MAX_LOG_LINES) {
    logContent.value = logContent.value.slice(-MAX_LOG_LINES);
  }
}

async function initTerminal() {
  if (!terminalRef.value) return;

  terminal = new Terminal({
    fontSize: 13,
    fontFamily: "Menlo, Monaco, 'Courier New', monospace",
    theme: {
      background: "#1e1e1e",
      foreground: "#cccccc",
      cursor: "#ffffff",
    },
    cursorBlink: true,
    scrollback: 5000,
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(terminalRef.value);

  await nextTick();
  fitAddon.fit();

  try {
    const cwd = projectStore.currentProject?.source_root ?? undefined;
    const cmd = await Command.create("exec-zsh", ["-l"], {
      cwd,
      encoding: "utf-8",
    });

    cmd.stdout.on("data", (data: string) => {
      terminal?.write(data);
    });
    cmd.stderr.on("data", (data: string) => {
      terminal?.write(data);
    });
    cmd.on("close", () => {
      terminal?.write("\r\n[Process exited]\r\n");
    });

    const child = await cmd.spawn();
    shellProcess = child as typeof shellProcess;

    terminal.onData((data: string) => {
      (shellProcess as { write(data: string): void })?.write(data);
    });
  } catch (err) {
    terminal.write(`\r\nFailed to start shell: ${err}\r\n`);
  }

  const resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit();
  });
  resizeObserver.observe(terminalRef.value);
}

function clearLog() {
  logContent.value = [];
}

onMounted(() => {
  if (activeTab.value === "terminal") {
    initTerminal();
  }
});

onBeforeUnmount(() => {
  terminal?.dispose();
  terminal = null;
  try {
    (shellProcess as { kill(): void })?.kill();
  } catch {
    // ignore
  }
});

watch(activeTab, (tab) => {
  if (tab === "terminal" && !terminal) {
    nextTick(() => initTerminal());
  }
  if (tab === "terminal" && terminal) {
    nextTick(() => fitAddon?.fit());
  }
});

watch(
  () => projectStore.parseProgress,
  (prog) => {
    if (prog) {
      const ts = new Date().toLocaleTimeString();
      appendLog(
        `[${ts}] [${prog.phase}] ${prog.parsed_files}/${prog.total_files} — ${prog.current_file?.split("/").pop() ?? ""}`,
      );
    }
  },
);

watch(
  () => projectStore.parseStatus,
  (status) => {
    const ts = new Date().toLocaleTimeString();
    if (status === "completed") {
      appendLog(`[${ts}] ✓ 解析完成`);
    } else if (status === "error") {
      appendLog(`[${ts}] ✗ 解析失败: ${projectStore.error ?? ""}`);
    }
  },
);
</script>

<template>
  <div class="terminal-panel">
    <div class="panel-header">
      <a-tabs v-model:active-key="activeTab" size="mini" type="text">
        <a-tab-pane key="terminal" :title="t('terminal.title')" />
        <a-tab-pane key="log" :title="t('terminal.log')" />
        <a-tab-pane key="progress" :title="t('terminal.parseProgress')" />
      </a-tabs>

      <div class="panel-actions">
        <a-button
          v-if="activeTab === 'log'"
          size="mini"
          type="text"
          @click="clearLog"
        >
          <template #icon><icon-delete /></template>
        </a-button>
      </div>
    </div>

    <div class="panel-body">
      <!-- 终端 -->
      <div v-show="activeTab === 'terminal'" ref="terminalRef" class="terminal-container" />

      <!-- 日志 -->
      <div v-show="activeTab === 'log'" class="log-container">
        <div v-if="logContent.length === 0" class="log-empty">
          <span>{{ t("terminal.log") }} — 暂无日志</span>
        </div>
        <div v-for="(line, idx) in logContent" :key="idx" class="log-line">
          {{ line }}
        </div>
      </div>

      <!-- 解析进度 -->
      <div v-show="activeTab === 'progress'" class="progress-container">
        <template v-if="projectStore.parseStatus === 'running' || projectStore.parseStatus === 'indexing'">
          <a-progress
            :percent="projectStore.parseProgress?.percentage ?? 0"
            :status="projectStore.parseProgress?.phase === 'indexing' ? 'warning' : 'normal'"
          />
          <div class="progress-detail">
            <span>{{ projectStore.parseProgress?.phase ?? "" }}</span>
            <span>
              {{ projectStore.parseProgress?.parsed_files ?? 0 }} /
              {{ projectStore.parseProgress?.total_files ?? 0 }}
            </span>
            <span class="progress-file">
              {{ projectStore.parseProgress?.current_file?.split("/").pop() ?? "" }}
            </span>
          </div>
        </template>
        <template v-else-if="projectStore.parseStatus === 'completed'">
          <a-result status="success" :title="t('parse.completed')" />
        </template>
        <template v-else>
          <div class="log-empty">
            <span>{{ t("parse.notStarted") }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.panel-header :deep(.arco-tabs-nav) {
  height: 30px;
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.panel-body {
  flex: 1;
  overflow: hidden;
}

.terminal-container {
  width: 100%;
  height: 100%;
  padding: 4px;
}

.log-container {
  height: 100%;
  overflow: auto;
  padding: 8px;
  font-size: 12px;
  font-family: Menlo, Monaco, "Courier New", monospace;
  color: var(--color-text-2);
}

.log-line {
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--color-text-3);
}

.progress-container {
  padding: 16px;
  height: 100%;
  overflow: auto;
}

.progress-detail {
  display: flex;
  gap: 16px;
  margin-top: 8px;
  font-size: 12px;
  color: var(--color-text-2);
}

.progress-file {
  color: var(--color-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
</style>
