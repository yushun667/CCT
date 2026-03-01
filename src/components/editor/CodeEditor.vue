<script setup lang="ts">
/**
 * Monaco Editor 代码查看器 — 只读模式展示 C/C++ 源码
 *
 * # 设计说明（适配器模式）
 * 将 Monaco Editor 原生 API 适配为 Vue 组件接口，
 * 通过 Props 接收文件内容，通过 Emits 通知符号点击事件。
 */
import { ref, watch, onMounted, onBeforeUnmount, type PropType } from "vue";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};

const props = defineProps({
  filePath: { type: String, default: "" },
  content: { type: String, default: "" },
  language: { type: String, default: "cpp" },
  line: { type: Number as PropType<number | null>, default: null },
});

const emit = defineEmits<{
  (e: "symbol-click", line: number, column: number): void;
}>();

const containerRef = ref<HTMLDivElement>();
let editorInstance: monaco.editor.IStandaloneCodeEditor | null = null;

onMounted(() => {
  if (!containerRef.value) return;

  editorInstance = monaco.editor.create(containerRef.value, {
    value: props.content,
    language: props.language,
    readOnly: true,
    minimap: { enabled: true },
    scrollBeyondLastLine: false,
    fontSize: 13,
    lineNumbers: "on",
    renderLineHighlight: "all",
    automaticLayout: true,
    wordWrap: "off",
    theme: "vs-dark",
  });

  editorInstance.onMouseDown((e) => {
    if (e.target.position) {
      emit(
        "symbol-click",
        e.target.position.lineNumber,
        e.target.position.column,
      );
    }
  });

  if (props.line) {
    revealLine(props.line);
  }
});

onBeforeUnmount(() => {
  editorInstance?.dispose();
  editorInstance = null;
});

watch(
  () => props.content,
  (newContent) => {
    if (editorInstance) {
      const model = editorInstance.getModel();
      if (model && model.getValue() !== newContent) {
        model.setValue(newContent);
      }
    }
  },
);

watch(
  () => props.language,
  (newLang) => {
    if (editorInstance) {
      const model = editorInstance.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, newLang);
      }
    }
  },
);

watch(
  () => props.line,
  (newLine) => {
    if (newLine) {
      revealLine(newLine);
    }
  },
);

function revealLine(line: number) {
  if (!editorInstance) return;
  editorInstance.revealLineInCenter(line);
  editorInstance.setPosition({ lineNumber: line, column: 1 });
}

defineExpose({ revealLine });
</script>

<template>
  <div ref="containerRef" class="code-editor" />
</template>

<style scoped>
.code-editor {
  width: 100%;
  height: 100%;
  min-height: 200px;
}
</style>
