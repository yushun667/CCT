<script setup lang="ts">
import { reactive, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { Message } from "@arco-design/web-vue";
import { invoke } from "@tauri-apps/api/core";
import SshStatusIndicator from "./SshStatusIndicator.vue";
import * as projectApi from "@/api/project";
import type { RemoteFileEntry, SSHConfigParam } from "@/api/project";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: "update:visible", val: boolean): void;
  (e: "success"): void;
}>();

const { t } = useI18n();

const currentStep = ref(0);
const testing = ref(false);
const deploying = ref(false);
const testResult = ref<boolean | null>(null);
const remoteDirs = ref<RemoteFileEntry[]>([]);
const browsingLoading = ref(false);
const sshStatus = ref<"disconnected" | "connecting" | "connected" | "error">(
  "disconnected",
);

/** 当前浏览路径的面包屑（用于 VS Code 风格导航） */
const pathBreadcrumbs = computed(() => {
  const p = form.remoteRoot || "/";
  if (p === "/") return [{ path: "/", label: "/" }];
  const segments = p.split("/").filter(Boolean);
  return segments.map((_, i) => {
    const path = "/" + segments.slice(0, i + 1).join("/");
    return { path, label: segments[i] };
  });
});

/** 上级目录路径（用于「上级目录」行） */
const parentBrowsePath = computed(() => {
  const p = form.remoteRoot || "/";
  if (p === "/") return null;
  const trimmed = p.replace(/\/+$/, "");
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx) || "/";
});

const form = reactive({
  host: "",
  port: 22,
  username: "",
  authMethod: "key" as "key" | "password",
  password: "",
  remoteRoot: "/",
  selectedPath: "",
});

const canNext = computed(() => {
  switch (currentStep.value) {
    case 0: {
      const base = form.host.trim() !== "" && form.username.trim() !== "";
      if (form.authMethod === "password") {
        return base && form.password.trim() !== "";
      }
      return base;
    }
    case 1:
      return testResult.value === true;
    case 2:
      return form.selectedPath.trim() !== "";
    case 3:
      return true;
    default:
      return false;
  }
});

async function handleTestConnection() {
  testing.value = true;
  testResult.value = null;
  sshStatus.value = "connecting";

  try {
    const sshConfig = buildSshConfig();
    const ok = await projectApi.testSshConnectionWithConfig(sshConfig);
    testResult.value = ok;
    sshStatus.value = ok ? "connected" : "error";
    if (ok) {
      Message.success(t("remote.testSuccess"));
    } else {
      Message.error(t("remote.testFailed"));
    }
  } catch (e) {
    testResult.value = false;
    sshStatus.value = "error";
    Message.error(String(e));
  } finally {
    testing.value = false;
  }
}

function buildSshConfig(): SSHConfigParam {
  const authMethod =
    form.authMethod === "key"
      ? {
          Key: {
            key_path: "~/.ssh/id_rsa",
            passphrase_ref: null,
          },
        }
      : { Password: { password_ref: form.password } };

  return {
    host: form.host,
    port: form.port,
    username: form.username,
    auth_method: authMethod,
    key_path: null,
    auth_ref: "",
    proxy_jump: null,
    keep_alive_interval: 30,
    connect_timeout: 15,
    known_hosts_policy: "Accept",
  };
}

async function handleBrowseDir(path: string) {
  browsingLoading.value = true;
  try {
    form.remoteRoot = path;
    const sshConfig = buildSshConfig();
    remoteDirs.value = await invoke<RemoteFileEntry[]>(
      "browse_remote_dir_temp",
      { sshConfig, path },
    );
  } catch (e) {
    Message.error(String(e));
  } finally {
    browsingLoading.value = false;
  }
}

/** 点击目录项：仅进入该目录（不设为选中） */
function handleEnterDir(entry: RemoteFileEntry) {
  if (entry.is_dir) {
    handleBrowseDir(entry.path);
  }
}

/** 将当前浏览路径设为选中的项目根目录 */
function handleSelectCurrentFolder() {
  form.selectedPath = form.remoteRoot || "/";
}

async function handleDeploy() {
  deploying.value = true;
  try {
    const sshConfig = buildSshConfig();
    await invoke("deploy_agent_temp", { sshConfig });
    Message.success(t("remote.deploySuccess"));
  } catch (e) {
    Message.error(String(e));
  } finally {
    deploying.value = false;
  }
}

function handleNext() {
  if (currentStep.value === 1 && testResult.value !== true) return;

  if (currentStep.value === 1) {
    form.remoteRoot = "/";
    handleBrowseDir("/");
  }

  if (currentStep.value < 3) {
    currentStep.value++;
  }
}

function handlePrev() {
  if (currentStep.value > 0) {
    currentStep.value--;
  }
}

function defaultProjectName(): string {
  const path = form.selectedPath || form.remoteRoot;
  const segment = path.split("/").filter(Boolean).pop() || "root";
  return `${form.host}_${segment}`;
}

async function handleFinish() {
  try {
    const sshConfig = buildSshConfig();
    const sourceRoot = form.selectedPath || form.remoteRoot;
    await projectApi.createRemoteProject(
      defaultProjectName(),
      sourceRoot,
      sshConfig,
    );
    Message.success(t("remote.createSuccess") || "远程项目创建成功");
    emit("success");
    handleClose();
  } catch (e) {
    Message.error(String(e));
  }
}

function handleClose() {
  currentStep.value = 0;
  testing.value = false;
  deploying.value = false;
  testResult.value = null;
  sshStatus.value = "disconnected";
  remoteDirs.value = [];
  form.host = "";
  form.port = 22;
  form.username = "";
  form.authMethod = "key";
  form.password = "";
  form.remoteRoot = "/";
  form.selectedPath = "";
  emit("update:visible", false);
}
</script>

<template>
  <a-modal
    :visible="props.visible"
    :title="t('remote.wizardTitle')"
    :width="640"
    :footer="false"
    unmount-on-close
    @cancel="handleClose"
    @update:visible="emit('update:visible', $event)"
  >
    <a-steps :current="currentStep + 1" size="small" style="margin-bottom: 24px">
      <a-step :title="t('remote.stepConnection')" />
      <a-step :title="t('remote.stepTest')" />
      <a-step :title="t('remote.stepDirectory')" />
      <a-step :title="t('remote.stepDeploy')" />
    </a-steps>

    <!-- Step 1: SSH 连接信息 -->
    <div v-show="currentStep === 0">
      <a-form :model="form" layout="vertical">
        <a-form-item :label="t('remote.host')" required>
          <a-input
            v-model="form.host"
            :placeholder="t('remote.hostPlaceholder')"
          />
        </a-form-item>
        <a-form-item :label="t('remote.port')">
          <a-input-number
            v-model="form.port"
            :min="1"
            :max="65535"
            style="width: 120px"
          />
        </a-form-item>
        <a-form-item :label="t('remote.username')" required>
          <a-input
            v-model="form.username"
            :placeholder="t('remote.usernamePlaceholder')"
            autocapitalize="off"
            autocomplete="username"
          />
        </a-form-item>
        <a-form-item :label="t('remote.authMethod')">
          <a-radio-group v-model="form.authMethod">
            <a-radio value="key">{{ t("remote.authKey") }}</a-radio>
            <a-radio value="password">{{ t("remote.authPassword") }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item
          v-if="form.authMethod === 'password'"
          :label="t('remote.password')"
          required
        >
          <a-input-password
            v-model="form.password"
            :placeholder="t('remote.passwordPlaceholder')"
            autocomplete="current-password"
          />
        </a-form-item>
      </a-form>
    </div>

    <!-- Step 2: 连接测试 -->
    <div v-show="currentStep === 1">
      <div class="test-section">
        <div class="test-info">
          <a-descriptions :column="1" size="small" bordered>
            <a-descriptions-item :label="t('remote.host')">
              {{ form.host }}
            </a-descriptions-item>
            <a-descriptions-item :label="t('remote.port')">
              {{ form.port }}
            </a-descriptions-item>
            <a-descriptions-item :label="t('remote.username')">
              {{ form.username }}
            </a-descriptions-item>
            <a-descriptions-item :label="t('remote.authMethod')">
              {{
                form.authMethod === "key"
                  ? t("remote.authKey")
                  : t("remote.authPassword")
              }}
            </a-descriptions-item>
          </a-descriptions>
        </div>

        <div class="test-action">
          <a-space direction="vertical" align="center" fill>
            <SshStatusIndicator :status="sshStatus" :size="12" show-label />
            <a-button
              type="primary"
              :loading="testing"
              @click="handleTestConnection"
            >
              {{ t("remote.testConnection") }}
            </a-button>
            <a-result
              v-if="testResult === true"
              status="success"
              :title="t('remote.testSuccess')"
              style="padding: 12px 0"
            />
            <a-result
              v-if="testResult === false"
              status="error"
              :title="t('remote.testFailed')"
              style="padding: 12px 0"
            />
          </a-space>
        </div>
      </div>
    </div>

    <!-- Step 3: 远程目录选择（VS Code 风格：浏览 + 面包屑 + 选择此目录） -->
    <div v-show="currentStep === 2">
      <p class="browse-hint">{{ t("remote.browseFolderHint") }}</p>

      <!-- 面包屑 -->
      <div class="breadcrumb-wrap">
        <a-breadcrumb>
          <a-breadcrumb-item
            v-for="(crumb, idx) in pathBreadcrumbs"
            :key="crumb.path"
          >
            <span
              class="breadcrumb-link"
              @click="handleBrowseDir(crumb.path)"
            >
              {{ crumb.label }}
            </span>
          </a-breadcrumb-item>
        </a-breadcrumb>
      </div>

      <!-- 当前路径 + 选择此目录 -->
      <div class="current-path-row">
        <span class="current-path-label">{{ t("remote.remotePath") }}:</span>
        <span class="current-path-value">{{ form.remoteRoot || "/" }}</span>
        <a-button type="primary" size="small" @click="handleSelectCurrentFolder">
          {{ t("remote.selectThisFolder") }}
        </a-button>
      </div>

      <div v-if="form.selectedPath" class="selected-path">
        <a-tag color="arcoblue">{{ form.selectedPath }}</a-tag>
      </div>

      <a-spin :loading="browsingLoading" class="dir-list-spin">
        <a-list :bordered="false" size="small" class="dir-list">
          <a-list-item
            v-if="parentBrowsePath !== null"
            class="dir-item dir-item-parent"
            @click="handleBrowseDir(parentBrowsePath!)"
          >
            <template #meta>
              <a-list-item-meta>
                <template #avatar>
                  <icon-folder style="color: var(--color-text-3)" />
                </template>
                <template #title>
                  <span class="dir-name">{{ t("remote.parentFolder") }}</span>
                </template>
              </a-list-item-meta>
            </template>
          </a-list-item>
          <a-list-item
            v-for="entry in remoteDirs"
            :key="entry.path"
            class="dir-item"
            @click="handleEnterDir(entry)"
          >
            <template #meta>
              <a-list-item-meta>
                <template #avatar>
                  <icon-folder
                    v-if="entry.is_dir"
                    style="color: var(--color-primary-light-4)"
                  />
                  <icon-file v-else style="color: var(--color-text-3)" />
                </template>
                <template #title>
                  <span :class="{ 'dir-name': entry.is_dir }">{{ entry.name }}</span>
                </template>
              </a-list-item-meta>
            </template>
          </a-list-item>
        </a-list>
      </a-spin>
    </div>

    <!-- Step 4: Agent 部署 -->
    <div v-show="currentStep === 3">
      <a-space direction="vertical" fill>
        <a-alert type="info">
          {{ t("remote.deployInfo") }}
        </a-alert>

        <a-descriptions :column="1" size="small" bordered>
          <a-descriptions-item :label="t('remote.host')">
            {{ form.host }}:{{ form.port }}
          </a-descriptions-item>
          <a-descriptions-item :label="t('remote.remotePath')">
            {{ form.selectedPath || form.remoteRoot }}
          </a-descriptions-item>
        </a-descriptions>

        <a-button
          type="primary"
          :loading="deploying"
          long
          @click="handleDeploy"
        >
          {{ t("remote.deployAgent") }}
        </a-button>
      </a-space>
    </div>

    <!-- 底部导航 -->
    <div class="wizard-footer">
      <a-button v-if="currentStep > 0" @click="handlePrev">
        {{ t("common.back") }}
      </a-button>
      <div style="flex: 1" />
      <a-space>
        <a-button @click="handleClose">
          {{ t("common.cancel") }}
        </a-button>
        <a-button
          v-if="currentStep < 3"
          type="primary"
          :disabled="!canNext"
          @click="handleNext"
        >
          {{ t("common.next") }}
        </a-button>
        <a-button
          v-if="currentStep === 3"
          type="primary"
          @click="handleFinish"
        >
          {{ t("common.finish") }}
        </a-button>
      </a-space>
    </div>
  </a-modal>
</template>

<style scoped>
.test-section {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.test-action {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}

.browse-hint {
  color: var(--color-text-3);
  font-size: 12px;
  margin-bottom: 12px;
}

.breadcrumb-wrap {
  margin-bottom: 12px;
}

.breadcrumb-link {
  cursor: pointer;
  color: var(--color-primary-6);
}
.breadcrumb-link:hover {
  text-decoration: underline;
}

.current-path-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.current-path-label {
  color: var(--color-text-2);
  font-size: 12px;
}

.current-path-value {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.selected-path {
  margin-bottom: 8px;
}

.dir-list-spin {
  min-height: 200px;
}

.dir-list {
  max-height: 300px;
  overflow-y: auto;
}

.dir-item-parent {
  color: var(--color-text-3);
}

.dir-item {
  cursor: pointer;
}

.dir-item:hover {
  background: var(--color-fill-1);
}

.dir-name {
  font-weight: 500;
  color: var(--color-primary-light-4);
}

.wizard-footer {
  display: flex;
  align-items: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--color-border);
}
</style>
