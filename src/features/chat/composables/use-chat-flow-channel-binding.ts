import { Channel } from "@tauri-apps/api/core";
import {
  readAssistantEvent,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import { normalizeConversationId } from "./use-chat-flow-utils";

export type ChatFlowDeltaSource = "sendChat" | "bound";

type UseChatFlowChannelBindingOptions = {
  debug?: boolean;
  getConversationId?: () => string;
  invokeBindActiveChatViewStream?: (input: {
    conversationId?: string;
    onDelta: Channel<AssistantDeltaEvent>;
  }) => Promise<void>;
  invokeUnbindActiveChatViewStream?: () => Promise<void>;
  invokeProbeActiveChatViewStream?: (input: {
    conversationId?: string;
    probeId: string;
  }) => Promise<boolean>;
  getRoundActiveGen: () => number;
  getCurrentGeneration: () => number;
  markHistoryFlushedReceived: (gen: number) => void;
  handleHistoryFlushed: (
    gen: number,
    parsed: AssistantDeltaEvent,
    source: ChatFlowDeltaSource,
  ) => Promise<void>;
  handleStreamingEvent: (gen: number, parsed: AssistantDeltaEvent) => void;
  formatRequestFailed: (error: unknown) => string;
  setChatErrorText: (text: string) => void;
};

export function useChatFlowChannelBinding(options: UseChatFlowChannelBindingOptions) {
  let boundConversationId = "";
  let boundConversationInitialized = false;
  let boundDisplayGeneration = 0;
  let boundDeltaChannel: Channel<AssistantDeltaEvent> | null = null;
  let boundChannelSeq = 0;
  const pendingProbeResolvers = new Map<string, (received: boolean) => void>();

  function getBoundDisplayGeneration(): number {
    return boundDisplayGeneration;
  }

  function isSameForegroundConversation(conversationId?: string | null): boolean {
    const expectedConversationId = normalizeConversationId(conversationId);
    const currentConversationId = normalizeConversationId(
      options.getConversationId ? options.getConversationId() : "",
    );
    return !!expectedConversationId
      && !!currentConversationId
      && expectedConversationId === currentConversationId;
  }

  function setBoundDisplayGeneration(gen: number) {
    boundDisplayGeneration = Math.max(0, Math.round(Number(gen || 0)));
  }

  function attachDeltaHandler(
    channel: Channel<AssistantDeltaEvent>,
    source: ChatFlowDeltaSource,
    getGen: () => number,
    nextGenOnHistoryFlushed: () => number,
    guard?: () => boolean,
  ) {
    channel.onmessage = (event) => {
      if (guard && !guard()) {
        if (options.debug) {
          if (source === "bound") {
            console.debug("[聊天] 丢弃过期 bound channel 事件", {
              conversationId: boundConversationId,
            });
          } else {
            console.debug("[聊天] 丢弃已切出会话的 sendChat 事件");
          }
        }
        return;
      }
      const parsed = readAssistantEvent(event);
      if (parsed.kind === "stream_probe") {
        const probeId = String(parsed.message || "").trim();
        if (probeId) {
          pendingProbeResolvers.get(probeId)?.(true);
          pendingProbeResolvers.delete(probeId);
        }
        return;
      }

      if (parsed.kind === "history_flushed") {
        const hfGen = nextGenOnHistoryFlushed();
        if (source === "sendChat" && hfGen !== options.getCurrentGeneration()) {
          return;
        }
        if (source === "sendChat") {
          options.markHistoryFlushedReceived(hfGen);
        }
        void options.handleHistoryFlushed(hfGen, parsed, source).catch((err) => {
          console.error("[聊天] history_flushed 处理失败", {
            message: String((err as { message?: string })?.message ?? err ?? ""),
            gen: hfGen,
          });
          options.setChatErrorText(options.formatRequestFailed(err));
        });
        return;
      }

      const currentGen = getGen();
      options.handleStreamingEvent(currentGen, parsed);
    };
  }

  function hasActiveBoundDeltaChannel(conversationId?: string | null): boolean {
    if (!boundDeltaChannel || !boundConversationInitialized) return false;
    const cid = normalizeConversationId(conversationId || (options.getConversationId ? options.getConversationId() : ""));
    const boundCid = normalizeConversationId(boundConversationId);
    return !!cid && !!boundCid && cid === boundCid;
  }

  async function bindActiveConversationStream(conversationId: string, force = false) {
    const id = String(conversationId || "").trim();
    if (!id) {
      await unbindActiveConversationStream();
      return;
    }
    if (!options.invokeBindActiveChatViewStream) return;
    if (!force && boundConversationInitialized && id === boundConversationId) return;
    // 流式绑定日志已移除
    const previousChannel = boundDeltaChannel;
    const previousConversationId = boundConversationId;
    const previousInitialized = boundConversationInitialized;
    const channelSeq = ++boundChannelSeq;
    const channel = new Channel<AssistantDeltaEvent>();
    attachDeltaHandler(
      channel,
      "bound",
      () => options.getRoundActiveGen() || boundDisplayGeneration,
      () => options.getRoundActiveGen() || boundDisplayGeneration,
      () => channelSeq === boundChannelSeq && boundDeltaChannel === channel,
    );
    boundDeltaChannel = channel;
    boundConversationId = id;
    boundConversationInitialized = true;
    try {
      await options.invokeBindActiveChatViewStream({
        conversationId: id || undefined,
        onDelta: channel,
      });
    } catch (error) {
      if (channelSeq === boundChannelSeq && boundDeltaChannel === channel) {
        boundDeltaChannel = previousChannel;
        boundConversationId = previousConversationId;
        boundConversationInitialized = previousInitialized;
      }
      throw error;
    }
    if (!id) boundDisplayGeneration = 0;
    // 流式绑定日志已移除
    if (options.debug) {
      console.debug("[聊天] 已绑定前台流式通道", { conversationId: id });
    }
  }

  async function unbindActiveConversationStream() {
    const hadBinding = !!boundDeltaChannel || boundConversationInitialized || !!boundConversationId;
    boundChannelSeq += 1;
    boundDeltaChannel = null;
    boundConversationId = "";
    boundConversationInitialized = false;
    boundDisplayGeneration = 0;
    for (const resolve of pendingProbeResolvers.values()) {
      resolve(false);
    }
    pendingProbeResolvers.clear();
    if (hadBinding && options.invokeUnbindActiveChatViewStream) {
      await options.invokeUnbindActiveChatViewStream();
    }
    if (options.debug) {
      console.debug("[聊天] 已取消前台流式通道绑定");
    }
  }

  function createSendChatDeltaChannel(gen: number, conversationId: string): Channel<AssistantDeltaEvent> {
    const expectedConversationId = normalizeConversationId(conversationId);
    const channel = new Channel<AssistantDeltaEvent>();
    attachDeltaHandler(
      channel,
      "sendChat",
      () => gen,
      () => gen,
      () => isSameForegroundConversation(expectedConversationId),
    );
    return channel;
  }

  async function probeBoundChannel(conversationId?: string | null, timeoutMs = 800): Promise<boolean> {
    if (!options.invokeProbeActiveChatViewStream) return false;
    const id = String(conversationId || (options.getConversationId ? options.getConversationId() : "")).trim();
    if (!id || !hasActiveBoundDeltaChannel(id)) return false;
    const probeId = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
    const receivedPromise = new Promise<boolean>((resolve) => {
      pendingProbeResolvers.set(probeId, resolve);
      timeoutHandle = setTimeout(() => {
        pendingProbeResolvers.delete(probeId);
        resolve(false);
      }, Math.max(100, timeoutMs));
    });
    try {
      const dispatched = await options.invokeProbeActiveChatViewStream({
        conversationId: id,
        probeId,
      });
      if (!dispatched) {
        pendingProbeResolvers.delete(probeId);
        if (timeoutHandle) clearTimeout(timeoutHandle);
        return false;
      }
      return await receivedPromise;
    } finally {
      pendingProbeResolvers.delete(probeId);
      if (timeoutHandle) clearTimeout(timeoutHandle);
    }
  }

  return {
    attachDeltaHandler,
    bindActiveConversationStream,
    createSendChatDeltaChannel,
    getBoundDisplayGeneration,
    hasActiveBoundDeltaChannel,
    probeBoundChannel,
    setBoundDisplayGeneration,
    unbindActiveConversationStream,
  };
}
