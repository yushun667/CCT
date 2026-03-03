<script setup lang="ts">
/**
 * 拆分编辑器 — 管理左右两个编辑窗格
 *
 * 每个窗格显示独立的 EditorTabs + 内容区（CodeEditor 或 CallGraphTab）。
 * 中间有可拖拽的分隔条，支持动态调整比例。
 */
import { ref, computed, watch, nextTick } from "vue";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import EditorTabs from "./EditorTabs.vue";
import CodeEditor from "./CodeEditor.vue";
import WelcomeScreen from "@/components/welcome/WelcomeScreen.vue";
import CallGraphTab from "@/components/graph/CallGraphTab.vue";

const emit = defineEmits<{
  (e: "show-call-graph", line: number, column: number): void;
  (e: "show-callers", line: number, column: number): void;
  (e: "find-references", line: number, column: number): void;
}>();

const editorStore = useEditorStore();
const projectStore = useProjectStore();

const leftEditorRef = ref<InstanceType<typeof CodeEditor> | null>(null);
const rightEditorRef = ref<InstanceType<typeof CodeEditor> | null>(null);
const leftRatio = ref(50);

watch(
  () => editorStore.targetLineSeq,
  () => {
    if (!editorStore.targetLine) return;
    const pi = editorStore.activePaneIndex;
    const edRef = pi === 0 ? leftEditorRef : rightEditorRef;
    nextTick(() => {
      edRef.value?.revealLine(editorStore.targetLine!);
    });
  },
);

function onSplitResizeStart(e: MouseEvent) {
  const container = (e.target as HTMLElement).parentElement;
  if (!container) return;
  const rect = container.getBoundingClientRect();
  const totalWidth = rect.width;

  const onMove = (ev: MouseEvent) => {
    const offset = ev.clientX - rect.left;
    leftRatio.value = Math.max(20, Math.min(80, (offset / totalWidth) * 100));
  };
  const onUp = () => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}

function onPaneClick(paneIdx: number) {
  editorStore.setActivePane(paneIdx);
}

const leftFiles = computed(() => editorStore.getPaneFiles(0));
const rightFiles = computed(() => editorStore.splitMode ? editorStore.getPaneFiles(1) : []);
const leftActiveFile = computed(() => editorStore.getPaneActiveFile(0));
const rightActiveFile = computed(() => editorStore.getPaneActiveFile(1));

function onPaneDrop(paneIdx: number, event: DragEvent) {
  event.preventDefault();
  event.stopPropagation();
  const data = editorStore.getDragTabFromEvent(event);
  if (!data) return;
  if (data.paneIndex === paneIdx) return;
  editorStore.moveToPane(data.paneIndex, paneIdx, data.fileIndex);
}

/** 未拆分时拖到右侧条：先拆分再移动到右侧窗格 */
function onDropToRightWhenUnsplit(event: DragEvent) {
  event.preventDefault();
  event.stopPropagation();
  const data = editorStore.getDragTabFromEvent(event);
  if (!data) return;
  editorStore.moveToPane(data.paneIndex, 1, data.fileIndex);
}
</script>

<template>
  <div class="split-editor">
    <!-- Left Pane (always visible) -->
    <div
      class="editor-pane"
      :class="{ 'pane-active': editorStore.activePaneIndex === 0 }"
      :style="editorStore.splitMode ? { width: leftRatio + '%' } : { width: '100%' }"
      @mousedown="onPaneClick(0)"
    >
      <template v-if="leftFiles.length > 0">
        <EditorTabs :pane-index="0" />
        <div
          class="pane-content"
          @dragover.prevent
          @drop.prevent="onPaneDrop(0, $event)"
        >
          <CallGraphTab
            v-if="leftActiveFile?.type === 'call-graph' && leftActiveFile.graphData"
            :tab-id="leftActiveFile.filePath"
            :graph-data="leftActiveFile.graphData"
          />
          <CodeEditor
            v-else-if="leftActiveFile"
            ref="leftEditorRef"
            :key="leftActiveFile.filePath"
            :file-path="leftActiveFile.filePath"
            :content="leftActiveFile.content"
            :language="leftActiveFile.language"
            :line="editorStore.activePaneIndex === 0 ? editorStore.targetLine : null"
            @show-call-graph="(line: number, col: number) => emit('show-call-graph', line, col)"
            @show-callers="(line: number, col: number) => emit('show-callers', line, col)"
            @find-references="(line: number, col: number) => emit('find-references', line, col)"
          />
        </div>
      </template>
      <WelcomeScreen v-else />
    </div>

    <!-- 未拆分时：右侧拖放区，拖入后自动拆分并放入右侧窗格 -->
    <div
      v-if="!editorStore.splitMode && leftFiles.length > 0"
      class="drop-zone-unsplit"
      @dragover.prevent
      @drop.prevent="onDropToRightWhenUnsplit"
    >
      <span>拖到此处以在右侧打开</span>
    </div>

    <!-- Split Divider -->
    <div
      v-if="editorStore.splitMode"
      class="split-divider"
      @mousedown.prevent="onSplitResizeStart"
    />

    <!-- Right Pane (optional) -->
    <div
      v-if="editorStore.splitMode"
      class="editor-pane"
      :class="{ 'pane-active': editorStore.activePaneIndex === 1 }"
      :style="{ width: (100 - leftRatio) + '%' }"
      @mousedown="onPaneClick(1)"
    >
      <EditorTabs :pane-index="1" />
      <div
        v-if="rightActiveFile"
        class="pane-content"
        @dragover.prevent
        @drop.prevent="onPaneDrop(1, $event)"
      >
        <CallGraphTab
          v-if="rightActiveFile.type === 'call-graph' && rightActiveFile.graphData"
          :tab-id="rightActiveFile.filePath"
          :graph-data="rightActiveFile.graphData"
        />
        <CodeEditor
          v-else
          ref="rightEditorRef"
          :key="rightActiveFile.filePath"
          :file-path="rightActiveFile.filePath"
          :content="rightActiveFile.content"
          :language="rightActiveFile.language"
          :line="editorStore.activePaneIndex === 1 ? editorStore.targetLine : null"
          @show-call-graph="(line: number, col: number) => emit('show-call-graph', line, col)"
          @show-callers="(line: number, col: number) => emit('show-callers', line, col)"
          @find-references="(line: number, col: number) => emit('find-references', line, col)"
        />
      </div>
      <div
        v-else
        class="empty-pane"
        @dragover.prevent
        @drop.prevent="onPaneDrop(1, $event)"
      >
        <span>将文件拖到此处</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.split-editor {
  display: flex;
  flex: 1;
  overflow: hidden;
  height: 100%;
}

.editor-pane {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.editor-pane.pane-active {
  outline: 1px solid rgb(var(--primary-6));
  outline-offset: -1px;
}

.pane-content {
  flex: 1;
  overflow: hidden;
}

.drop-zone-unsplit {
  width: 48px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-fill-1);
  border: 1px dashed var(--color-border);
  border-radius: 4px;
  margin: 4px;
  color: var(--color-text-3);
  font-size: 11px;
  writing-mode: vertical-rl;
  letter-spacing: 2px;
  cursor: default;
  transition: background 0.15s, border-color 0.15s;
}
.drop-zone-unsplit:hover,
.drop-zone-unsplit:focus-within {
  background: var(--color-fill-2);
  border-color: rgb(var(--primary-6));
  color: var(--color-text-2);
}

.split-divider {
  width: 4px;
  cursor: col-resize;
  background: var(--color-border);
  flex-shrink: 0;
  transition: background 0.15s;
}

.split-divider:hover,
.split-divider:active {
  background: rgb(var(--primary-6));
}

.empty-pane {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-3);
  font-size: 13px;
  background: var(--color-bg-1);
}
</style>
