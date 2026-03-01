<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

interface MessageCode {
  code: number;
  name: string;
  handler_function: string;
}

interface IpcService {
  service_name: string;
  stub_class: string | null;
  proxy_class: string | null;
  message_codes: MessageCode[];
}

interface SymbolInfo {
  id: number;
  name: string;
  qualified_name: string;
  kind: string;
  file_path: string;
  line: number;
}

const projectId = ref("");
const loading = ref(false);
const errorMsg = ref("");

const services = ref<IpcService[]>([]);
const callPath = ref<SymbolInfo[]>([]);
const selectedService = ref("");

const hasServices = computed(() => services.value.length > 0);
const hasCallPath = computed(() => callPath.value.length > 0);

async function loadServices() {
  if (!projectId.value) return;
  loading.value = true;
  errorMsg.value = "";
  try {
    services.value = await invoke<IpcService[]>("list_ipc_services", {
      projectId: projectId.value,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function traceIpcCall(serviceName: string) {
  if (!projectId.value) return;
  selectedService.value = serviceName;
  loading.value = true;
  errorMsg.value = "";
  try {
    callPath.value = await invoke<SymbolInfo[]>("get_ipc_call_path", {
      projectId: projectId.value,
      serviceName,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="ipc-analysis-view">
    <div class="header">
      <h2>{{ t("ipc.title") }}</h2>
      <a-space>
        <a-input
          v-model="projectId"
          :placeholder="t('ipc.projectIdPlaceholder')"
          style="width: 280px"
        />
        <a-button type="primary" :loading="loading" @click="loadServices">
          {{ t("ipc.analyze") }}
        </a-button>
      </a-space>
    </div>

    <a-alert v-if="errorMsg" type="error" :content="errorMsg" closable style="margin-bottom: 16px" />

    <div class="content-layout">
      <div class="service-list">
        <h3>{{ t("ipc.serviceList") }}</h3>
        <a-spin :loading="loading" style="width: 100%">
          <div v-if="hasServices" class="services">
            <a-collapse :default-active-key="[]">
              <a-collapse-item
                v-for="svc in services"
                :key="svc.service_name"
                :header="svc.service_name"
              >
                <template #extra>
                  <a-button
                    size="mini"
                    type="text"
                    @click.stop="traceIpcCall(svc.service_name)"
                  >
                    {{ t("ipc.trace") }}
                  </a-button>
                </template>

                <a-descriptions :column="1" size="small" bordered>
                  <a-descriptions-item :label="t('ipc.stubClass')">
                    {{ svc.stub_class || "-" }}
                  </a-descriptions-item>
                  <a-descriptions-item :label="t('ipc.proxyClass')">
                    {{ svc.proxy_class || "-" }}
                  </a-descriptions-item>
                </a-descriptions>

                <div v-if="svc.message_codes.length > 0" class="message-codes">
                  <h4>{{ t("ipc.messageCodes") }}</h4>
                  <a-table
                    :data="svc.message_codes"
                    :columns="[
                      { title: t('ipc.code'), dataIndex: 'code', width: 80 },
                      { title: t('ipc.codeName'), dataIndex: 'name' },
                      { title: t('ipc.handler'), dataIndex: 'handler_function', ellipsis: true },
                    ]"
                    :pagination="false"
                    size="small"
                    row-key="code"
                  />
                </div>
              </a-collapse-item>
            </a-collapse>
          </div>
          <a-empty v-else :description="t('ipc.noServices')" />
        </a-spin>
      </div>

      <div class="path-panel">
        <h3>
          {{ t("ipc.communicationPath") }}
          <span v-if="selectedService" class="path-service-name">
            — {{ selectedService }}
          </span>
        </h3>
        <div v-if="hasCallPath" class="call-path">
          <div
            v-for="(sym, index) in callPath"
            :key="sym.id"
            class="path-node"
          >
            <div class="path-index">{{ index + 1 }}</div>
            <div class="path-info">
              <div class="path-name">
                <a-tag
                  :color="sym.kind === 'Function' ? 'arcoblue' : sym.kind === 'Type' ? 'purple' : 'gray'"
                  size="small"
                >
                  {{ sym.kind }}
                </a-tag>
                {{ sym.qualified_name || sym.name }}
              </div>
              <div class="path-file">{{ sym.file_path }}:{{ sym.line }}</div>
            </div>
          </div>
        </div>
        <a-empty v-else :description="t('ipc.noPath')" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.ipc-analysis-view {
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

.content-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  min-height: 0;
}

.service-list h3,
.path-panel h3 {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 500;
}

.path-service-name {
  color: var(--color-text-3);
  font-weight: 400;
  font-size: 13px;
}

.message-codes {
  margin-top: 12px;
}

.message-codes h4 {
  margin: 0 0 8px;
  font-size: 13px;
  font-weight: 500;
}

.call-path {
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px;
  background: var(--color-bg-2);
}

.path-node {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px;
  border-bottom: 1px solid var(--color-border-2);
}

.path-node:last-child {
  border-bottom: none;
}

.path-index {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--color-primary-light-1);
  color: var(--color-primary-6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
}

.path-info {
  flex: 1;
  min-width: 0;
}

.path-name {
  font-family: "SF Mono", "Fira Code", monospace;
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 6px;
}

.path-file {
  color: var(--color-text-3);
  font-size: 12px;
  margin-top: 2px;
}
</style>
