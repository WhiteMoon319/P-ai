import { ref } from "vue";
import {
  currentTransportWindowIsAlwaysOnTop,
  currentTransportWindowIsMaximized,
  getCurrentTransportWindowRole,
  hideCurrentTransportWindow,
  minimizeCurrentTransportWindow,
  setCurrentTransportWindowAlwaysOnTop,
  startCurrentTransportWindowDragging,
  toggleCurrentTransportWindowMaximize,
} from "../../../services/tauri-api";

export function useWindowShell() {
  const windowReady = ref(false);
  const alwaysOnTop = ref(false);
  const maximized = ref(false);

  function initWindow(): "chat" | "archives" | "config" {
    const role = getCurrentTransportWindowRole();
    windowReady.value = true;
    void syncWindowControlsState();
    return role;
  }

  async function syncWindowControlsState() {
    try {
      alwaysOnTop.value = await currentTransportWindowIsAlwaysOnTop();
    } catch {
      alwaysOnTop.value = false;
    }
    try {
      maximized.value = await currentTransportWindowIsMaximized();
    } catch {
      maximized.value = false;
    }
  }

  async function closeWindow() {
    try {
      await hideCurrentTransportWindow();
    } catch (error) {
      console.error("[窗口] 隐藏当前窗口失败", error);
    }
  }

  async function startDrag() {
    try {
      await startCurrentTransportWindowDragging();
      await syncWindowControlsState();
    } catch (error) {
      console.error("[窗口] 开始拖动当前窗口失败", error);
    }
  }

  async function toggleAlwaysOnTop() {
    const desired = !alwaysOnTop.value;
    try {
      await setCurrentTransportWindowAlwaysOnTop(desired);
      alwaysOnTop.value = desired;
    } catch (error) {
      console.error("[窗口] setAlwaysOnTop failed:", error);
    }
  }

  async function minimizeWindow() {
    try {
      await minimizeCurrentTransportWindow();
    } catch (error) {
      console.error("[窗口] minimize failed:", error);
    }
  }

  async function toggleMaximizeWindow() {
    try {
      maximized.value = await toggleCurrentTransportWindowMaximize();
    } catch (error) {
      console.error("[窗口] 切换最大化失败", error);
    }
  }

  return {
    windowReady,
    alwaysOnTop,
    maximized,
    initWindow,
    syncWindowControlsState,
    closeWindow,
    startDrag,
    toggleAlwaysOnTop,
    minimizeWindow,
    toggleMaximizeWindow,
  };
}
