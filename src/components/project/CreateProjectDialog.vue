<script setup lang="ts">
import { reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/stores/project";
import type { FieldRule } from "@arco-design/web-vue";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: "update:visible", val: boolean): void;
  (e: "success"): void;
}>();

const { t } = useI18n();
const projectStore = useProjectStore();
const submitting = ref(false);
const formRef = ref();

const form = reactive({
  name: "",
  sourceRoot: "",
  compileDbPath: "",
});

const rules: Record<string, FieldRule[]> = {
  name: [{ required: true, message: () => t("project.nameRequired") }],
  sourceRoot: [
    { required: true, message: () => t("project.sourceRootRequired") },
  ],
};

async function selectSourceRoot() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    form.sourceRoot = selected as string;
  }
}

async function selectCompileDb() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (selected) {
    form.compileDbPath = selected as string;
  }
}

async function handleSubmit() {
  const valid = await formRef.value?.validate();
  if (valid) return;

  submitting.value = true;
  try {
    const project = await projectStore.createLocalProject(
      form.name,
      form.sourceRoot,
    );
    if (form.compileDbPath) {
      await projectStore.updateProject(
        project.id,
        undefined,
        form.compileDbPath,
      );
    }
    resetForm();
    emit("success");
  } catch {
    // error is set in store
  } finally {
    submitting.value = false;
  }
}

function resetForm() {
  form.name = "";
  form.sourceRoot = "";
  form.compileDbPath = "";
  formRef.value?.clearValidate();
}

function handleCancel() {
  resetForm();
  emit("update:visible", false);
}
</script>

<template>
  <a-modal
    :visible="props.visible"
    :title="t('project.createTitle')"
    :ok-loading="submitting"
    :on-before-ok="handleSubmit"
    @cancel="handleCancel"
    @update:visible="emit('update:visible', $event)"
    unmount-on-close
  >
    <a-form ref="formRef" :model="form" :rules="rules" layout="vertical">
      <a-form-item field="name" :label="t('project.name')">
        <a-input
          v-model="form.name"
          :placeholder="t('project.namePlaceholder')"
        />
      </a-form-item>

      <a-form-item field="sourceRoot" :label="t('project.sourceRoot')">
        <a-input-group>
          <a-input
            v-model="form.sourceRoot"
            :placeholder="t('project.sourceRootPlaceholder')"
            style="flex: 1"
          />
          <a-button type="primary" @click="selectSourceRoot">
            <template #icon><icon-folder /></template>
            {{ t("project.selectDirectory") }}
          </a-button>
        </a-input-group>
      </a-form-item>

      <a-form-item field="compileDbPath" :label="t('project.compileDb')">
        <a-input-group>
          <a-input
            v-model="form.compileDbPath"
            :placeholder="t('project.compileDbPlaceholder')"
            style="flex: 1"
          />
          <a-button @click="selectCompileDb">
            <template #icon><icon-file /></template>
          </a-button>
        </a-input-group>
      </a-form-item>
    </a-form>
  </a-modal>
</template>
