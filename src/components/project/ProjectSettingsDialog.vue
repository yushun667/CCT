<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useI18n } from "vue-i18n";
import { Message } from "@arco-design/web-vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/stores/project";
import type { Project } from "@/api/types";

const props = defineProps<{
  visible: boolean;
  project: Project;
}>();

const emit = defineEmits<{
  "update:visible": [val: boolean];
  saved: [];
}>();

const { t } = useI18n();
const projectStore = useProjectStore();

const form = ref({
  compileDbPath: "",
  excludedDirs: [] as string[],
});
const newDirName = ref("");
const saving = ref(false);

const dialogVisible = computed({
  get: () => props.visible,
  set: (val: boolean) => emit("update:visible", val),
});

watch(
  () => props.visible,
  (v) => {
    if (v) {
      form.value = {
        compileDbPath: props.project.compile_db_path ?? "",
        excludedDirs: [...(props.project.excluded_dirs ?? [])],
      };
      newDirName.value = "";
    }
  },
);

function addDirByName() {
  const name = newDirName.value.trim();
  if (!name) return;
  if (form.value.excludedDirs.includes(name)) {
    Message.warning(t("projectSettings.dirExists"));
    return;
  }
  form.value.excludedDirs.push(name);
  newDirName.value = "";
}

async function addDirByPicker() {
  const selected = await open({
    directory: true,
    multiple: true,
    defaultPath: props.project.source_root,
  });
  if (!selected) return;

  const paths = Array.isArray(selected) ? selected : [selected];
  for (const fullPath of paths) {
    const dirName = (fullPath as string).split("/").filter(Boolean).pop() ?? "";
    if (!dirName) continue;
    if (form.value.excludedDirs.includes(dirName)) continue;
    form.value.excludedDirs.push(dirName);
  }
}

function removeDir(idx: number) {
  form.value.excludedDirs.splice(idx, 1);
}

async function handleSelectCompileDb() {
  const selected = await open({
    filters: [{ name: "JSON", extensions: ["json"] }],
    multiple: false,
  });
  if (selected) {
    form.value.compileDbPath = selected as string;
  }
}

async function handleSave() {
  saving.value = true;
  try {
    await projectStore.updateProject(props.project.id, {
      compileDbPath: form.value.compileDbPath || undefined,
      excludedDirs: form.value.excludedDirs,
    });
    Message.success(t("projectSettings.saved"));
    emit("saved");
    dialogVisible.value = false;
  } catch {
    // error handled in store
  } finally {
    saving.value = false;
  }
}

const builtinDirs = [
  ".git", ".svn", "node_modules", "__pycache__",
  "test", "tests", "unittests", "testing",
  "benchmarks", "examples",
];
</script>

<template>
  <a-modal
    v-model:visible="dialogVisible"
    :title="t('projectSettings.title')"
    :ok-text="t('common.save')"
    :cancel-text="t('common.cancel')"
    :ok-loading="saving"
    @ok="handleSave"
    :width="560"
    unmount-on-close
  >
    <a-form :model="form" layout="vertical" size="small">
      <a-form-item :label="t('project.sourceRoot')">
        <a-input :model-value="project.source_root" disabled />
      </a-form-item>

      <a-form-item :label="t('project.compileDb')">
        <a-input-group>
          <a-input
            v-model="form.compileDbPath"
            :placeholder="t('project.compileDbPlaceholder')"
            style="flex: 1"
          />
          <a-button @click="handleSelectCompileDb">
            <template #icon><icon-folder /></template>
          </a-button>
        </a-input-group>
      </a-form-item>

      <a-form-item :label="t('projectSettings.excludedDirs')">
        <div class="excluded-dirs-section">
          <div class="excluded-dirs-hint">
            {{ t("projectSettings.builtinHint") }}
          </div>
          <div class="builtin-tags">
            <a-tag v-for="d in builtinDirs" :key="d" size="small" color="gray">
              {{ d }}
            </a-tag>
          </div>

          <a-divider :margin="8" />

          <div class="custom-label">{{ t("projectSettings.customDirs") }}</div>
          <div class="dir-input-row">
            <a-input
              v-model="newDirName"
              :placeholder="t('projectSettings.dirPlaceholder')"
              @press-enter="addDirByName"
              style="flex: 1"
            />
            <a-button type="primary" @click="addDirByPicker">
              <template #icon><icon-folder /></template>
              {{ t("projectSettings.browse") }}
            </a-button>
          </div>

          <div v-if="form.excludedDirs.length" class="custom-tags">
            <a-tag
              v-for="(d, idx) in form.excludedDirs"
              :key="d"
              size="small"
              closable
              @close="removeDir(idx)"
            >
              {{ d }}
            </a-tag>
          </div>
          <div v-else class="no-custom">
            {{ t("projectSettings.noneAdded") }}
          </div>
        </div>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<style scoped>
.excluded-dirs-section {
  width: 100%;
}

.excluded-dirs-hint {
  font-size: 12px;
  color: var(--color-text-3);
  margin-bottom: 4px;
}

.builtin-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.custom-label {
  font-size: 12px;
  font-weight: 500;
  margin-bottom: 4px;
}

.dir-input-row {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}

.custom-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.no-custom {
  font-size: 12px;
  color: var(--color-text-4);
}
</style>
