import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import type { ChatMessageBlock } from "../../../types/app";
import type { ChatRenderItem } from "../utils/chat-render";
import { estimateChatRenderItemHeight } from "./use-chat-virtual-list";

interface UseChatVirtualScrollOptions {
  renderItems: Ref<ChatRenderItem[]>;
  renderItemById: Ref<Map<string, ChatRenderItem>>;
  blockChronologicalIndexMap: Ref<Map<string, number>>;
  scrollContainer: Ref<HTMLElement | null>;
  scrollbarRef: Ref<{ updateThumb: () => void } | null>;
  activeJumpToBottomRequest: Ref<number>;
  activeConversationId: Ref<string>;
  latestOwnElasticItemId: Ref<string>;
  latestOwnElasticMinHeight: Ref<number>;
  debugEnabled?: Ref<boolean> | boolean;
  smoothScrollEnabled?: Ref<boolean> | boolean;
  onUserScroll: () => void;
}

export function useChatVirtualScroll(options: UseChatVirtualScrollOptions) {
  const {
    renderItems,
    renderItemById,
    blockChronologicalIndexMap,
    scrollContainer,
    scrollbarRef,
    activeJumpToBottomRequest,
    activeConversationId,
    latestOwnElasticItemId,
    latestOwnElasticMinHeight,
    debugEnabled,
    smoothScrollEnabled,
    onUserScroll,
  } = options;

  const observedVirtualItemElements = new Map<string, HTMLElement>();
  const observedVirtualItemResizeElements = new Map<string, HTMLElement>();
  const measuredVirtualItemHeights = new Map<string, number>();
  const streamingVirtualItemViewportTop = new Map<string, number>();

  let pendingMeasureFrame = 0;
  let pendingVirtualResizeFrame = 0;
  const pendingVirtualResizeElements = new Set<HTMLElement>();
  let virtualItemResizeObserver: ResizeObserver | null = null;

  let pendingJumpToBottomFrame = 0;
  const initialBottomOffset = ref(0);
  let conversationVirtualizerResetRequest = 0;
  let debugTraceRequest = 0;

  // ==================== virtualizer ====================

  function estimateRenderItemSize(index: number): number {
    const item = renderItems.value[index];
    const estimatedHeight = estimateChatRenderItemHeight(item);
    return item?.id === latestOwnElasticItemId.value
      ? Math.max(estimatedHeight, latestOwnElasticMinHeight.value)
      : estimatedHeight;
  }

  function estimateTotalRenderSize(): number {
    return renderItems.value.reduce((total, _item, index) => total + estimateRenderItemSize(index), 0);
  }

  function chatVirtualScrollDebugEnabled(): boolean {
    if (typeof window === "undefined") return false;
    const configuredDebugEnabled = typeof debugEnabled === "object" && debugEnabled && "value" in debugEnabled
      ? debugEnabled.value
      : debugEnabled;
    if (configuredDebugEnabled === false) return false;
    return window.localStorage.getItem("easy-call.debug.chat-virtual-scroll") === "1"
      || (window as any).__easyCallDebugChatVirtualScroll === true;
  }

  function nativeSmoothScrollEnabled(): boolean {
    const configured = typeof smoothScrollEnabled === "object" && smoothScrollEnabled && "value" in smoothScrollEnabled
      ? smoothScrollEnabled.value
      : smoothScrollEnabled;
    return configured !== false;
  }

  function debugVirtualScrollState(label: string) {
    const scrollEl = scrollContainer.value;
    const rows = virtualizer.value.getVirtualItems();
    const firstRow = rows[0];
    const lastRow = rows[rows.length - 1];
    const range = rows.length > 0 ? `${firstRow?.index}-${lastRow?.index}` : "empty";
    const scrollTop = Math.round(scrollEl?.scrollTop ?? 0);
    const scrollHeight = Math.round(scrollEl?.scrollHeight ?? 0);
    const clientHeight = Math.round(scrollEl?.clientHeight ?? 0);
    const distanceToBottom = Math.round(scrollHeight - scrollTop - clientHeight);
    console.warn(
      `[聊天虚拟滚动] ${label}`
      + ` count=${renderItems.value.length}`
      + ` range=${range}`
      + ` scroll=${scrollTop}/${clientHeight}/${scrollHeight}`
      + ` bottom=${distanceToBottom}`
      + ` init=${Math.round(initialBottomOffset.value)}`
      + ` est=${Math.round(estimateTotalRenderSize())}`
      + ` total=${Math.round(virtualizer.value.getTotalSize())}`
      + ` elastic=${latestOwnElasticItemId.value ? "yes" : "no"}:${Math.round(latestOwnElasticMinHeight.value)}`,
    );
  }

  function traceVirtualScrollFrames(label: string, frameCount = 6) {
    if (!chatVirtualScrollDebugEnabled()) return;
    const requestId = ++debugTraceRequest;
    let frame = 0;
    const tick = () => {
      if (requestId !== debugTraceRequest) return;
      debugVirtualScrollState(`${label}:frame-${frame}`);
      frame += 1;
      if (frame >= frameCount) return;
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }

  const virtualizer = useVirtualizer(
    computed(() => ({
      count: renderItems.value.length,
      getScrollElement: () => scrollContainer.value,
      getItemKey: (index: number) => renderItems.value[index]?.id ?? `row-${index}`,
      estimateSize: estimateRenderItemSize,
      initialOffset: () => initialBottomOffset.value,
      measureElement: (element: Element, _entry: unknown, instance: any) => {
        const measuredHeight = (element as HTMLElement).scrollHeight;
        if (instance?.scrollDirection !== "backward") return measuredHeight;
        const indexAttr = Number((element as HTMLElement).getAttribute("data-index"));
        // NOTE: depends on @tanstack/vue-virtual internal property `itemSizeCache` (not part of public API).
        // TODO: validate `itemSizeCache` availability after any @tanstack/virtual upgrade.
        const cachedHeight = Number.isFinite(indexAttr) ? instance?.itemSizeCache?.get(indexAttr) : undefined;
        return typeof cachedHeight === "number" ? cachedHeight : measuredHeight;
      },
      overscan: 4,
    })),
  );

  const virtualRows = computed(() => virtualizer.value.getVirtualItems());
  const virtualEntries = computed(() =>
    virtualRows.value
      .map((row) => {
        const item = renderItems.value[row.index];
        return item ? { row, item } : null;
      })
      .filter((entry): entry is { row: typeof virtualRows.value[number]; item: ChatRenderItem } => Boolean(entry)),
  );
  const totalVirtualSize = computed(() => virtualizer.value.getTotalSize());
  const virtualDebugVisible = computed(() => chatVirtualScrollDebugEnabled());
  const virtualDebugState = computed(() => {
    const scrollEl = scrollContainer.value;
    const rows = virtualizer.value.getVirtualItems();
    const firstRow = rows[0];
    const lastRow = rows[rows.length - 1];
    const firstItem = firstRow ? renderItems.value[firstRow.index] : undefined;
    const lastItem = lastRow ? renderItems.value[lastRow.index] : undefined;
    return {
      conversationId: String(activeConversationId.value || "").trim(),
      itemCount: renderItems.value.length,
      initialBottomOffset: Math.round(initialBottomOffset.value),
      estimatedTotal: Math.round(estimateTotalRenderSize()),
      totalSize: Math.round(virtualizer.value.getTotalSize()),
      scrollTop: Math.round(scrollEl?.scrollTop ?? 0),
      scrollHeight: Math.round(scrollEl?.scrollHeight ?? 0),
      clientHeight: Math.round(scrollEl?.clientHeight ?? 0),
      range: rows.length > 0 ? `${firstRow?.index}-${lastRow?.index}` : "empty",
      firstItemId: firstItem?.id || "",
      lastItemId: lastItem?.id || "",
      latestOwnElasticItemId: latestOwnElasticItemId.value,
      latestOwnElasticMinHeight: Math.round(latestOwnElasticMinHeight.value),
    };
  });

  // ==================== helpers ====================

  function isStreamingAssistantBlock(block: ChatMessageBlock): boolean {
    return !!block.isStreaming && !(block as any).isRightAligned;
  }

  function renderItemContainsStreamingAssistant(item: ChatRenderItem | undefined): boolean {
    if (!item) return false;
    if (item.kind === "message") return isStreamingAssistantBlock(item.block);
    if (item.kind === "group") return item.items.some((g) => isStreamingAssistantBlock(g.block));
    return false;
  }

  function elementTopInScrollViewport(scrollEl: HTMLElement, element: HTMLElement): number {
    const containerRect = scrollEl.getBoundingClientRect();
    return element.getBoundingClientRect().top - containerRect.top;
  }

  function elementVisibleInScrollViewport(scrollEl: HTMLElement, element: HTMLElement): boolean {
    const containerRect = scrollEl.getBoundingClientRect();
    const rect = element.getBoundingClientRect();
    return rect.bottom > containerRect.top + 1 && rect.top < containerRect.bottom - 1;
  }

  function isNearBottomForStability(scrollEl: HTMLElement): boolean {
    return scrollEl.scrollHeight - (scrollEl.scrollTop + scrollEl.clientHeight) <= 24;
  }

  function updateStreamingVirtualItemViewportTop(itemId: string, element?: HTMLElement | null) {
    const normalizedItemId = String(itemId || "").trim();
    if (!normalizedItemId) return;
    const scrollEl = scrollContainer.value;
    const target = element || observedVirtualItemElements.get(normalizedItemId) || null;
    const item = renderItemById.value.get(normalizedItemId);
    if (!scrollEl || !target || !target.isConnected || !renderItemContainsStreamingAssistant(item) || !elementVisibleInScrollViewport(scrollEl, target)) {
      streamingVirtualItemViewportTop.delete(normalizedItemId);
      return;
    }
    streamingVirtualItemViewportTop.set(normalizedItemId, elementTopInScrollViewport(scrollEl, target));
  }

  function syncVisibleStreamingVirtualItemViewportTops() {
    for (const [itemId, element] of observedVirtualItemElements.entries()) {
      updateStreamingVirtualItemViewportTop(itemId, element);
    }
  }

  // ==================== resize handling ====================

  function handleVirtualItemResize(element: HTMLElement) {
    const itemId = String(element.getAttribute("data-render-item-id") || "").trim();
    if (!itemId) return;
    const scrollEl = scrollContainer.value;
    const previousTop = streamingVirtualItemViewportTop.get(itemId);
    virtualizer.value.measureElement(element);
    const nextHeight = Math.round(element.getBoundingClientRect().height);
    measuredVirtualItemHeights.set(itemId, nextHeight);
    observedVirtualItemElements.set(itemId, element);
    if (
      scrollEl
      && previousTop !== undefined
      && !isNearBottomForStability(scrollEl)
      && renderItemContainsStreamingAssistant(renderItemById.value.get(itemId))
      && elementVisibleInScrollViewport(scrollEl, element)
    ) {
      const nextTop = elementTopInScrollViewport(scrollEl, element);
      const delta = nextTop - previousTop;
      if (Math.abs(delta) >= 1) {
        scrollEl.scrollTop += delta;
        onUserScroll();
      }
    }
    updateStreamingVirtualItemViewportTop(itemId, element);
  }

  function scheduleVirtualMeasure() {
    if (pendingMeasureFrame) return;
    void nextTick(() => {
      if (pendingMeasureFrame) return;
      pendingMeasureFrame = requestAnimationFrame(() => {
        pendingMeasureFrame = 0;
        refreshObservedVirtualItemElements();
        syncVisibleStreamingVirtualItemViewportTops();
        virtualizer.value.measure();
        syncVisibleStreamingVirtualItemViewportTops();
        if (activeJumpToBottomRequest.value) scrollConversationToBottomOnce();
      });
    });
  }

  function scheduleVirtualResizeMeasure(entries: ResizeObserverEntry[]) {
    for (const entry of entries) {
      if (entry.target instanceof HTMLElement) {
        pendingVirtualResizeElements.add(entry.target);
      }
    }
    if (pendingVirtualResizeFrame) return;
    pendingVirtualResizeFrame = requestAnimationFrame(() => {
      pendingVirtualResizeFrame = 0;
      const elements = Array.from(pendingVirtualResizeElements);
      pendingVirtualResizeElements.clear();
      for (const element of elements) {
        if (!element.isConnected) continue;
        handleVirtualItemResize(element);
      }
    });
  }

  // ==================== measurement ====================

  function measureVirtualRow(itemId: string, element: Element | { $el?: Element } | null) {
    const normalizedItemId = String(itemId || "").trim();
    if (!element) {
      if (normalizedItemId) {
        const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
        if (previousResizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(previousResizeElement);
        }
        observedVirtualItemResizeElements.delete(normalizedItemId);
        observedVirtualItemElements.delete(normalizedItemId);
        measuredVirtualItemHeights.delete(normalizedItemId);
        streamingVirtualItemViewportTop.delete(normalizedItemId);
      }
      return;
    }
    const target = element instanceof Element ? element : ((element as any).$el as Element | undefined) ?? null;
    if (!target) {
      if (normalizedItemId) {
        const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
        if (previousResizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(previousResizeElement);
        }
        observedVirtualItemResizeElements.delete(normalizedItemId);
        observedVirtualItemElements.delete(normalizedItemId);
        measuredVirtualItemHeights.delete(normalizedItemId);
        streamingVirtualItemViewportTop.delete(normalizedItemId);
      }
      return;
    }
    virtualizer.value.measureElement(target);
    const resolvedItemId = normalizedItemId || String(target.getAttribute("data-render-item-id") || "").trim();
    if (resolvedItemId && target instanceof HTMLElement) {
      const previousResizeElement = observedVirtualItemResizeElements.get(resolvedItemId);
      if (previousResizeElement && previousResizeElement !== target && virtualItemResizeObserver) {
        virtualItemResizeObserver.unobserve(previousResizeElement);
      }
      if (virtualItemResizeObserver && previousResizeElement !== target) {
        virtualItemResizeObserver.observe(target);
      }
      observedVirtualItemResizeElements.set(resolvedItemId, target);
      const nextHeight = Math.round(target.getBoundingClientRect().height);
      measuredVirtualItemHeights.set(resolvedItemId, nextHeight);
      observedVirtualItemElements.set(resolvedItemId, target);
      updateStreamingVirtualItemViewportTop(resolvedItemId, target);
    }
  }

  function refreshObservedVirtualItemElements() {
    const validIds = new Set<string>();
    for (const entry of virtualEntries.value) {
      const itemId = String(entry.item.id || "").trim();
      if (!itemId) continue;
      validIds.add(itemId);
      if (entry.item.kind === "message" || entry.item.kind === "group") {
        const blocks = entry.item.kind === "message" ? [entry.item.block] : entry.item.items.map((g) => g.block);
        blocks.forEach((block) => {
          const blockId = String(block.id || "").trim();
          if (blockId) validIds.add(blockId);
        });
      }
    }
    for (const [itemId] of observedVirtualItemElements.entries()) {
      if (!validIds.has(itemId)) {
        const resizeElement = observedVirtualItemResizeElements.get(itemId);
        if (resizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(resizeElement);
        }
        observedVirtualItemResizeElements.delete(itemId);
        observedVirtualItemElements.delete(itemId);
        measuredVirtualItemHeights.delete(itemId);
        streamingVirtualItemViewportTop.delete(itemId);
      }
    }
  }

  function clearMeasuredVirtualState() {
    for (const element of observedVirtualItemResizeElements.values()) {
      virtualItemResizeObserver?.unobserve(element);
    }
    observedVirtualItemElements.clear();
    observedVirtualItemResizeElements.clear();
    measuredVirtualItemHeights.clear();
    streamingVirtualItemViewportTop.clear();
    pendingVirtualResizeElements.clear();
  }

  function resetVirtualizerAtConversationBottom() {
    const requestId = ++conversationVirtualizerResetRequest;
    activeJumpToBottomRequest.value = 0;
    clearMeasuredVirtualState();
    initialBottomOffset.value = estimateTotalRenderSize();
    virtualizer.value.measure();
    debugVirtualScrollState("初始底部定位准备");
    void nextTick(async () => {
      if (requestId !== conversationVirtualizerResetRequest) return;
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (requestId !== conversationVirtualizerResetRequest) return;
      const scrollEl = scrollContainer.value;
      if (scrollEl) {
        scrollEl.scrollTop = scrollEl.scrollHeight;
      }
      const lastIndex = renderItems.value.length - 1;
      if (lastIndex >= 0) {
        virtualizer.value.scrollToIndex(lastIndex, { align: "end" });
      }
      debugVirtualScrollState("初始底部定位完成");
      scrollbarRef.value?.updateThumb();
      traceVirtualScrollFrames("reset:after-nextTick");
    });
  }

  function beginConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    activeJumpToBottomRequest.value = 0;
    clearMeasuredVirtualState();
    initialBottomOffset.value = 0;
    debugVirtualScrollState("切换会话等待首批消息");
    void nextTick(() => {
      if (String(activeConversationId.value || "").trim() !== conversationId) return;
      if (renderItems.value.length <= 0) return;
      debugVirtualScrollState("首批消息已到达");
      resetVirtualizerAtConversationBottom();
    });
  }

  // ==================== scroll helpers ====================

  function scrollConversationToBottomOnce() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    scrollEl.scrollTop = scrollEl.scrollHeight;
    onUserScroll();
    scrollbarRef.value?.updateThumb();
  }

  function scrollToLastItem() {
    const count = renderItems.value.length;
    if (count <= 0) return;
    virtualizer.value.scrollToIndex(count - 1, { align: "end" });
    onUserScroll();
    scrollbarRef.value?.updateThumb();
  }

  function scheduleJumpToBottomStep(requestId: number, remainingFrames: number) {
    if (activeJumpToBottomRequest.value !== requestId) return;
    if (pendingJumpToBottomFrame) {
      cancelAnimationFrame(pendingJumpToBottomFrame);
      pendingJumpToBottomFrame = 0;
    }
    pendingJumpToBottomFrame = requestAnimationFrame(() => {
      pendingJumpToBottomFrame = 0;
      if (activeJumpToBottomRequest.value !== requestId) return;
      scheduleVirtualMeasure();
      scrollConversationToBottomOnce();
      if (remainingFrames > 0) {
        scheduleJumpToBottomStep(requestId, remainingFrames - 1);
        return;
      }
      activeJumpToBottomRequest.value = 0;
    });
  }

  function startJumpToBottomTransaction() {
    activeJumpToBottomRequest.value += 1;
    scheduleVirtualMeasure();
    void nextTick(() => {
      scrollConversationToBottomOnce();
      requestAnimationFrame(() => {
        scrollConversationToBottomOnce();
        activeJumpToBottomRequest.value = 0;
      });
    });
  }

  function syncViewportMetrics() {
    scheduleVirtualMeasure();
    void nextTick(() => scrollbarRef.value?.updateThumb());
  }

  function alignItemToTop(itemId: string, behavior: ScrollBehavior = "smooth") {
    const scrollEl = scrollContainer.value;
    if (!scrollEl || !itemId) return;
    const wrapper = observedVirtualItemElements.get(itemId);
    if (!wrapper || !wrapper.isConnected) return;
    const containerRect = scrollEl.getBoundingClientRect();
    const wrapperRect = wrapper.getBoundingClientRect();
    const scrollStyles = window.getComputedStyle(scrollEl);
    const targetTop = parseFloat(scrollStyles.paddingTop || "0");
    const nextTop = scrollEl.scrollTop + (wrapperRect.top - containerRect.top) - targetTop;
    const resolvedBehavior: ScrollBehavior = behavior === "smooth" && !nativeSmoothScrollEnabled() ? "auto" : behavior;
    scrollEl.scrollTo({ top: Math.max(0, nextTop), behavior: resolvedBehavior });
    onUserScroll();
  }

  function escapeCssString(value: string): string {
    return value.replace(/["\\]/g, "\\$&");
  }

  function findRenderedMessageElement(messageId: string): HTMLElement | null {
    const scrollEl = scrollContainer.value;
    const normalizedId = String(messageId || "").trim();
    if (!scrollEl || !normalizedId) return null;
    const escapedId = typeof CSS !== "undefined" && typeof CSS.escape === "function"
      ? CSS.escape(normalizedId)
      : escapeCssString(normalizedId);
    return scrollEl.querySelector(`[data-message-id="${escapedId}"]`) as HTMLElement | null;
  }

  function resolveMessageAnchorElement(messageElement: HTMLElement | null): HTMLElement | null {
    if (!messageElement) return null;
    return (messageElement.querySelector("[data-message-avatar-anchor='true']") as HTMLElement | null) || messageElement;
  }

  function captureVisibleAnchor(edge: "top" | "bottom"): { messageId: string; edge: "top" | "bottom"; offset: number } | null {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return null;
    const containerRect = scrollEl.getBoundingClientRect();
    let anchor: { messageId: string; offset: number; chronologicalIndex: number } | null = null;
    for (const entry of virtualEntries.value) {
      const itemId = String(entry.item.id || "").trim();
      if (!itemId) continue;
      const wrapper = observedVirtualItemElements.get(itemId);
      if (!wrapper || !wrapper.isConnected) continue;
      const messageElements = Array.from(wrapper.querySelectorAll("[data-message-id]")) as HTMLElement[];
      for (const element of messageElements) {
        const messageId = String(element.getAttribute("data-message-id") || "").trim();
        if (!messageId) continue;
        const anchorElement = resolveMessageAnchorElement(element);
        if (!anchorElement) continue;
        const rect = anchorElement.getBoundingClientRect();
        if (rect.bottom <= containerRect.top + 1 || rect.top >= containerRect.bottom - 1) continue;
        const chronologicalIndex = blockChronologicalIndexMap.value.get(messageId);
        if (chronologicalIndex === undefined) continue;
        const offset = edge === "bottom"
          ? containerRect.bottom - rect.bottom
          : rect.top - containerRect.top;
        if (!anchor) {
          anchor = { messageId, offset, chronologicalIndex };
          continue;
        }
        const shouldReplace = edge === "bottom"
          ? chronologicalIndex > anchor.chronologicalIndex
          : chronologicalIndex < anchor.chronologicalIndex;
        if (shouldReplace) {
          anchor = { messageId, offset, chronologicalIndex };
        }
      }
    }
    return anchor ? { messageId: anchor.messageId, edge, offset: anchor.offset } : null;
  }

  // ==================== lifecycle ====================

  onMounted(() => {
    if (typeof ResizeObserver !== "undefined") {
      virtualItemResizeObserver = new ResizeObserver((entries) => {
        scheduleVirtualResizeMeasure(entries);
      });
      for (const element of observedVirtualItemResizeElements.values()) {
        if (!element.isConnected) continue;
        virtualItemResizeObserver.observe(element);
      }
      syncVisibleStreamingVirtualItemViewportTops();
    }
  });

  watch(
    () => String(activeConversationId.value || "").trim(),
    () => {
      beginConversationBottomInitialization();
    },
    { immediate: true, flush: "post" },
  );

  onBeforeUnmount(() => {
    conversationVirtualizerResetRequest += 1;
    debugTraceRequest += 1;
    virtualItemResizeObserver?.disconnect();
    virtualItemResizeObserver = null;
    if (pendingMeasureFrame) {
      cancelAnimationFrame(pendingMeasureFrame);
      pendingMeasureFrame = 0;
    }
    if (pendingVirtualResizeFrame) {
      cancelAnimationFrame(pendingVirtualResizeFrame);
      pendingVirtualResizeFrame = 0;
    }
    pendingVirtualResizeElements.clear();
    if (pendingJumpToBottomFrame) {
      cancelAnimationFrame(pendingJumpToBottomFrame);
      pendingJumpToBottomFrame = 0;
    }
    observedVirtualItemElements.clear();
    observedVirtualItemResizeElements.clear();
    measuredVirtualItemHeights.clear();
    streamingVirtualItemViewportTop.clear();
  });

  return {
    virtualizer,
    virtualRows,
    virtualEntries,
    totalVirtualSize,
    virtualDebugVisible,
    virtualDebugState,
    measureVirtualRow,
    refreshObservedVirtualItemElements,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    scrollConversationToBottomOnce,
    scrollToLastItem,
    scheduleJumpToBottomStep,
    startJumpToBottomTransaction,
    resetVirtualizerAtConversationBottom,
    alignItemToTop,
    captureVisibleAnchor,
    findRenderedMessageElement,
    resolveMessageAnchorElement,
    updateStreamingVirtualItemViewportTop,
    syncVisibleStreamingVirtualItemViewportTops,
  };
}
