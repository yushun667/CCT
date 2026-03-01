<script setup lang="ts">
import { reactive, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { Message } from "@arco-design/web-vue";
import SshStatusIndicator from "./SshStatusIndicator.vue";
import * as projectApi from "@/api/project";
import type { RemoteFileEntry } from "@/api/project";

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
const sshStatus = ref<"disconnected" | "connecting" | "connected" | "error">(
  "disconnected",
);

const form = reactive({
  name: "",
  host: "",
  port: 22,
  username: "",
  authMethod: "key" as "key" | "password" | "agent",
  keyPath: "",
  remoteRoot: "/home",
  selectedPath: "",
});

const canNext = computed(() => {
  switch (currentStep.value) {
    case 0:
      return (
        form.name.trim() !== "" &&
        form.host.trim() !== "" &&
        form.username.trim() !== ""
      );
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
    const ok = await projectApi.testSshConnection(
      form.host,
      form.port,
      form.username,
    );
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

async function handleBrowseDir(path: string) {
  try {
    form.remoteRoot = path;
    remoteDirs.value = await projectApi.browseRemoteDir("", path);
  } catch (e) {
    Message.error(String(e));
  }
}

function handleSelectPath(entry: RemoteFileEntry) {
  if (entry.is_dir) {
    form.selectedPath = entry.path;
    handleBrowseDir(entry.path);
  }
}

async function handleDeploy() {
  deploying.value = true;
  try {
    await projectApi.deployAgent("");
    Message.success(t("remote.deploySuccess"));
  } catch (e) {
    Message.error(String(e));
  } finally {
    deploying.value = false;
  }
}

function handleNext() {
  if (currentStep.value === 1 && testResult.value !== true) return;

  if (currentStep.value === 2 && remoteDirs.value.length === 0) {
    handleBrowseDir(form.remoteRoot);
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

function handleFinish() {
  emit("success");
  handleClose();
}

function handleClose() {
  currentStep.value = 0;
  testing.value = false;
  deploying.value = false;
  testResult.value = null;
  sshStatus.value = "disconnected";
  remoteDirs.value = [];
  form.name = "";
  form.host = "";
  form.port = 22;
  form.username = "";
  form.authMethod = "key";
  form.keyPath = "";
  form.remoteRoot = "/home";
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
        <a-form-item :label="t('project.name')" required>
          <a-input
            v-model="form.name"
            :placeholder="t('project.namePlaceholder')"
          />
        </a-form-item>
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
          />
        </a-form-item>
        <a-form-item :label="t('remote.authMethod')">
          <a-radio-group v-model="form.authMethod">
            <a-radio value="key">{{ t("remote.authKey") }}</a-radio>
            <a-radio value="password">{{ t("remote.authPassword") }}</a-radio>
            <a-radio value="agent">{{ t("remote.authAgent") }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item
          v-if="form.authMethod === 'key'"
          :label="t('remote.keyPath')"
        >
          <a-input
            v-model="form.keyPath"
            :placeholder="t('remote.keyPathPlaceholder')"
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
              {{ form.authMethod }}
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

    <!-- Step 3: 远程目录选择 -->
    <div v-show="currentStep === 2">
      <a-form-item :label="t('remote.remotePath')">
        <a-input v-model="form.remoteRoot" style="margin-bottom: 8px" />
      </a-form-item>

      <div v-if="form.selectedPath" class="selected-path">
        <a-tag color="arcoblue">{{ form.selectedPath }}</a-tag>
      </div>

      <a-list :bordered="false" size="small" class="dir-list">
        <a-list-item
          v-for="entry in remoteDirs"
          :key="entry.path"
          class="dir-item"
          @click="handleSelectPath(entry)"
        >
          <template #meta>
            <a-list-item-meta>
              <template #avatar>
                <icon-folder v-if="entry.is_dir" style="color: var(--color-primary-light-4)" />
                <icon-file v-else style="color: var(--color-text-3)" />
              </template>
              <template #title>
                <span :class="{ 'dir-name': entry.is_dir }">{{ entry.name }}</span>
              </template>
            </a-list-item-meta>
          </template>
        </a-list-item>
      </a-list>
    </div>

    <!-- Step 4: Agent 部署 -->
    <div v-show="currentStep === 3">
      <a-space direction="vertical" fill>
        <a-alert type="info">
          {{ t("remote.deployInfo") }}
        </a-alert>

        <a-descriptions :column="1" size="small" bordered>
          <a-descriptions-item :label="t('project.name')">
            {{ form.name }}
          </a-descriptions-item>
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

.selected-path {
  margin-bottom: 8px;
}

.dir-list {
  max-height: 300px;
  overflow-y: auto;
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
