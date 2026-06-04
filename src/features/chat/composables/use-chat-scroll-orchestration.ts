import { nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";

export interface UseChatScrollOrchestrationOptions {
  scrollContainer: Ref<HTMLElement | null>;
  chatScrollbarRef: Ref<{ updateThumb: () => void; hide?: () => void } | null>;
  activeJumpToBottomRequest?: Ref<number>;
  prepareBottomAlignmentLayout?: () => Promise<void> | void;
  onScroll: () => void;
  scheduleVirtualMeasure: () => void;
  syncViewportMetrics: () => void;
  resetConversationToBottom: () => void;
  alignItemToTop: (itemId: string, behavior?: ScrollBehavior) => void;
  captureVisibleAnchor: (edge: "top" | "bottom") => { messageId: string; edge: "top" | "bottom"; offset: number } | null;
  findRenderedMessageElement: (messageId: string) => HTMLElement | null;
  resolveMessageAnchorElement: (element: HTMLElement | null) => HTMLElement | null;
  syncVisibleStreamingVirtualItemViewportTops: () => void;
  refreshObservedVirtualItemElements: () => void;
  latestOwnElasticItemId: Ref<string>;
  props: {
    hasMoreHistory: Ref<boolean>;
    loadingOlderHistory: Ref<boolean>;
    chatting: Ref<boolean>;
    conversationBusy: Ref<boolean>;
    frozen: Ref<boolean>;
    activeConversationId: Ref<string>;
    conversationScrollToBottomRequest: Ref<number>;
    latestOwnMessageAlignRequest: Ref<number>;
    messageBlocks: Ref<ChatMessageBlock[]>;
  };
  emit: {
    loadOlderHistory: () => void;
    jumpToConversationBottom: () => void;
  };
}

export function useChatScrollOrchestration(options: UseChatScrollOrchestrationOptions) {
  const {
    scrollContainer,
    chatScrollbarRef,
    activeJumpToBottomRequest: sharedActiveJumpToBottomRequest,
    prepareBottomAlignmentLayout,
    onScroll,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    resetConversationToBottom,
    alignItemToTop,
    captureVisibleAnchor,
    findRenderedMessageElement,
    resolveMessageAnchorElement,
    syncVisibleStreamingVirtualItemViewportTops,
    refreshObservedVirtualItemElements,
    latestOwnElasticItemId,
    props,
    emit,
  } = options;

  const LOAD_OLDER_HISTORY_THRESHOLD_PX = 8;
  const activeJumpToBottomRequest = sharedActiveJumpToBottomRequest ?? ref(0);
  const olderHistoryRequestPending = ref(false);
  const suppressOlderHistoryPaginationOnce = ref(false);
  const pendingOlderHistoryAnchor = ref<{ messageId: string; edge: "top" | "bottom"; offset: number } | null>(null);
  const pendingOlderHistoryScrollRestore = ref<{ scrollTop: number; scrollHeight: number } | null>(null);
  let pendingProgrammaticScrollPaginationResetFrame = 0;
  let pendingAutoOlderHistoryFrame = 0;
  let autoOlderHistoryScheduled = false;

  function armProgrammaticScrollPaginationSuppression() {
    suppressOlderHistoryPaginationOnce.value = true;
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
    pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
      pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
        suppressOlderHistoryPaginationOnce.value = false;
        pendingProgrammaticScrollPaginationResetFrame = 0;
      });
    });
  }

  function maybeRequestOlderHistory() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    if (!props.hasMoreHistory.value || props.loadingOlderHistory.value || olderHistoryRequestPending.value) return;
    if (scrollEl.scrollTop > LOAD_OLDER_HISTORY_THRESHOLD_PX) return;
    pendingOlderHistoryScrollRestore.value = { scrollTop: scrollEl.scrollTop, scrollHeight: scrollEl.scrollHeight };
    pendingOlderHistoryAnchor.value = captureVisibleAnchor("bottom");
    olderHistoryRequestPending.value = true;
    emit.loadOlderHistory();
  }

  function scheduleAutoRequestOlderHistory() {
    if (autoOlderHistoryScheduled) return;
    autoOlderHistoryScheduled = true;
    void nextTick(() => {
      pendingAutoOlderHistoryFrame = requestAnimationFrame(() => {
        pendingAutoOlderHistoryFrame = 0;
        autoOlderHistoryScheduled = false;
        maybeRequestOlderHistory();
      });
    });
  }

  function onConversationScroll() {
    onScroll();
    chatScrollbarRef.value?.updateThumb();
    if (suppressOlderHistoryPaginationOnce.value) {
      suppressOlderHistoryPaginationOnce.value = false;
      if (pendingProgrammaticScrollPaginationResetFrame) {
        cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
        pendingProgrammaticScrollPaginationResetFrame = 0;
      }
    } else {
      maybeRequestOlderHistory();
    }
    syncVisibleStreamingVirtualItemViewportTops();
  }

  function doScrollToBottom() {
    activeJumpToBottomRequest.value += 1;
    armProgrammaticScrollPaginationSuppression();
    pendingOlderHistoryAnchor.value = null;
    pendingOlderHistoryScrollRestore.value = null;
    scheduleVirtualMeasure();
    void nextTick(async () => {
      await prepareBottomAlignmentLayout?.();
      resetConversationToBottom();
    });
  }

  function handleJumpToBottom() {
    doScrollToBottom();
    if (props.chatting.value || props.conversationBusy.value || props.frozen.value) return;
    emit.jumpToConversationBottom();
  }

  function alignLatestOwnMessageToTop(behavior: ScrollBehavior = "smooth") {
    alignItemToTop(latestOwnElasticItemId.value, behavior);
  }

  // ==================== watchers ====================

  watch(
    () => String(props.activeConversationId.value || "").trim(),
    () => {
      chatScrollbarRef.value?.hide?.();
      olderHistoryRequestPending.value = false;
      pendingOlderHistoryAnchor.value = null;
      pendingOlderHistoryScrollRestore.value = null;
      armProgrammaticScrollPaginationSuppression();
      void prepareBottomAlignmentLayout?.();
      scheduleAutoRequestOlderHistory();
    },
    { immediate: true },
  );

  watch(
    () => props.conversationScrollToBottomRequest.value,
    (nextValue, prevValue) => {
      if (!nextValue || nextValue === prevValue) return;
      doScrollToBottom();
    },
  );

  watch(
    () => props.latestOwnMessageAlignRequest.value,
    (nextValue, prevValue) => {
      if (!nextValue || nextValue === prevValue) return;
      void nextTick(async () => {
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        refreshObservedVirtualItemElements();
        alignLatestOwnMessageToTop("smooth");
      });
    },
  );

  watch(
    () => props.messageBlocks.value,
    () => {
      refreshObservedVirtualItemElements();
      void nextTick(() => {
        chatScrollbarRef.value?.updateThumb();
        scheduleAutoRequestOlderHistory();
      });
    },
  );

  watch(
    () => props.loadingOlderHistory.value,
    async (loading, wasLoading) => {
      if (loading) return;
      if (!wasLoading) return;
      const scrollEl = scrollContainer.value;
      if (!scrollEl) return;
      await nextTick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      await nextTick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      refreshObservedVirtualItemElements();
      const scrollRestore = pendingOlderHistoryScrollRestore.value;
      if (scrollRestore) {
        const deltaHeight = scrollEl.scrollHeight - scrollRestore.scrollHeight;
        scrollEl.scrollTop = Math.max(0, scrollRestore.scrollTop + deltaHeight);
      }
      const anchor = pendingOlderHistoryAnchor.value;
      if (anchor) {
        const anchorMessageElement = findRenderedMessageElement(anchor.messageId);
        const anchorElement = resolveMessageAnchorElement(anchorMessageElement);
        if (anchorElement && anchorElement.isConnected) {
          const containerRect = scrollEl.getBoundingClientRect();
          const rect = anchorElement.getBoundingClientRect();
          if (anchor.edge === "bottom") {
            scrollEl.scrollTop += anchor.offset - (containerRect.bottom - rect.bottom);
          } else {
            scrollEl.scrollTop += (rect.top - containerRect.top) - anchor.offset;
          }
        }
      }
      olderHistoryRequestPending.value = false;
      pendingOlderHistoryAnchor.value = null;
      pendingOlderHistoryScrollRestore.value = null;
      scheduleAutoRequestOlderHistory();
    },
  );

  onBeforeUnmount(() => {
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
    if (pendingAutoOlderHistoryFrame) {
      cancelAnimationFrame(pendingAutoOlderHistoryFrame);
      pendingAutoOlderHistoryFrame = 0;
    }
    autoOlderHistoryScheduled = false;
  });

  return {
    activeJumpToBottomRequest,
    onConversationScroll,
    handleJumpToBottom,
    alignLatestOwnMessageToTop,
    activeConversationChangedCleanup: () => {
      olderHistoryRequestPending.value = false;
      pendingOlderHistoryAnchor.value = null;
      pendingOlderHistoryScrollRestore.value = null;
    },
    armProgrammaticScrollPaginationSuppression,
  };
}
