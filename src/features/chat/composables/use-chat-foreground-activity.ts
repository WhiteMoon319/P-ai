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
  let recheckTimer: ReturnType<typeof setTimeout> | null = null;
  let coldStartTimer: ReturnType<typeof setTimeout> | null = null;

  // 可见回来后的延迟复核：移动端可见先到、获焦后到，第一下判不稳时由它补一次；
  // 边沿去重保证补上的那次在已同步时是空转，不会造成桌面端重复唤醒。
  const VISIBLE_WAKE_RECHECK_MS = 350;
  // 冷启动兜底重试：进程被杀后重载，挂载时传输/视图可能还没就绪，一次复核可能不够；
  // 有界（仅冷启动路径一次），已同步时空转，不形成轮询。
  const COLD_START_RETRY_MS = 2500;

  function clearSyncTimer() {
    if (syncTimer === null) return;
    clearTimeout(syncTimer);
    syncTimer = null;
  }

  function clearRecheckTimer() {
    if (recheckTimer === null) return;
    clearTimeout(recheckTimer);
    recheckTimer = null;
  }

  function clearColdStartTimer() {
    if (coldStartTimer === null) return;
    clearTimeout(coldStartTimer);
    coldStartTimer = null;
  }

  function scheduleRecheck(reason: string, delayMs = VISIBLE_WAKE_RECHECK_MS) {
    clearRecheckTimer();
    recheckTimer = setTimeout(() => {
      recheckTimer = null;
      if (!isVisibleNow()) return;
      void sync(reason);
    }, Math.max(0, delayMs));
  }

  // 可见即前台：移动端从隐藏回被动（可见但暂无获焦）就该恢复，不等获焦。
  function isVisibleNow(): boolean {
    return (options.isEnabled?.() ?? true)
      && typeof document !== "undefined"
      && document.visibilityState === "visible";
  }

  // 完全活跃（可见+获焦）：只做标记，不做恢复门。录音/麦克风预热等仍用各自的严格判断。
  function isFullyActiveNow(): boolean {
    return isVisibleNow()
      && (typeof document.hasFocus !== "function" || document.hasFocus());
  }

  function isActiveNow(): boolean {
    return isFullyActiveNow();
  }

  async function sync(reason = "unknown") {
    // 先判边沿再占代次：空转的复核/连击不得 bump generation，
    // 否则会杀死正在进行的 restore（进程被杀后冷启动恢复慢，必被复核杀死）。
    const visible = isVisibleNow();
    if (options.activeSynced.value === visible) return;
    const generation = ++syncGeneration;
    // 边沿触发：只有隐藏<->可见翻转才跑完整恢复，可见内的 focus/pageshow 连击直接空转。
    options.activeSynced.value = visible;
    if (!visible) {
      clearRecheckTimer();
      clearColdStartTimer();
      options.onBackground?.(reason);
      await setTransportChatViewActive(false).catch(() => {});
      return;
    }
    try {
      await restoreTransportAfterForegroundWake();
      // 被更新的真实边沿超车：新边沿会自己负责恢复，这里直接让路，不碰标记。
      if (generation !== syncGeneration) return;
      // 恢复途中又切走：回到未同步，让下次可见重试，而不是卡在“已同步但没恢复”。
      if (!isVisibleNow()) {
        options.activeSynced.value = false;
        return;
      }
      await options.onWake(reason);
      if (generation !== syncGeneration) return;
      if (!isVisibleNow()) {
        options.activeSynced.value = false;
        return;
      }
      await setTransportChatViewActive(true);
    } catch (error) {
      // 失败时回到未同步，让延迟复核或下一次事件重试，而不是卡在“已同步但没恢复”。
      options.activeSynced.value = null;
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
    // 可见已同步时 focus 只是完全活跃标记，不重跑恢复，也不碰复核定时器。
    if (isVisibleNow() && options.activeSynced.value === true) return;
    schedule("focus");
    scheduleRecheck("focus_recheck");
  }

  function handleBlur() {
    options.onBackground?.("blur");
  }

  function handleVisibilityChange() {
    clearSyncTimer();
    if (typeof document !== "undefined" && document.visibilityState !== "visible") {
      clearRecheckTimer();
      options.onBackground?.("visibility_hidden");
      void sync("visibilitychange");
      return;
    }
    void sync("visibilitychange");
    scheduleRecheck("visibilitychange_recheck");
  }

  function handlePageShow(event?: { persisted?: boolean }) {
    const persisted = !!(event && (event as { persisted?: boolean }).persisted);
    // bfcache/冻结恢复时 JS 堆还在、同步标记还是旧值，必须强制重跑一次。
    if (persisted) options.activeSynced.value = null;
    void sync(persisted ? "pageshow_persisted" : "pageshow");
    if (isVisibleNow()) scheduleRecheck(persisted ? "pageshow_persisted_recheck" : "pageshow_recheck");
  }

  function handleResume() {
    // freeze 已把同步标记置 false，resume 到来时边沿判断自然重跑；
    // 这里不强制置 null，避免无可见翻转的 resume 绕过去重造成桌面重复唤醒。
    void sync("resume");
    if (isVisibleNow()) scheduleRecheck("resume_recheck");
  }

  function handleFreeze() {
    clearSyncTimer();
    clearRecheckTimer();
    clearColdStartTimer();
    options.activeSynced.value = false;
    options.onBackground?.("freeze");
    void setTransportChatViewActive(false).catch(() => {});
  }

  function handleOnline() {
    // 断网期间被杀后回来、传输失败时靠它补一次；可见内已同步则空转。
    void sync("online");
    if (isVisibleNow()) scheduleRecheck("online_recheck");
  }

  function handleColdStart(reason = "cold_start") {
    // 进程被杀后重载：JS 全新，无边沿事件可依赖，强制跑一次；
    // 挂载时视图/传输可能还没就绪，短复核管对焦时差、长重试管就绪时差，各一次、有界。
    clearColdStartTimer();
    options.activeSynced.value = null;
    void sync(reason);
    scheduleRecheck(`${reason}_recheck`);
    coldStartTimer = setTimeout(() => {
      coldStartTimer = null;
      if (options.activeSynced.value === true) return;
      if (!isVisibleNow()) return;
      void sync(`${reason}_retry`);
    }, COLD_START_RETRY_MS);
  }

  function cleanup() {
    ++syncGeneration;
    clearSyncTimer();
    clearRecheckTimer();
    clearColdStartTimer();
    options.activeSynced.value = null;
    options.onBackground?.("cleanup");
    void setTransportChatViewActive(false).catch(() => {});
  }

  return {
    isActiveNow,
    isVisibleNow,
    isFullyActiveNow,
    clearSyncTimer,
    clearRecheckTimer,
    clearColdStartTimer,
    sync,
    schedule,
    scheduleRecheck,
    handleFocus,
    handleBlur,
    handleVisibilityChange,
    handlePageShow,
    handleResume,
    handleFreeze,
    handleOnline,
    handleColdStart,
    cleanup,
  };
}
