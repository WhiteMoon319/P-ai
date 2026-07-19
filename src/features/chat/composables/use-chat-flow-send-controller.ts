import type { Ref } from "vue";
import type { Channel } from "@tauri-apps/api/core";
import type { AssistantStreamBlock, ChatIngressPart, ChatMentionTarget, ChatMessage } from "../../../types/app";
import type { PreparedChatSendInput } from "./use-chat-flow-send-input";
import type { RoundState, SendChatOverrides } from "./use-chat-flow-types";
import type { AssistantDeltaEvent } from "./use-chat-flow-events";
import { isChatAbortedByUser } from "./use-chat-flow-utils";

type StreamUserImageAttachment = { mime: string; bytesBase64: string; savedPath?: string };

export function buildChatIngressParts(
  text: string,
  images: StreamUserImageAttachment[],
  attachments: Array<{ fileName: string; path: string; mime: string }>,
): ChatIngressPart[] {
  const parts: ChatIngressPart[] = [];
  const normalizedText = String(text || "");
  if (normalizedText) parts.push({ type: "text", text: normalizedText });
  const seenAttachmentPaths = new Set<string>();
  for (const image of images) {
    const path = String(image.savedPath || "").trim().replace(/\\/g, "/");
    const mime = String(image.mime || "").trim();
    if (!mime) continue;
    if (path) {
      seenAttachmentPaths.add(path.toLowerCase());
      parts.push({
        type: "attachment",
        path,
        mime,
        name: path.split("/").pop() || "image",
      });
      continue;
    }
    const bytesBase64 = String(image.bytesBase64 || "").trim();
    if (!bytesBase64) continue;
    parts.push({ type: "attachment", bytesBase64, mime, name: "image" });
  }
  for (const attachment of attachments) {
    const path = String(attachment.path || "").trim().replace(/\\/g, "/");
    if (!path || seenAttachmentPaths.has(path.toLowerCase())) continue;
    seenAttachmentPaths.add(path.toLowerCase());
    parts.push({
      type: "attachment",
      path,
      mime: String(attachment.mime || "").trim(),
      name: String(attachment.fileName || "").trim() || path.split("/").pop() || "attachment",
    });
  }
  return parts;
}

type UseChatFlowSendControllerOptions = {
  chatting: Ref<boolean>;
  submitPending?: Ref<boolean>;
  isConversationBusy?: () => boolean;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  getConversationId?: () => string;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  createSendChatDeltaChannel: (gen: number, conversationId: string) => Channel<AssistantDeltaEvent>;
  invokeSendChatMessage: (input: {
    text: string;
    displayText?: string;
    parts: ChatIngressPart[];
    extraTextBlocks?: string[];
    mentions?: ChatMentionTarget[];
    session: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };
    traceId: string;
    onDelta: Channel<AssistantDeltaEvent>;
  }) => Promise<{
    accepted: boolean;
    duplicate: boolean;
    eventId: string;
    conversationId: string;
    traceId: string;
    ingress: string;
    userMessageId?: string;
    assistantMessageId?: string;
  }>;
  onOwnUserDraftInserted?: (payload: { conversationId: string; messageId: string }) => void;
  onAssistantDraftInserted?: () => void;
  t: (key: string, params?: Record<string, unknown>) => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  setBoundDisplayGeneration?: (gen: number) => void;
  nextGeneration: () => number;
  setSendChatActiveGen: (gen: number) => void;
  setActiveActivationId: (value: string) => void;
  setActiveRoundAgentId: (value: string) => void;
  setPendingTerminalEventNull: () => void;
  sendStartedAtMsByGen: Map<number, number>;
  startFrontendDispatchTimer: (gen: number, startedAtMs?: number, elapsedMs?: number) => void;
  clearFrontendDispatchTimer: () => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearChatErrorText: (conversationId?: string | null) => void;
  applyPreparedSendInput: (input: PreparedChatSendInput) => void;
  prepareSendInput: (overrides?: SendChatOverrides) => PreparedChatSendInput | null;
  insertUserDraft: (
    messageId: string,
    gen: number,
    text: string,
    images: StreamUserImageAttachment[],
    attachments: Array<{ fileName: string; path: string; mime: string }>,
    extraTextBlocks: string[],
    mentions: ChatMentionTarget[],
  ) => string;
  resetDisplayState: () => void;
  removeMessage: (messageId: string) => void;
  updateQueuedAssistantMessageStatus: (messageId: string, statusText: string) => void;
  handleRoundCompleted: (gen: number, result: {
    assistantText: string;
    assistantMessage?: ChatMessage;
  }) => Promise<void>;
  sendRecovery: {
    handleAbortedSend: (gen: number, sendConversationId: string) => void;
    handleFailedSend: (
      gen: number,
      error: unknown,
      sendSession: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string },
      sendConversationId: string,
    ) => Promise<void>;
    finalizeSendChat: (gen: number, suppressInitialReload?: boolean) => Promise<void>;
  };
};

export function useChatFlowSendController(options: UseChatFlowSendControllerOptions) {
  async function sendChat(overrides?: SendChatOverrides) {
    const prepared = options.prepareSendInput(overrides);
    if (!prepared) return;
    const {
      plainText,
      displayText,
      selectedMentions,
      extraTextBlocks,
      sentImages,
      attachments,
      sendSession,
      sendConversationId,
    } = prepared;

    const hasForegroundRoundInFlight =
      options.chatting.value
      || options.getRound().phase !== "idle"
      || !!options.isConversationBusy?.();
    if (!hasForegroundRoundInFlight) {
      options.clearConversationStreamCache(sendConversationId);
      options.setActiveActivationId("");
      options.toolStatusText.value = "";
      options.toolStatusState.value = "";
      if (options.streamBlocks) options.streamBlocks.value = [];
      options.clearChatErrorText(sendConversationId);
    }

    options.applyPreparedSendInput(prepared);

    const gen = options.nextGeneration();
    options.setBoundDisplayGeneration?.(gen);
    options.setSendChatActiveGen(gen);
    options.setActiveRoundAgentId(sendSession.agentId);
    options.sendStartedAtMsByGen.set(gen, Date.now());
    if (!hasForegroundRoundInFlight) {
      options.startFrontendDispatchTimer(gen, options.sendStartedAtMsByGen.get(gen));
    }
    options.setPendingTerminalEventNull();

    const traceId = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    let queuedAssistantMessageId = "";

    if (!hasForegroundRoundInFlight) {
      options.resetDisplayState();
      const round = options.getRound();
      if (round.phase === "streaming") options.removeMessage(round.messageId);
    }

    const shouldBlockStopUntilHistoryFlushed = !hasForegroundRoundInFlight;
    let submitSucceeded = false;

    try {
      const onDelta = options.createSendChatDeltaChannel(gen, sendConversationId);
      if (shouldBlockStopUntilHistoryFlushed && options.submitPending) {
        options.submitPending.value = true;
      }
      const ingressParts = buildChatIngressParts(plainText, sentImages, attachments);
      const submitResult = await options.invokeSendChatMessage({
        text: plainText,
        displayText,
        parts: ingressParts,
        extraTextBlocks: extraTextBlocks.length > 0 ? extraTextBlocks : undefined,
        mentions: selectedMentions.length > 0 ? selectedMentions : undefined,
        session: {
          ...sendSession,
          conversationId: sendConversationId,
        },
        traceId,
        onDelta,
      });
      submitSucceeded = true;
      const ingress = String((submitResult as { ingress?: string } | null)?.ingress || "").trim();
      const accepted = typeof (submitResult as { accepted?: unknown } | null)?.accepted === "boolean"
        ? !!(submitResult as { accepted?: boolean }).accepted
        : true;
      const userMessageId = String((submitResult as { userMessageId?: string } | null)?.userMessageId || "").trim();
      const assistantMessageId = String((submitResult as { assistantMessageId?: string } | null)?.assistantMessageId || "").trim();
      if (import.meta.env.DEV) {
        console.info("[主聊天发送] 收到发送结果", {
          conversationId: sendConversationId,
          userMessageId,
          assistantMessageId,
          accepted,
          ingress,
          textLength: plainText.length,
          sentImageCount: sentImages.length,
          attachmentCount: attachments.length,
          extraTextBlockCount: extraTextBlocks.length,
        });
      }
      if (userMessageId) {
        options.insertUserDraft(userMessageId, gen, plainText, sentImages, attachments, extraTextBlocks, selectedMentions);
      }
      if (!hasForegroundRoundInFlight && accepted && ingress !== "queued") {
        if (selectedMentions.length === 0 && assistantMessageId) {
          queuedAssistantMessageId = assistantMessageId;
          options.setRound({ phase: "queued", gen, messageId: queuedAssistantMessageId });
          options.updateQueuedAssistantMessageStatus(queuedAssistantMessageId, options.t("chat.statusPreparingMessage"));
          options.onAssistantDraftInserted?.();
        }
      }
      if (userMessageId) {
        options.onOwnUserDraftInserted?.({
          conversationId: String(submitResult.conversationId || sendConversationId || "").trim(),
          messageId: userMessageId,
        });
      }
      if (!hasForegroundRoundInFlight && (ingress === "queued" || !accepted)) {
        options.removeMessage(queuedAssistantMessageId);
        if (options.getRound().phase !== "idle") {
          options.setRound({ phase: "idle" });
        }
        options.chatting.value = false;
        options.clearFrontendDispatchTimer();
      }
      if ((!accepted || ingress === "queued") && options.submitPending) {
        options.submitPending.value = false;
      }
    } catch (error) {
      if (options.submitPending) options.submitPending.value = false;
      if (isChatAbortedByUser(error)) {
        options.sendRecovery.handleAbortedSend(gen, sendConversationId);
        return;
      }
      await options.sendRecovery.handleFailedSend(gen, error, sendSession, sendConversationId);
    } finally {
      if (!submitSucceeded && options.submitPending) options.submitPending.value = false;
      // submit_chat_message 是短提交命令；成功后的轮次收束只由 history_flushed、round_started、round_completed 等事件驱动。
    }
  }

  return {
    sendChat,
  };
}
