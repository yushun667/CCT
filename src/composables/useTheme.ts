import { computed, watch } from "vue";
import { useSettingsStore } from "@/stores/settings";

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
    (dark) => {
      if (dark) {
        document.body.setAttribute("arco-theme", "dark");
      } else {
        document.body.removeAttribute("arco-theme");
      }
    },
    { immediate: true }
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
