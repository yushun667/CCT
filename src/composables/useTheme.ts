import { computed, watch } from "vue";
import { useSettingsStore } from "@/stores/settings";

let tauriWindow: ReturnType<typeof import("@tauri-apps/api/window").getCurrentWindow> | null = null;

async function getTauriWindow() {
  if (!tauriWindow) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    tauriWindow = getCurrentWindow();
  }
  return tauriWindow;
}

export function useTheme() {
  const settings = useSettingsStore();

  const isDark = computed(() => {
    if (settings.theme === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    return settings.theme === "dark";
  });

  watch(
    isDark,
    async (dark) => {
      if (dark) {
        document.body.setAttribute("arco-theme", "dark");
        document.documentElement.classList.add("dark");
        document.documentElement.style.colorScheme = "dark";
      } else {
        document.body.removeAttribute("arco-theme");
        document.documentElement.classList.remove("dark");
        document.documentElement.style.colorScheme = "light";
      }

      try {
        const win = await getTauriWindow();
        await win.setTheme(dark ? "dark" : "light");
      } catch {
        // Tauri API not available in browser dev mode
      }
    },
    { immediate: true },
  );

  const toggleTheme = () => {
    if (settings.theme === "dark") {
      settings.setTheme("light");
    } else {
      settings.setTheme("dark");
    }
  };

  return { isDark, toggleTheme };
}
