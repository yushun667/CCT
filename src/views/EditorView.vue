<script setup lang="ts">
/**
 * 编辑器视图 — 整合 Tab 栏、代码编辑器和查询结果面板
 */
import { useI18n } from "vue-i18n";
import { useEditorStore } from "@/stores/editor";
import EditorTabs from "@/components/editor/EditorTabs.vue";
import CodeEditor from "@/components/editor/CodeEditor.vue";
import ResultPanel from "@/components/search/ResultPanel.vue";

const { t } = useI18n();
const editorStore = useEditorStore();
</script>

<template>
  <div class="editor-view">
    <EditorTabs />
    <div class="editor-body">
      <div class="editor-main">
        <CodeEditor
          v-if="editorStore.activeFile"
          :file-path="editorStore.activeFile.filePath"
          :content="editorStore.activeFile.content"
          :language="editorStore.activeFile.language"
        />
        <div v-else class="editor-placeholder">
          <a-empty :description="t('editor.noFileOpen')" />
        </div>
      </div>
      <div class="editor-sidebar">
        <ResultPanel />
      </div>
    </div>
  </div>
</template>

<style scoped>
.editor-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.editor-main {
  flex: 1;
  min-width: 0;
}

.editor-sidebar {
  width: 300px;
  border-left: 1px solid var(--color-border);
  overflow-y: auto;
  flex-shrink: 0;
}

.editor-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}
</style>
