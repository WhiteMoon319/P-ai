import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import { PANE_WIDTH_LIMITS } from "./use-chat-panes";

export type ChatWindowPaneSide = "left" | "right";

type ChatWindowPaneVisibility = {
  leftVisible: boolean;
  rightVisible: boolean;
  leftWidth: number;
  rightWidth: number;
};

type LockedCenterPane = {
  element: HTMLElement;
  flex: string;
  width: string;
};

export function normalizeExternalPaneCssWidth(side: ChatWindowPaneSide, width: number) {
  const limits = PANE_WIDTH_LIMITS[side];
  const normalized = Number.isFinite(width) ? Math.round(width) : limits.default;
  return Math.min(limits.max, Math.max(limits.min, normalized));
}

export function cssWidthToPhysical(widthCss: number, viewportCss: number, viewportPhysical: number, fallbackRatio: number) {
  const measuredRatio = viewportCss > 0 && viewportPhysical > 0
    ? viewportPhysical / viewportCss
    : fallbackRatio;
  const ratio = Number.isFinite(measuredRatio) && measuredRatio > 0 ? measuredRatio : 1;
  return Math.max(1, Math.round(widthCss * ratio));
}

export function useChatWindowPaneExpansion() {
  let operationQueue: Promise<unknown> = Promise.resolve();
  let lockedCenterPane: LockedCenterPane | null = null;

  function enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const next = operationQueue.then(operation, operation);
    operationQueue = next.then(() => undefined, () => undefined);
    return next;
  }

  function lockCenterPaneWidth() {
    if (lockedCenterPane) return;
    const element = document.querySelector<HTMLElement>('[data-chat-center-pane="true"]');
    if (!element) return;
    const width = element.getBoundingClientRect().width;
    if (!(width > 0)) return;
    lockedCenterPane = {
      element,
      flex: element.style.flex,
      width: element.style.width,
    };
    element.style.flex = `0 0 ${width}px`;
    element.style.width = `${width}px`;
  }

  function unlockCenterPaneWidth() {
    const locked = lockedCenterPane;
    lockedCenterPane = null;
    if (!locked) return;
    locked.element.style.flex = locked.flex;
    locked.element.style.width = locked.width;
  }

  function waitForLayoutFrame() {
    return new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  }

  async function panePhysicalWidth(side: ChatWindowPaneSide, width: number) {
    const widthCss = normalizeExternalPaneCssWidth(side, width);
    const innerSize = await getCurrentWindow().innerSize();
    return cssWidthToPhysical(
      widthCss,
      window.innerWidth,
      innerSize.width,
      window.devicePixelRatio,
    );
  }

  async function setExpanded(side: ChatWindowPaneSide, expanded: boolean, width: number) {
    if (!isTauriRuntimeAvailable()) return false;
    try {
      const widthPhysical = expanded ? await panePhysicalWidth(side, width) : 1;
      return await invokeTauri<boolean>("set_chat_window_side_expanded", {
        side,
        expanded,
        widthPhysical,
      });
    } catch (error) {
      console.warn(`[聊天侧栏] ${side === "left" ? "左侧" : "右侧"}栏窗口外扩调整失败`, error);
      return false;
    }
  }

  function beforeOpen(side: ChatWindowPaneSide, width: number) {
    return enqueue(async () => {
      lockCenterPaneWidth();
      const expanded = await setExpanded(side, true, width);
      if (!expanded) unlockCenterPaneWidth();
    });
  }

  function afterOpen() {
    return enqueue(async () => {
      await waitForLayoutFrame();
      unlockCenterPaneWidth();
    });
  }

  function beforeClose() {
    return enqueue(async () => lockCenterPaneWidth());
  }

  function afterClose(side: ChatWindowPaneSide) {
    return enqueue(async () => {
      try {
        await setExpanded(side, false, 1);
        await waitForLayoutFrame();
      } finally {
        unlockCenterPaneWidth();
      }
    });
  }

  function syncVisiblePanes(visibility: ChatWindowPaneVisibility) {
    return enqueue(async () => {
      lockCenterPaneWidth();
      try {
        if (visibility.leftVisible) {
          await setExpanded("left", true, visibility.leftWidth);
        }
        if (visibility.rightVisible) {
          await setExpanded("right", true, visibility.rightWidth);
        }
        await waitForLayoutFrame();
      } finally {
        unlockCenterPaneWidth();
      }
    });
  }

  function collapseVisiblePanes(visibility: Pick<ChatWindowPaneVisibility, "leftVisible" | "rightVisible">) {
    return enqueue(async () => {
      lockCenterPaneWidth();
      try {
        if (visibility.leftVisible) await setExpanded("left", false, 1);
        if (visibility.rightVisible) await setExpanded("right", false, 1);
        await waitForLayoutFrame();
      } finally {
        unlockCenterPaneWidth();
      }
    });
  }

  return {
    beforeOpen,
    afterOpen,
    beforeClose,
    afterClose,
    syncVisiblePanes,
    collapseVisiblePanes,
  };
}
