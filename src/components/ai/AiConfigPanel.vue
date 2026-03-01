<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useAiStore } from "@/stores/ai";
import type { AiConfig } from "@/api/ai";

const { t } = useI18n();
const aiStore = useAiStore();

const emit = defineEmits<{
  close: [];
}>();

const formData = ref<AiConfig>({
  provider: null,
  model: null,
  api_key_ref: null,
  base_url: null,
  privacy_mode: "Local",
});

const saving = ref(false);

const providerOptions = [
  { label: "OpenAI", value: "openai" },
  { label: "Anthropic (Claude)", value: "anthropic" },
  { label: "Ollama", value: "ollama" },
  { label: t("ai.customProvider"), value: "custom" },
];

const privacyOptions = [
  { label: t("ai.privacyFull"), value: "Full" },
  { label: t("ai.privacyAnonymized"), value: "Anonymized" },
  { label: t("ai.privacyLocal"), value: "Local" },
];

const modelSuggestions: Record<string, string[]> = {
  openai: ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
  anthropic: ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
  ollama: ["codellama", "llama3", "deepseek-coder"],
  custom: [],
};

const currentModelOptions = ref<string[]>([]);

function onProviderChange(val: string | null) {
  formData.value.provider = val;
  currentModelOptions.value = val ? modelSuggestions[val] || [] : [];
  if (val === "ollama") {
    formData.value.base_url = formData.value.base_url || "http://localhost:11434";
  }
}

async function handleSave() {
  saving.value = true;
  try {
    await aiStore.updateConfig(formData.value);
    emit("close");
  } finally {
    saving.value = false;
  }
}

onMounted(async () => {
  await aiStore.loadConfig();
  formData.value = { ...aiStore.aiConfig };
  if (formData.value.provider) {
    currentModelOptions.value =
      modelSuggestions[formData.value.provider] || [];
  }
});
</script>

<template>
  <div class="ai-config-panel">
    <div class="ai-config-panel__header">
      <span class="ai-config-panel__title">{{ t("ai.config") }}</span>
      <a-button type="text" size="small" @click="emit('close')">
        <template #icon><icon-close /></template>
      </a-button>
    </div>

    <a-form :model="formData" layout="vertical" class="ai-config-panel__form">
      <a-form-item :label="t('ai.provider')">
        <a-select
          v-model="formData.provider"
          :placeholder="t('ai.selectProvider')"
          allow-clear
          @change="onProviderChange"
        >
          <a-option
            v-for="opt in providerOptions"
            :key="opt.value"
            :value="opt.value"
            :label="opt.label"
          />
        </a-select>
      </a-form-item>

      <a-form-item :label="t('ai.model')">
        <a-select
          v-model="formData.model"
          :placeholder="t('ai.selectModel')"
          allow-clear
          allow-create
        >
          <a-option
            v-for="m in currentModelOptions"
            :key="m"
            :value="m"
            :label="m"
          />
        </a-select>
      </a-form-item>

      <a-form-item
        v-if="formData.provider && formData.provider !== 'ollama'"
        :label="t('ai.apiKey')"
      >
        <a-input-password
          v-model="formData.api_key_ref"
          :placeholder="t('ai.apiKeyPlaceholder')"
        />
      </a-form-item>

      <a-form-item
        v-if="formData.provider === 'ollama' || formData.provider === 'custom'"
        :label="t('ai.baseUrl')"
      >
        <a-input
          v-model="formData.base_url"
          :placeholder="t('ai.baseUrlPlaceholder')"
        />
      </a-form-item>

      <a-form-item :label="t('ai.privacyMode')">
        <a-radio-group v-model="formData.privacy_mode" direction="vertical">
          <a-radio
            v-for="opt in privacyOptions"
            :key="opt.value"
            :value="opt.value"
          >
            {{ opt.label }}
          </a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item>
        <a-space>
          <a-button type="primary" :loading="saving" @click="handleSave">
            {{ t("common.save") }}
          </a-button>
          <a-button @click="emit('close')">
            {{ t("common.cancel") }}
          </a-button>
        </a-space>
      </a-form-item>
    </a-form>
  </div>
</template>

<style scoped>
.ai-config-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.ai-config-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-border);
}

.ai-config-panel__title {
  font-weight: 600;
  font-size: 14px;
}

.ai-config-panel__form {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
}
</style>
