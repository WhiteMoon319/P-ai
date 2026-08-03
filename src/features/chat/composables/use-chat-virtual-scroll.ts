import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import type { ChatRenderItem } from "../utils/chat-render";

interface UseChatVirtualScrollOptions {
  renderItems: Ref<ChatRenderItem[]>;
  scrollContainer: Ref<HTMLElement | null>;
  scrollbarRef: Ref<{ updateThumb: () => void } | null>;
  activeConversationId: Ref<string>;
  latestOwnElasticItemId: Ref<string>;
  latestOwnElasticMinHeight: Ref<number>;
  chatting?: Ref<boolean> | boolean;
  olderHistoryCorrectionAllowed?: Ref<boolean> | boolean;
  debugEnabled?: Ref<boolean> | boolean;
  onUserScroll: () => void;
}

export function useChatVirtualScroll(options: UseChatVirtualScrollOptions) {
  const {
    renderItems,
    scrollContainer,
    scrollbarRef,
    activeConversationId,
    latestOwnElasticItemId,
    latestOwnElasticMinHeight,
    chatting,
    olderHistoryCorrectionAllowed,
    debugEnabled,
    onUserScroll,
  } = options;

  const observedVirtualItemElements = new Map<string, HTMLElement>();
  const observedVirtualItemResizeElements = new Map<string, HTMLElement>();
  const measuredVirtualItemHeights = new Map<string, number>();
  const measuredVirtualItemRevision = ref(0);

  let pendingMeasureFrame = 0;
  let pendingVirtualResizeFrame = 0;
  const pendingVirtualResizeElements = new Set<HTMLElement>();
  let virtualItemResizeObserver: ResizeObserver | null = null;
  let completionLayoutGuardActive = false;
  let completionLayoutGuardFrame = 0;
  let explicitVirtualScrollActive = false;
  let explicitVirtualScrollFrame = 0;

  const initialBottomOffset = ref(0);
  let conversationVirtualizerResetRequest = 0;
  let pendingConversationBottomInitializationId = "";

  // ==================== virtualizer ====================

  const latestOwnTailContentRange = computed(() => {
    measuredVirtualItemRevision.value;
    const itemId = String(latestOwnElasticItemId.value || "").trim();
    if (!itemId) return [];
    const startIndex = renderItems.value.findIndex((item) => item.id === itemId);
    return startIndex < 0 ? [] : renderItems.value.slice(startIndex);
  });

  const latestOwnTailContentHeight = computed(() => {
    return latestOwnTailContentRange.value.reduce(
      (total, item) => total + (measuredVirtualItemHeights.get(item.id) ?? 0),
      0,
    );
  });

  const latestOwnTailContentMeasured = computed(() => {
    const tailItems = latestOwnTailContentRange.value;
    return tailItems.length > 0 && tailItems.every((item) => measuredVirtualItemHeights.has(item.id));
  });

  function chatVirtualScrollDebugEnabled(): boolean {
    if (typeof window === "undefined") return false;
    const configuredDebugEnabled = typeof debugEnabled === "object" && debugEnabled && "value" in debugEnabled
      ? debugEnabled.value
      : debugEnabled;
    if (configuredDebugEnabled === false) return false;
    return window.localStorage.getItem("easy-call.debug.chat-virtual-scroll") === "1"
      || (window as any).__easyCallDebugChatVirtualScroll === true;
  }

  function chatStreamingActive(): boolean {
    const configured = typeof chatting === "object" && chatting && "value" in chatting
      ? chatting.value
      : chatting;
    return !!configured;
  }

  function olderHistoryCorrectionActive(): boolean {
    const configured = typeof olderHistoryCorrectionAllowed === "object" && olderHistoryCorrectionAllowed && "value" in olderHistoryCorrectionAllowed
      ? olderHistoryCorrectionAllowed.value
      : olderHistoryCorrectionAllowed;
    return !!configured;
  }

  function virtualizerScrollToFn(
    offset: number,
    options: { adjustments?: number; behavior?: ScrollBehavior },
    instance: { scrollElement: Element | Window | null },
  ) {
    const scrollEl = instance.scrollElement instanceof HTMLElement ? instance.scrollElement : null;
    if (!scrollEl) return;
    const nextTop = Math.max(0, Math.round(Number(offset || 0)));
    // 覆盖 virtualizer 的尺寸修正滚动：我们定位到流式消息高度持续增长时，
    // @tanstack/virtual 会走 ResizeObserver -> resizeItem -> applyScrollAdjustment
    // -> _scrollToOffset -> scrollToFn，把 scrollTop 连续往下补，维持距底部固定偏移。
    // 这会让聊天窗口在流式期间“自己往下走”。尺寸变化既可能携带 adjustments，
    // 也可能以普通 offset 重定位，因此流式期间只放行我们明确发起的滚动到底。
    if (
      (chatStreamingActive() || completionLayoutGuardActive)
      && !explicitVirtualScrollActive
      && !olderHistoryCorrectionActive()
    ) {
      return;
    }
    scrollEl.scrollTo({
      top: nextTop,
      behavior: options.behavior || "auto",
    });
  }

  function markExplicitVirtualScrollForNextFrame() {
    explicitVirtualScrollActive = true;
    if (explicitVirtualScrollFrame) {
      cancelAnimationFrame(explicitVirtualScrollFrame);
    }
    explicitVirtualScrollFrame = requestAnimationFrame(() => {
      explicitVirtualScrollFrame = 0;
      explicitVirtualScrollActive = false;
    });
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
      + ` total=${Math.round(virtualizer.value.getTotalSize())}`
      + ` elastic=${latestOwnElasticItemId.value ? "yes" : "no"}:${Math.round(latestOwnElasticMinHeight.value)}`
      + ` tail=${Math.round(latestOwnTailContentHeight.value)}`,
    );
  }

  const virtualizer = useVirtualizer(
    computed(() => ({
      count: renderItems.value.length,
      getScrollElement: () => scrollContainer.value,
      getItemKey: (index: number) => renderItems.value[index]?.id ?? `row-${index}`,
      // 不用预估高度参与 absolute 行定位。富文本、工具、图片和代码块没有可靠上界，
      // 低估会让后一行在首帧压住前一行；挂载后统一以真实行元素实测高度定位。
      estimateSize: () => 1,
      initialOffset: () => initialBottomOffset.value,
      scrollToFn: virtualizerScrollToFn,
      anchorTo: "end",
      // 仅在精确贴底时才允许尾部锚定跟随，避免“接近底部”时被持续往下带。
      scrollEndThreshold: 0,
      shouldAdjustScrollPositionOnItemSizeChange: (item: { end: number }, _delta: number, instance: {
        getScrollOffset: () => number;
        getSize: () => number;
      }) => {
        const viewportTop = instance.getScrollOffset();
        const viewportBottom = viewportTop + instance.getSize();
        // 只补偿当前视口上方的尺寸变化；底部新增内容和视口内变化不要自动推着用户往下走。
        return item.end <= viewportTop || viewportBottom <= 0;
      },
      measureElement: (element: Element) => (element as HTMLElement).getBoundingClientRect().height,
      overscan: 600,
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
      measuredTotal: Math.round(virtualizer.value.getTotalSize()),
      totalSize: Math.round(virtualizer.value.getTotalSize()),
      scrollTop: Math.round(scrollEl?.scrollTop ?? 0),
      scrollHeight: Math.round(scrollEl?.scrollHeight ?? 0),
      clientHeight: Math.round(scrollEl?.clientHeight ?? 0),
      range: rows.length > 0 ? `${firstRow?.index}-${lastRow?.index}` : "empty",
      firstItemId: firstItem?.id || "",
      lastItemId: lastItem?.id || "",
      latestOwnElasticItemId: latestOwnElasticItemId.value,
      latestOwnElasticMinHeight: Math.round(latestOwnElasticMinHeight.value),
      latestOwnTailContentHeight: Math.round(latestOwnTailContentHeight.value),
    };
  });

  watch(
    () => chatStreamingActive(),
    (isStreaming, wasStreaming) => {
      if (!wasStreaming || isStreaming) return;
      const scrollEl = scrollContainer.value;
      if (!scrollEl || typeof window === "undefined") return;
      const preservedScrollTop = scrollEl.scrollTop;
      completionLayoutGuardActive = true;
      if (completionLayoutGuardFrame) {
        window.cancelAnimationFrame(completionLayoutGuardFrame);
      }
      completionLayoutGuardFrame = window.requestAnimationFrame(() => {
        completionLayoutGuardFrame = window.requestAnimationFrame(() => {
          completionLayoutGuardFrame = 0;
          scrollEl.scrollTop = preservedScrollTop;
          completionLayoutGuardActive = false;
          scrollbarRef.value?.updateThumb();
        });
      });
    },
  );

  // ==================== helpers ====================

  // ==================== resize handling ====================

  function handleVirtualItemResize(element: HTMLElement) {
    const itemId = String(element.getAttribute("data-render-item-id") || "").trim();
    if (!itemId) return;
    const nextHeight = Math.round(element.getBoundingClientRect().height);
    const previousHeight = measuredVirtualItemHeights.get(itemId);
    if (previousHeight === nextHeight) {
      observedVirtualItemElements.set(itemId, element);
      return;
    }
    // 先更新缓存再测量：measureElement 的缓存分支会读取 measuredVirtualItemHeights，
    // 若仍为旧值会返回旧高度导致 virtualizer 不更新布局。
    measuredVirtualItemHeights.set(itemId, nextHeight);
    measuredVirtualItemRevision.value += 1;
    virtualizer.value.measureElement(element);
    observedVirtualItemElements.set(itemId, element);
  }

  function scheduleVirtualMeasure() {
    if (pendingMeasureFrame) return;
    void nextTick(() => {
      if (pendingMeasureFrame) return;
      pendingMeasureFrame = requestAnimationFrame(() => {
        pendingMeasureFrame = 0;
        refreshObservedVirtualItemElements();
        virtualizer.value.measure();
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
        if (measuredVirtualItemHeights.delete(normalizedItemId)) {
          measuredVirtualItemRevision.value += 1;
        }
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
        if (measuredVirtualItemHeights.delete(normalizedItemId)) {
          measuredVirtualItemRevision.value += 1;
        }
      }
      return;
    }
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
      measuredVirtualItemRevision.value += 1;
      virtualizer.value.measureElement(target);
      observedVirtualItemElements.set(resolvedItemId, target);
    }
  }

  function refreshObservedVirtualItemElements() {
    const validIds = new Set<string>();
    for (const entry of virtualEntries.value) {
      const itemId = String(entry.item.id || "").trim();
      if (!itemId) continue;
      validIds.add(itemId);
      if (entry.item.kind === "message") {
        const blockId = String(entry.item.block.id || "").trim();
        if (blockId) validIds.add(blockId);
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
        if (measuredVirtualItemHeights.delete(itemId)) {
          measuredVirtualItemRevision.value += 1;
        }
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
    measuredVirtualItemRevision.value += 1;
    pendingVirtualResizeElements.clear();
  }

  function scrollVirtualizerToConversationBottomLightweight(behavior: "auto" | "smooth" = "auto") {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    void nextTick(async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const targetTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
      scrollEl.scrollTo({ top: targetTop, behavior });
      scrollbarRef.value?.updateThumb();
    });
  }

  function resetVirtualizerAtConversationBottom(behavior: "auto" | "smooth" = "auto") {
    const requestId = ++conversationVirtualizerResetRequest;
    clearMeasuredVirtualState();
    initialBottomOffset.value = 0;
    virtualizer.value.measure();
    void nextTick(async () => {
      if (requestId !== conversationVirtualizerResetRequest) return;
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (requestId !== conversationVirtualizerResetRequest) return;
      if (renderItems.value.length > 0) {
        markExplicitVirtualScrollForNextFrame();
        virtualizer.value.scrollToEnd({ behavior });
      }
      // smooth 滚动依赖浏览器原生动画，强制赋值 scrollTop 会打断它；仅在 auto 时兜底钳制到真实底端
      if (behavior !== "smooth") {
        const scrollEl = scrollContainer.value;
        if (scrollEl) {
          scrollEl.scrollTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
        }
      }
      scrollbarRef.value?.updateThumb();
    });
  }

  function beginConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    pendingConversationBottomInitializationId = conversationId;
    clearMeasuredVirtualState();
    initialBottomOffset.value = 0;
  }

  function renderListReadyKey() {
    const items = renderItems.value;
    const firstId = String(items[0]?.id || "").trim();
    const lastId = String(items[items.length - 1]?.id || "").trim();
    return `${items.length}:${firstId}:${lastId}`;
  }

  function resolvePendingConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!pendingConversationBottomInitializationId || pendingConversationBottomInitializationId !== conversationId) return;
    if (renderItems.value.length <= 0) return;
    pendingConversationBottomInitializationId = "";
    resetVirtualizerAtConversationBottom();
  }

  function syncViewportMetrics() {
    scheduleVirtualMeasure();
    void nextTick(() => scrollbarRef.value?.updateThumb());
  }

  function scrollVirtualizerToIndex(
    index: number,
    options?: { align?: "auto" | "start" | "center" | "end"; behavior?: ScrollBehavior },
  ) {
    markExplicitVirtualScrollForNextFrame();
    virtualizer.value.scrollToIndex(index, options);
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
    }
  });

  watch(
    () => String(activeConversationId.value || "").trim(),
    () => {
      beginConversationBottomInitialization();
    },
    { immediate: true, flush: "post" },
  );

  watch(
    renderListReadyKey,
    () => {
      void nextTick(() => resolvePendingConversationBottomInitialization());
    },
    { immediate: true, flush: "post" },
  );

  onBeforeUnmount(() => {
    if (completionLayoutGuardFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(completionLayoutGuardFrame);
      completionLayoutGuardFrame = 0;
    }
    completionLayoutGuardActive = false;
    if (explicitVirtualScrollFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(explicitVirtualScrollFrame);
      explicitVirtualScrollFrame = 0;
    }
    explicitVirtualScrollActive = false;
    conversationVirtualizerResetRequest += 1;
    pendingConversationBottomInitializationId = "";
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
    observedVirtualItemElements.clear();
    observedVirtualItemResizeElements.clear();
    measuredVirtualItemHeights.clear();
  });

  return {
    virtualizer,
    virtualRows,
    virtualEntries,
    totalVirtualSize,
    latestOwnTailContentHeight,
    latestOwnTailContentMeasured,
    virtualDebugVisible,
    virtualDebugState,
    measureVirtualRow,
    refreshObservedVirtualItemElements,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    scrollVirtualizerToIndex,
    scrollVirtualizerToConversationBottomLightweight,
    resetVirtualizerAtConversationBottom,
  };
}
