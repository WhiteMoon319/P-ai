import { computed, nextTick, onBeforeUnmount, onMounted, ref, type Ref, watch } from "vue";

const TODO_DROPDOWN_SAFE_GAP = 30;
const FLOATING_TOOLBAR_MIN_RESERVE = 24;

type UseChatScrollLayoutOptions = {
  activeConversationId: Ref<string>;
  chatting: Ref<boolean>;
  busy: Ref<boolean>;
  frozen: Ref<boolean>;
  timelineItemCount: Ref<number>;
  onReachedBottom: () => void;
  focusComposerInput: (options?: FocusOptions) => void;
};

export function useChatScrollLayout(options: UseChatScrollLayoutOptions) {
  const scrollContainer = ref<HTMLElement | null>(null);
  const composerContainer = ref<HTMLElement | null>(null);
  const toolbarContainer = ref<HTMLElement | null>(null);
  const chatLayoutRoot = ref<HTMLElement | null>(null);
  const latestOwnElasticMinHeight = ref(0);
  const jumpToBottomOffset = ref(96);
  const lastBottomState = ref(false);
  const lastScreenState = ref(false);
  const lastScrollTop = ref(0);
  const userScrollingDown = ref(false);
  const userScrollingUp = ref(false);
  let composerResizeObserver: ResizeObserver | null = null;
  let chatLayoutResizeObserver: ResizeObserver | null = null;
  let pendingComposerResizeFrame = 0;
  let pendingChatLayoutResizeFrame = 0;
  let wheelScrollIntentUntil = 0;
  let pointerScrollIntentActive = false;

  const showJumpToBottom = computed(() => !lastScreenState.value && userScrollingDown.value);
  const jumpToBottomStyle = computed(() => ({
    bottom: `${jumpToBottomOffset.value}px`,
  }));
  const jumpAboveBottomStyle = computed(() => ({
    bottom: `${jumpToBottomOffset.value + 44}px`,
  }));
  const toolbarReservedHeight = computed(() => {
    const measuredHeight = toolbarContainer.value?.offsetHeight ?? 0;
    if (measuredHeight <= 0) return 0;
    return Math.max(FLOATING_TOOLBAR_MIN_RESERVE, measuredHeight);
  });
  const floatingToolbarStyle = computed(() => ({
    bottom: `${jumpToBottomOffset.value}px`,
  }));

  function updateJumpToBottomOffset() {
    const composerHeight = composerContainer.value?.offsetHeight ?? 0;
    const nextOffset = Math.max(16, composerHeight + 12);
    if (jumpToBottomOffset.value !== nextOffset) {
      jumpToBottomOffset.value = nextOffset;
    }
  }

  function updateLatestOwnElasticMinHeight() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) {
      if (latestOwnElasticMinHeight.value !== 0) {
        latestOwnElasticMinHeight.value = 0;
      }
      return;
    }
    const scrollStyles = window.getComputedStyle(scrollEl);
    const scrollViewportHeight =
      scrollEl.clientHeight
      - parseFloat(scrollStyles.paddingTop || "0")
      - parseFloat(scrollStyles.paddingBottom || "0");
    const nextMinHeight = Math.max(0, scrollViewportHeight - toolbarReservedHeight.value - TODO_DROPDOWN_SAFE_GAP);
    if (latestOwnElasticMinHeight.value !== nextMinHeight) {
      latestOwnElasticMinHeight.value = nextMinHeight;
    }
  }

  async function prepareBottomAlignmentLayout() {
    updateJumpToBottomOffset();
    updateLatestOwnElasticMinHeight();
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    updateJumpToBottomOffset();
    updateLatestOwnElasticMinHeight();
    await nextTick();
  }

  function isNearBottom(el: HTMLElement): boolean {
    const threshold = 24;
    const distance = el.scrollHeight - (el.scrollTop + el.clientHeight);
    return distance <= threshold;
  }

  function isInLastScreen(el: HTMLElement): boolean {
    const distance = el.scrollHeight - (el.scrollTop + el.clientHeight);
    return distance <= el.clientHeight;
  }

  function updateScrollPositionState(el: HTMLElement, optionsOverride: { notifyReachedBottom?: boolean } = {}) {
    const nearBottom = isNearBottom(el);
    if (optionsOverride.notifyReachedBottom && nearBottom && !lastBottomState.value) {
      options.onReachedBottom();
    }
    if (nearBottom) {
      userScrollingDown.value = false;
      userScrollingUp.value = false;
    }
    lastBottomState.value = nearBottom;
    lastScreenState.value = isInLastScreen(el);
  }

  function onScroll() {
    const el = scrollContainer.value;
    if (!el) return;
    const nextScrollTop = el.scrollTop;
    const previousScrollTop = lastScrollTop.value;
    const userInitiatedScroll = pointerScrollIntentActive || Date.now() <= wheelScrollIntentUntil;
    if (userInitiatedScroll) {
      if (nextScrollTop > previousScrollTop) {
        userScrollingDown.value = true;
        userScrollingUp.value = false;
      } else if (nextScrollTop < previousScrollTop) {
        userScrollingDown.value = false;
        userScrollingUp.value = true;
      }
    }
    lastScrollTop.value = nextScrollTop;
    updateScrollPositionState(el, { notifyReachedBottom: true });
  }

  function noteWheelScrollIntent() {
    wheelScrollIntentUntil = Date.now() + 500;
  }

  function endPointerScrollIntent() {
    pointerScrollIntentActive = false;
    window.removeEventListener("pointerup", endPointerScrollIntent);
    window.removeEventListener("pointercancel", endPointerScrollIntent);
  }

  function beginPointerScrollIntent() {
    pointerScrollIntentActive = true;
    window.addEventListener("pointerup", endPointerScrollIntent);
    window.addEventListener("pointercancel", endPointerScrollIntent);
  }

  onMounted(() => {
    nextTick(() => {
      updateJumpToBottomOffset();
      updateLatestOwnElasticMinHeight();
      if (composerContainer.value && typeof ResizeObserver !== "undefined") {
        composerResizeObserver = new ResizeObserver(() => {
          if (typeof window === "undefined") {
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
            return;
          }
          if (pendingComposerResizeFrame) return;
          pendingComposerResizeFrame = window.requestAnimationFrame(() => {
            pendingComposerResizeFrame = 0;
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
          });
        });
        composerResizeObserver.observe(composerContainer.value);
      }
      if (chatLayoutRoot.value && typeof ResizeObserver !== "undefined") {
        chatLayoutResizeObserver = new ResizeObserver(() => {
          if (typeof window === "undefined") {
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
            return;
          }
          if (pendingChatLayoutResizeFrame) return;
          pendingChatLayoutResizeFrame = window.requestAnimationFrame(() => {
            pendingChatLayoutResizeFrame = 0;
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
          });
        });
        chatLayoutResizeObserver.observe(chatLayoutRoot.value);
      }
      const el = scrollContainer.value;
      if (el) {
        updateScrollPositionState(el);
        lastScrollTop.value = el.scrollTop;
        userScrollingDown.value = false;
        userScrollingUp.value = false;
      }
    });
  });

  onBeforeUnmount(() => {
    if (composerResizeObserver) {
      composerResizeObserver.disconnect();
      composerResizeObserver = null;
    }
    if (pendingComposerResizeFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(pendingComposerResizeFrame);
      pendingComposerResizeFrame = 0;
    }
    if (chatLayoutResizeObserver) {
      chatLayoutResizeObserver.disconnect();
      chatLayoutResizeObserver = null;
    }
    if (pendingChatLayoutResizeFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(pendingChatLayoutResizeFrame);
      pendingChatLayoutResizeFrame = 0;
    }
    if (typeof window !== "undefined") {
      window.removeEventListener("pointerup", endPointerScrollIntent);
      window.removeEventListener("pointercancel", endPointerScrollIntent);
    }
  });

  watch(
    options.chatting,
    (isChatting, wasChatting) => {
      if (wasChatting && !isChatting && !options.frozen.value && !options.busy.value) {
        nextTick(() => options.focusComposerInput({ preventScroll: true }));
      }
    },
  );

  watch(
    options.activeConversationId,
    () => {
      nextTick(() => {
        updateJumpToBottomOffset();
        updateLatestOwnElasticMinHeight();
        const el = scrollContainer.value;
        if (el) {
          updateScrollPositionState(el);
          lastScrollTop.value = el.scrollTop;
          userScrollingDown.value = false;
          userScrollingUp.value = false;
        }
      });
    },
    { immediate: true },
  );

  watch(
    options.timelineItemCount,
    () => {
      nextTick(() => {
        updateJumpToBottomOffset();
        updateLatestOwnElasticMinHeight();
        const el = scrollContainer.value;
        if (el) {
          updateScrollPositionState(el);
          lastScrollTop.value = el.scrollTop;
        }
      });
    },
  );

  return {
    scrollContainer,
    composerContainer,
    toolbarContainer,
    chatLayoutRoot,
    latestOwnElasticMinHeight,
    showJumpToBottom,
    atConversationBottom: lastBottomState,
    userScrollingDown,
    userScrollingUp,
    jumpToBottomStyle,
    jumpAboveBottomStyle,
    toolbarReservedHeight,
    floatingToolbarStyle,
    onScroll,
    noteWheelScrollIntent,
    beginPointerScrollIntent,
    prepareBottomAlignmentLayout,
  };
}
