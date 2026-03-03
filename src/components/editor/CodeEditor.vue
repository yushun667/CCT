<script setup lang="ts">
/**
 * Monaco Editor 代码查看器 — 只读模式展示 C/C++ 源码
 *
 * 支持右键菜单"显示调用图"和"查找引用"，用于代码 → 图谱的跳转。
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
  lineSeq: { type: Number, default: 0 },
});

const emit = defineEmits<{
  (e: "symbol-click", line: number, column: number): void;
  (e: "show-call-graph", line: number, column: number): void;
  (e: "find-references", line: number, column: number): void;
  (e: "show-callers", line: number, column: number): void;
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
    contextmenu: true,
  });

  const callGraphAction = editorInstance.addAction({
    id: "cct.showCallGraph",
    label: "显示调用图",
    keybindings: [],
    contextMenuGroupId: "navigation",
    contextMenuOrder: 1,
    run: (editor) => {
      const pos = editor.getPosition();
      if (pos) {
        emit("show-call-graph", pos.lineNumber, pos.column);
      }
    },
  });

  const findCallersAction = editorInstance.addAction({
    id: "cct.showCallers",
    label: "查找调用者",
    keybindings: [],
    contextMenuGroupId: "navigation",
    contextMenuOrder: 2,
    run: (editor) => {
      const pos = editor.getPosition();
      if (pos) {
        emit("show-callers", pos.lineNumber, pos.column);
      }
    },
  });

  const findRefsAction = editorInstance.addAction({
    id: "cct.findReferences",
    label: "查找引用",
    keybindings: [],
    contextMenuGroupId: "navigation",
    contextMenuOrder: 3,
    run: (editor) => {
      const pos = editor.getPosition();
      if (pos) {
        emit("find-references", pos.lineNumber, pos.column);
      }
    },
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
  () => props.lineSeq,
  () => {
    if (props.line) {
      revealLine(props.line);
    }
  },
);

function revealLine(line: number) {
  if (!editorInstance) return;
  editorInstance.setSelection({
    startLineNumber: line,
    startColumn: 1,
    endLineNumber: line,
    endColumn: 1,
  });
  editorInstance.revealLineInCenter(line);
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
