<script setup lang="ts">
import MainLayout from "@/components/layout/MainLayout.vue";
import { useTheme } from "@/composables/useTheme";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";

useTheme();
useKeyboardShortcuts();
</script>

<template>
  <MainLayout />
</template>
