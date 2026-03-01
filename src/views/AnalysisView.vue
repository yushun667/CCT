<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <div class="analysis-view">
    <a-result status="info" :title="t('app.fullName')">
      <template #subtitle>
        <span>选择左侧项目开始分析，或创建新项目</span>
      </template>
      <template #extra>
        <a-space>
          <a-button type="primary" @click="$router.push('/projects')">
            <template #icon><icon-folder /></template>
            {{ t("sidebar.projects") }}
          </a-button>
        </a-space>
      </template>
    </a-result>
  </div>
</template>

<style scoped>
.analysis-view {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}
</style>
