<script setup lang="ts">
import { ref, watch, computed, type Component } from "vue";
import { useI18n } from "vue-i18n";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useSettingsStore } from "@/stores/settings";
import * as editorApi from "@/api/editor";
import type { DirEntry } from "@/api/editor";

import MdiFolder from "~icons/mdi/folder";
import MdiFolderOpen from "~icons/mdi/folder-open";
import MdiLanguageC from "~icons/mdi/language-c";
import MdiLanguageCpp from "~icons/mdi/language-cpp";
import MdiLanguageTypescript from "~icons/mdi/language-typescript";
import MdiLanguageJavascript from "~icons/mdi/language-javascript";
import MdiVuejs from "~icons/mdi/vuejs";
import MdiLanguageRust from "~icons/mdi/language-rust";
import MdiLanguagePython from "~icons/mdi/language-python";
import MdiLanguageJava from "~icons/mdi/language-java";
import MdiLanguageGo from "~icons/mdi/language-go";
import MdiLanguageSwift from "~icons/mdi/language-swift";
import MdiLanguageKotlin from "~icons/mdi/language-kotlin";
import MdiLanguageRuby from "~icons/mdi/language-ruby";
import MdiLanguageHtml5 from "~icons/mdi/language-html5";
import MdiLanguageCss3 from "~icons/mdi/language-css3";
import MdiLanguageMarkdown from "~icons/mdi/language-markdown";
import MdiCodeJson from "~icons/mdi/code-json";
import MdiXml from "~icons/mdi/xml";
import MdiConsole from "~icons/mdi/console";
import MdiGit from "~icons/mdi/git";
import MdiFileCog from "~icons/mdi/file-cog";
import MdiFileDocumentOutline from "~icons/mdi/file-document-outline";

const { t } = useI18n();
const projectStore = useProjectStore();
const editorStore = useEditorStore();
const settings = useSettingsStore();

const fileIconMap: Record<string, Component> = {
  ".c": MdiLanguageC,
  ".h": MdiLanguageC,
  ".cpp": MdiLanguageCpp,
  ".cc": MdiLanguageCpp,
  ".cxx": MdiLanguageCpp,
  ".hpp": MdiLanguageCpp,
  ".hxx": MdiLanguageCpp,
  ".ts": MdiLanguageTypescript,
  ".tsx": MdiLanguageTypescript,
  ".js": MdiLanguageJavascript,
  ".jsx": MdiLanguageJavascript,
  ".vue": MdiVuejs,
  ".rs": MdiLanguageRust,
  ".py": MdiLanguagePython,
  ".java": MdiLanguageJava,
  ".go": MdiLanguageGo,
  ".swift": MdiLanguageSwift,
  ".kt": MdiLanguageKotlin,
  ".kts": MdiLanguageKotlin,
  ".rb": MdiLanguageRuby,
  ".html": MdiLanguageHtml5,
  ".htm": MdiLanguageHtml5,
  ".css": MdiLanguageCss3,
  ".scss": MdiLanguageCss3,
  ".less": MdiLanguageCss3,
  ".md": MdiLanguageMarkdown,
  ".json": MdiCodeJson,
  ".xml": MdiXml,
  ".svg": MdiXml,
  ".sh": MdiConsole,
  ".bash": MdiConsole,
  ".zsh": MdiConsole,
  ".yaml": MdiFileCog,
  ".yml": MdiFileCog,
  ".toml": MdiFileCog,
  ".ini": MdiFileCog,
  ".conf": MdiFileCog,
  ".cmake": MdiFileCog,
};

const fileNameIconMap: Record<string, Component> = {
  ".gitignore": MdiGit,
  ".gitattributes": MdiGit,
  ".gitmodules": MdiGit,
  "Makefile": MdiFileCog,
  "CMakeLists.txt": MdiFileCog,
  "Dockerfile": MdiConsole,
};

function getFileIcon(filename: string): Component {
  if (fileNameIconMap[filename]) return fileNameIconMap[filename];
  const dotIdx = filename.lastIndexOf(".");
  if (dotIdx >= 0) {
    const ext = filename.slice(dotIdx).toLowerCase();
    if (fileIconMap[ext]) return fileIconMap[ext];
  }
  return MdiFileDocumentOutline;
}

interface TreeNode {
  key: string;
  title: string;
  isLeaf: boolean;
  children?: TreeNode[];
}

const treeData = ref<TreeNode[]>([]);
const loadingTree = ref(false);

const currentProject = computed(() => projectStore.currentProject);

watch(
  () => projectStore.currentProject,
  async (proj) => {
    if (proj) {
      await loadRootDir(proj.source_root);
    } else {
      treeData.value = [];
    }
  },
  { immediate: true },
);

async function loadRootDir(rootPath: string) {
  loadingTree.value = true;
  try {
    const entries = await editorApi.listDirectory(rootPath);
    treeData.value = entriesToNodes(entries);
  } catch {
    treeData.value = [];
  } finally {
    loadingTree.value = false;
  }
}

function entriesToNodes(entries: DirEntry[]): TreeNode[] {
  return entries.map((e) => ({
    key: e.path,
    title: e.name,
    isLeaf: !e.is_dir,
    children: e.is_dir ? undefined : undefined,
  }));
}

async function onLoadMore(node: TreeNode) {
  try {
    const entries = await editorApi.listDirectory(node.key);
    node.children = entriesToNodes(entries);
  } catch {
    node.children = [];
  }
  return node;
}

function onNodeClick(_: string[], data: { node?: TreeNode }) {
  const node = data.node;
  if (node && node.isLeaf) {
    const projectId = projectStore.currentProjectId ?? undefined;
    editorStore.openFile(node.key, projectId);
  }
}
</script>

<template>
  <div class="sidebar">
    <div v-if="!settings.sidebarCollapsed" class="sidebar-content">
      <!-- 项目名称标题 -->
      <div v-if="currentProject" class="project-header">
        <MdiFolder class="project-icon" />
        <span class="project-name" :title="currentProject.source_root">
          {{ currentProject.name }}
        </span>
      </div>

      <!-- 文件树 -->
      <div class="file-tree-area">
        <a-spin :loading="loadingTree" style="width: 100%">
          <a-tree
            v-if="treeData.length > 0"
            :data="treeData"
            :load-more="onLoadMore"
            block-node
            size="small"
            @select="onNodeClick"
          >
            <template #icon="{ node, expanded }">
              <component
                :is="node.isLeaf ? getFileIcon(node.title ?? '') : (expanded ? MdiFolderOpen : MdiFolder)"
                :class="node.isLeaf ? 'file-icon' : 'folder-icon'"
              />
            </template>
          </a-tree>
          <div v-else class="tree-empty">
            <a-empty
              :description="currentProject ? t('sidebar.files') : t('project.noProjects')"
            />
          </div>
        </a-spin>
      </div>
    </div>

    <!-- 折叠态 -->
    <div v-else class="sidebar-collapsed">
      <a-tooltip :content="t('sidebar.files')" position="right">
        <a-button type="text" @click="settings.toggleSidebar()">
          <template #icon><MdiFolder /></template>
        </a-button>
      </a-tooltip>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sidebar-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.project-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.project-icon {
  color: var(--color-primary-light-4);
  flex-shrink: 0;
  font-size: 16px;
}

.project-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder-icon {
  color: var(--color-primary-light-4);
  font-size: 16px;
}

.file-icon {
  color: var(--color-text-3);
  font-size: 16px;
}

.file-tree-area {
  flex: 1;
  overflow: auto;
  padding: 4px 0;
}

.tree-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
}

.sidebar-collapsed {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 8px;
  gap: 4px;
}
</style>
