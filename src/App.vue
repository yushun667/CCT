<script setup lang="ts">
import { onMounted } from "vue";
import MainLayout from "@/components/layout/MainLayout.vue";
import { useTheme } from "@/composables/useTheme";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";
import { useProjectStore } from "@/stores/project";

useTheme();
useKeyboardShortcuts();

const projectStore = useProjectStore();

onMounted(async () => {
  await projectStore.fetchProjects();
  await projectStore.listenParseProgress();
});
</script>

<template>
  <MainLayout />
</template>
