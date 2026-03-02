import { computed, watch } from "vue";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";

export function useWindowTitle() {
  const projectStore = useProjectStore();
  const editorStore = useEditorStore();

  const windowTitle = computed(() => {
    const file = editorStore.activeFile;
    const proj = projectStore.currentProject;

    if (file) {
      const fileName = file.fileName;
      if (proj) {
        const rel =
          file.filePath.replace(proj.source_root, "").replace(/^\//, "") ||
          file.filePath;
        return `${fileName} — ${rel} — CCT`;
      }
      return `${fileName} — CCT`;
    }

    if (proj) {
      return `${proj.name} — CCT`;
    }

    return "CCT — C/C++ 架构分析平台";
  });

  watch(
    windowTitle,
    async (title) => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().setTitle(title);
      } catch {
        document.title = title;
      }
    },
    { immediate: true },
  );

  return { windowTitle };
}
