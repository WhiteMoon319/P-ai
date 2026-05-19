import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invokeTauri } from "../../../services/tauri-api";

type UseAppLifecycleOptions = {
  appBootstrapMount: () => Promise<void>;
  appBootstrapUnmount: () => void;
  restoreThemeFromStorage: () => void;
  onPaste: (event: ClipboardEvent) => void;
  onDragOver: (event: DragEvent) => void;
  onDrop: (event: DragEvent) => void;
  onNativeFileDrop?: (paths: string[]) => Promise<void> | void;
  onNativeDragState?: (active: boolean) => void;
  recordHotkeyMount: () => void;
  recordHotkeyUnmount: () => void;
  beforeRefreshData?: () => Promise<void> | void;
  afterSafetyGateReady?: () => Promise<void> | void;
  refreshAllViewData: () => Promise<void>;
  afterRefreshData?: () => Promise<void> | void;
  viewMode: Ref<"chat" | "archives" | "config">;
  syncWindowControlsState: () => Promise<void>;
  stopRecording: (discard: boolean) => Promise<void>;
  cleanupSpeechRecording: () => void;
  cleanupChatMedia: () => Promise<void>;
  afterMountedReady?: () => Promise<void> | void;
  onStartupStepFailed?: (label: string, error: unknown) => void;
};

const STARTUP_STEP_TIMEOUT_MS = 10_000;
const BACKEND_READY_TIMEOUT_MS = 30_000;

function startupTimeoutError(label: string): Error {
  return new Error(`启动步骤超时：${label} 超过 ${STARTUP_STEP_TIMEOUT_MS / 1000} 秒未完成，已跳过。`);
}

async function runStartupStep(
  label: string,
  task: () => Promise<void> | void,
  onFailed?: (label: string, error: unknown) => void,
): Promise<boolean> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  try {
    await Promise.race([
      Promise.resolve().then(task),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(startupTimeoutError(label)), STARTUP_STEP_TIMEOUT_MS);
      }),
    ]);
    return true;
  } catch (error) {
    console.error(`[LIFECYCLE] startup step failed: ${label}`, error);
    onFailed?.(label, error);
    return false;
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * 等待后端就绪信号。先查询当前状态（处理窗口晚于 setup 完成的情况），
 * 如果未就绪则监听事件等待。
 */
async function waitForBackendReady(): Promise<void> {
  try {
    const ready = await invokeTauri<boolean>("is_backend_ready");
    if (ready) {
      console.info("[LIFECYCLE] 后端已就绪（轮询确认）");
      return;
    }
  } catch {
    // invoke 失败说明后端还没完全初始化 IPC，继续等事件
  }
  return new Promise<void>((resolve, reject) => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    let unlisten: (() => void) | null = null;
    const cleanup = () => {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
    timer = setTimeout(() => {
      cleanup();
      reject(new Error(`等待后端就绪超时（${BACKEND_READY_TIMEOUT_MS / 1000}秒）`));
    }, BACKEND_READY_TIMEOUT_MS);
    listen("easy-call:backend-ready", () => {
      cleanup();
      console.info("[LIFECYCLE] 后端已就绪（事件通知）");
      resolve();
    })
      .then((fn) => {
        unlisten = fn;
        // 注册监听后再查一次，防止事件在注册前已发出
        invokeTauri<boolean>("is_backend_ready")
          .then((ready) => {
            if (ready) {
              cleanup();
              console.info("[LIFECYCLE] 后端已就绪（二次轮询确认）");
              resolve();
            }
          })
          .catch(() => {});
      })
      .catch((error) => {
        cleanup();
        reject(error);
      });
  });
}

export function useAppLifecycle(options: UseAppLifecycleOptions) {
  onMounted(async () => {
    // 等待后端完成初始化，避免多窗口并发请求导致死锁
    const backendReady = await runStartupStep(
      "waitForBackendReady",
      () => waitForBackendReady(),
      options.onStartupStepFailed,
    );
    if (!backendReady) return;

    const bootstrapMounted = await runStartupStep(
      "appBootstrapMount",
      () => options.appBootstrapMount(),
      options.onStartupStepFailed,
    );
    if (!bootstrapMounted) return;
    options.restoreThemeFromStorage();
    window.addEventListener("paste", options.onPaste);
    window.addEventListener("dragover", options.onDragOver);
    window.addEventListener("drop", options.onDrop);
    options.recordHotkeyMount();
    try {
      await options.beforeRefreshData?.();
    } catch (error) {
      console.error("[LIFECYCLE] startup safety gate failed: beforeRefreshData", error);
      options.onStartupStepFailed?.("beforeRefreshData", error);
      return;
    }
    const backendReadyNotified = await runStartupStep(
      "afterSafetyGateReady",
      () => options.afterSafetyGateReady?.(),
      options.onStartupStepFailed,
    );
    if (!backendReadyNotified) return;
    try {
      await options.refreshAllViewData();
    } catch (error) {
      console.error("[LIFECYCLE] startup refresh failed: refreshAllViewData", error);
      options.onStartupStepFailed?.("refreshAllViewData", error);
      return;
    }
    const afterRefreshCompleted = await runStartupStep(
      "afterRefreshData",
      () => options.afterRefreshData?.(),
      options.onStartupStepFailed,
    );
    if (!afterRefreshCompleted) return;
    if (options.viewMode.value === "chat") {
      const windowControlsSynced = await runStartupStep(
        "syncWindowControlsState",
        () => options.syncWindowControlsState(),
        options.onStartupStepFailed,
      );
      if (!windowControlsSynced) return;
    }
    await runStartupStep(
      "afterMountedReady",
      () => options.afterMountedReady?.(),
      options.onStartupStepFailed,
    );
  });

  onBeforeUnmount(() => {
    options.appBootstrapUnmount();
    void options.stopRecording(true);
    options.cleanupSpeechRecording();
    options.recordHotkeyUnmount();
    void options.cleanupChatMedia();
    window.removeEventListener("paste", options.onPaste);
    window.removeEventListener("dragover", options.onDragOver);
    window.removeEventListener("drop", options.onDrop);
  });
}
