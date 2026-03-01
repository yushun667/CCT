import { onMounted, onUnmounted, ref, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 封装 Tauri 事件监听，自动在组件卸载时取消
 */
export function useTauriEvent<T>(
  eventName: string,
  handler?: (payload: T) => void
): { payload: Ref<T | null>; listening: Ref<boolean> } {
  const payload = ref<T | null>(null) as Ref<T | null>;
  const listening = ref(false);
  let unlisten: UnlistenFn | null = null;

  onMounted(async () => {
    unlisten = await listen<T>(eventName, (event) => {
      payload.value = event.payload;
      handler?.(event.payload);
    });
    listening.value = true;
  });

  onUnmounted(() => {
    if (unlisten) {
      unlisten();
      unlisten = null;
      listening.value = false;
    }
  });

  return { payload, listening };
}
