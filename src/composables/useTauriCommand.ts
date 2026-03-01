import { ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface UseTauriCommandReturn<T> {
  data: Ref<T | null>;
  error: Ref<string | null>;
  loading: Ref<boolean>;
  execute: (...args: any[]) => Promise<T | null>;
}

/**
 * 封装 Tauri invoke 调用，提供响应式 loading/error 状态
 */
export function useTauriCommand<T>(
  command: string
): UseTauriCommandReturn<T> {
  const data = ref<T | null>(null) as Ref<T | null>;
  const error = ref<string | null>(null);
  const loading = ref(false);

  const execute = async (...args: any[]): Promise<T | null> => {
    loading.value = true;
    error.value = null;
    try {
      const payload = args.length === 1 ? args[0] : undefined;
      const result = await invoke<T>(command, payload);
      data.value = result;
      return result;
    } catch (e: any) {
      error.value = typeof e === "string" ? e : e.message ?? String(e);
      return null;
    } finally {
      loading.value = false;
    }
  };

  return { data, error, loading, execute };
}
