import { onMounted, onUnmounted } from "vue";
import { useSettingsStore } from "@/stores/settings";

export interface ShortcutEntry {
  /** 快捷键组合（用于显示） */
  keys: string;
  /** macOS 下的快捷键显示 */
  keysMac: string;
  /** 功能说明的 i18n key */
  labelKey: string;
  /** 分类 */
  category: string;
}

/** 所有已注册的快捷键（用于 Settings 页面展示） */
export const shortcutList: ShortcutEntry[] = [
  {
    keys: "Ctrl+P",
    keysMac: "⌘+P",
    labelKey: "shortcuts.search",
    category: "shortcuts.catGeneral",
  },
  {
    keys: "Ctrl+B",
    keysMac: "⌘+B",
    labelKey: "shortcuts.toggleSidebar",
    category: "shortcuts.catLayout",
  },
  {
    keys: "Ctrl+J",
    keysMac: "⌘+J",
    labelKey: "shortcuts.toggleBottomPanel",
    category: "shortcuts.catLayout",
  },
  {
    keys: "Ctrl+Shift+A",
    keysMac: "⌘+Shift+A",
    labelKey: "shortcuts.toggleAiPanel",
    category: "shortcuts.catLayout",
  },
];

const isMac = navigator.platform.toUpperCase().includes("MAC");

/**
 * 全局键盘快捷键 composable
 *
 * 在 App 根组件中调用一次即可注册全局快捷键，
 * 组件卸载时自动清理监听器。
 */
export function useKeyboardShortcuts() {
  const settings = useSettingsStore();

  function handleKeydown(e: KeyboardEvent) {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (!mod) return;

    if (e.key === "p" || e.key === "P") {
      e.preventDefault();
      document.dispatchEvent(new CustomEvent("cct:open-search"));
      return;
    }

    if ((e.key === "b" || e.key === "B") && !e.shiftKey) {
      e.preventDefault();
      settings.toggleSidebar();
      return;
    }

    if ((e.key === "j" || e.key === "J") && !e.shiftKey) {
      e.preventDefault();
      settings.toggleBottomPanel();
      return;
    }

    if ((e.key === "a" || e.key === "A") && e.shiftKey) {
      e.preventDefault();
      settings.toggleAiPanel();
      return;
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
}
