<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  rootPath: string;
}>();

const treeData = computed(() => [
  {
    title: props.rootPath,
    key: props.rootPath,
    isLeaf: false,
  },
]);
</script>

<template>
  <a-tree :data="treeData" block-node default-expand-all />
</template>
