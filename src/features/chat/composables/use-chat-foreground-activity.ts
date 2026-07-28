import type { Ref } from "vue";
import {
  restoreTransportAfterForegroundWake,
  setTransportChatViewActive,
} from "../../../services/tauri-api";

export type ChatForegroundActivityOptions = {
  activeSynced: Ref<boolean | null>;
  onWake: (reason: string) => Promise<void>;
  onBackground?: (reason: string) => void;
  onWakeError?: (reason: string, error: unknown) => void;
  isEnabled?: () => boolean;
};

/** 所有聊天宿主共用的 focus/visibility 生命周期；传输差异由适配器处理。 */
export function useChatForegroundActivity(options: ChatForegroundActivityOptions) {
  let syncGeneration = 0;
  let syncTimer: ReturnType<typeof setTimeout> | null = null;

  function clearSyncTimer() {
    if (syncTimer === null) return;
    clearTimeout(syncTimer);
    syncTimer = null;
  }

  function isActiveNow(): boolean {
    return (options.isEnabled?.() ?? true)
      && typeof document !== "undefined"
      && document.visibilityState === "visible"
      && (typeof document.hasFocus !== "function" || document.hasFocus());
  }

  async function sync(reason = "unknown") {
    const generation = ++syncGeneration;
    const active = isActiveNow();
    const activeChanged = options.activeSynced.value !== active;
    if (!activeChanged && !active) return;
    options.activeSynced.value = active;
    if (!active) {
      options.onBackground?.(reason);
      await setTransportChatViewActive(false).catch(() => {});
      return;
    }
    try {
      await restoreTransportAfterForegroundWake();
      if (generation !== syncGeneration || !isActiveNow()) return;
      await options.onWake(reason);
      if (generation !== syncGeneration || !isActiveNow()) return;
      await setTransportChatViewActive(true);
    } catch (error) {
      options.onWakeError?.(reason, error);
    }
  }

  function schedule(reason: string, delayMs = 0) {
    clearSyncTimer();
    if (delayMs <= 0) {
      void sync(reason);
      return;
    }
    syncTimer = setTimeout(() => {
      syncTimer = null;
      void sync(reason);
    }, delayMs);
  }

  function handleFocus() {
    schedule("focus");
  }

  function handleBlur() {
    options.onBackground?.("blur");
    schedule("blur");
  }

  function handleVisibilityChange() {
    clearSyncTimer();
    if (typeof document !== "undefined" && document.visibilityState !== "visible") {
      options.onBackground?.("visibility_hidden");
    }
    void sync("visibilitychange");
  }

  function handlePageShow() {
    options.activeSynced.value = null;
    void sync("pageshow");
  }

  function cleanup() {
    ++syncGeneration;
    clearSyncTimer();
    options.activeSynced.value = null;
    options.onBackground?.("cleanup");
    void setTransportChatViewActive(false).catch(() => {});
  }

  return {
    isActiveNow,
    clearSyncTimer,
    sync,
    schedule,
    handleFocus,
    handleBlur,
    handleVisibilityChange,
    handlePageShow,
    cleanup,
  };
}
