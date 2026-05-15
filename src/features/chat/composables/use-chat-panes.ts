import { nextTick, ref, watch, type Ref } from "vue";

type PaneResizeSide = "left" | "right";

export const PANE_WIDTH_LIMITS = {
  left: { min: 200, max: 10000, default: 320 },
  right: { min: 350, max: 10000, default: 320 },
} as const;

export const PANE_CENTER_MIN_WIDTH = 100;
export const PANE_WIDTH_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_width.v1",
  right: "easy_call.chat_right_sidebar_width.v1",
} as const;
const LEGACY_PANE_WIDTH_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-width",
  right: "easy-call.chat.right-sidebar-width",
} as const;

export function loadStoredPaneWidth(side: PaneResizeSide): number {
  if (typeof window === "undefined") return PANE_WIDTH_LIMITS[side].default;
  const raw = window.localStorage.getItem(PANE_WIDTH_STORAGE_KEYS[side])
    ?? window.localStorage.getItem(LEGACY_PANE_WIDTH_STORAGE_KEYS[side]);
  const parsed = Number(raw);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : PANE_WIDTH_LIMITS[side].default;
}

export interface UseChatPanesOptions {
  chatLayoutRoot: Ref<HTMLElement | null>;
  toolReviewPanelOpen: Ref<boolean>;
  showSideConversationList: Ref<boolean>;
  detachedChatWindow: boolean;
  syncViewportMetrics: () => void;
  onPaneWidthsChange: (leftWidth: number, rightWidth: number) => void;
  onPaneWidthsCommit: (leftWidth: number, rightWidth: number) => void;
  onBeforeUnmountCleanup: (fn: () => void) => void;
}

export function useChatPanes(options: UseChatPanesOptions) {
  const { chatLayoutRoot, toolReviewPanelOpen, showSideConversationList, detachedChatWindow, syncViewportMetrics, onPaneWidthsChange, onPaneWidthsCommit, onBeforeUnmountCleanup } = options;

  const leftSidebarWidth = ref(loadStoredPaneWidth("left"));
  const rightSidebarWidth = ref(loadStoredPaneWidth("right"));
  const activePaneResizeSide = ref<PaneResizeSide | null>(null);
  let paneResizeStartX = 0;
  let paneResizeStartWidth = 0;
  let paneResizePreviousBodyCursor = "";
  let paneResizePreviousBodyUserSelect = "";

  function storePaneWidth(side: PaneResizeSide, width: number) {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEYS[side], String(Math.round(width)));
  }

  function clampPaneWidth(side: PaneResizeSide, width: number): number {
    const limits = PANE_WIDTH_LIMITS[side];
    const layoutWidth = chatLayoutRoot.value?.getBoundingClientRect().width || 0;
    const otherPaneWidth =
      side === "left"
        ? (toolReviewPanelOpen.value ? rightSidebarWidth.value : 0)
        : (showSideConversationList.value && !detachedChatWindow ? leftSidebarWidth.value : 0);
    const layoutMax = layoutWidth > 0 ? layoutWidth - otherPaneWidth - PANE_CENTER_MIN_WIDTH : limits.max;
    const effectiveMax = Math.max(limits.min, layoutMax > 0 ? layoutMax : limits.max);
    return Math.round(Math.min(effectiveMax, Math.max(limits.min, width)));
  }

  async function setPaneWidth(side: PaneResizeSide, width: number, persist = false) {
    const nextWidth = clampPaneWidth(side, width);
    if (side === "left") {
      leftSidebarWidth.value = nextWidth;
    } else {
      rightSidebarWidth.value = nextWidth;
    }
    if (persist) storePaneWidth(side, nextWidth);
    await nextTick();
    syncViewportMetrics();
  }

  function startPaneResize(side: PaneResizeSide, event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    activePaneResizeSide.value = side;
    paneResizeStartX = event.clientX;
    paneResizeStartWidth = side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value;
    paneResizePreviousBodyCursor = document.body.style.cursor;
    paneResizePreviousBodyUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", handlePaneResizeMove);
    window.addEventListener("pointerup", stopPaneResize, { once: true });
    window.addEventListener("pointercancel", stopPaneResize, { once: true });
  }

  function handlePaneResizeMove(event: PointerEvent) {
    const side = activePaneResizeSide.value;
    if (!side) return;
    const pointerDelta = event.clientX - paneResizeStartX;
    const nextWidth = side === "left" ? paneResizeStartWidth + pointerDelta : paneResizeStartWidth - pointerDelta;
    const nextWidthClamped = clampPaneWidth(side, nextWidth);
    if (side === "left") {
      leftSidebarWidth.value = nextWidthClamped;
    } else {
      rightSidebarWidth.value = nextWidthClamped;
    }
  }

  async function stopPaneResize() {
    const side = activePaneResizeSide.value;
    window.removeEventListener("pointermove", handlePaneResizeMove);
    window.removeEventListener("pointerup", stopPaneResize);
    window.removeEventListener("pointercancel", stopPaneResize);
    document.body.style.cursor = paneResizePreviousBodyCursor;
    document.body.style.userSelect = paneResizePreviousBodyUserSelect;
    activePaneResizeSide.value = null;
    if (side) {
      storePaneWidth(side, side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value);
      await nextTick();
      syncViewportMetrics();
      onPaneWidthsCommit(leftSidebarWidth.value, rightSidebarWidth.value);
    }
  }

  function adjustPaneWidthByKeyboard(side: PaneResizeSide, delta: number) {
    const currentWidth = side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value;
    setPaneWidth(side, currentWidth + delta, true);
    onPaneWidthsCommit(leftSidebarWidth.value, rightSidebarWidth.value);
  }

  watch(
    [leftSidebarWidth, rightSidebarWidth],
    ([leftWidth, rightWidth]) => {
      onPaneWidthsChange(leftWidth, rightWidth);
    },
    { immediate: true },
  );

  onBeforeUnmountCleanup(stopPaneResize);

  return {
    leftSidebarWidth,
    rightSidebarWidth,
    activePaneResizeSide,
    clampPaneWidth,
    setPaneWidth,
    startPaneResize,
    adjustPaneWidthByKeyboard,
  };
}
