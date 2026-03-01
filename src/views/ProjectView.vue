<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <div class="project-view">
    <div class="project-header">
      <h3>{{ t("project.title") }}</h3>
      <a-space>
        <a-button type="primary" size="small">
          <template #icon><icon-plus /></template>
          {{ t("project.newLocal") }}
        </a-button>
        <a-button size="small">
          <template #icon><icon-cloud /></template>
          {{ t("project.newRemote") }}
        </a-button>
      </a-space>
    </div>
    <div class="project-list">
      <a-empty :description="t('project.noProjects')" />
    </div>
  </div>
</template>

<style scoped>
.project-view {
  padding: 16px;
}

.project-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.project-header h3 {
  margin: 0;
}

.project-list {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
}
</style>
