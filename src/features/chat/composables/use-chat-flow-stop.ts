import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMessage } from "../../../types/app";
import { normalizeAssistantStreamBlocks } from "../../../utils/chat-message-semantics";
import { summarizeToolCallsText } from "./use-chat-flow-drafts";
import type { RoundState } from "./use-chat-flow-types";
import { readMessagePlainText } from "./use-chat-flow-utils";

type UseChatFlowStopOptions = {
  chatting: Ref<boolean>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  allMessages: Ref<ChatMessage[]>;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  getConversationId?: () => string;
  invokeStopChatMessage?: (input: {
    session: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };
    partialAssistantText: string;
    partialStreamBlocks: AssistantStreamBlock[];
  }) => Promise<{
    aborted: boolean;
    persisted: boolean;
    conversationId?: string | null;
    assistantText?: string;
    assistantMessage?: ChatMessage;
  }>;
  t: (key: string, params?: Record<string, unknown>) => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  advanceGeneration: () => void;
  setSendChatActiveGen: (gen: number) => void;
  clearDeferredRoundCompletion: () => void;
  clearPendingTerminalEvent: () => void;
  setActiveActivationId: (value: string) => void;
  setActiveRoundAgentId: (value: string) => void;
  clearFrontendDispatchTimer: () => void;
  getPendingUserDraftId: () => string;
  removeMessage: (messageId: string) => void;
  finalizeMessage: (messageId: string, finalMessage?: ChatMessage) => void;
  updateMessageText: (
    messageId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta?: string,
    rawBlocks?: AssistantStreamBlock[],
  ) => void;
  deleteSendStartedAtMs: (gen: number) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  reasoningStartedAtMs: Ref<number>;
};

function stringifyStopError(error: unknown): string {
  return error instanceof Error
    ? `${error.message}\n${error.stack || ""}`.trim()
    : (() => {
        try {
          return JSON.stringify(error);
        } catch {
          return String(error);
        }
      })();
}

export function useChatFlowStop(options: UseChatFlowStopOptions) {
  async function finishLocalStoppedRound(input?: {
    statusState?: "failed" | "";
  }) {
    const statusState = input?.statusState || "";
    options.advanceGeneration();
    options.setSendChatActiveGen(0);
    options.clearDeferredRoundCompletion();
    options.clearPendingTerminalEvent();
    options.setActiveActivationId("");
    options.setActiveRoundAgentId("");
    options.clearFrontendDispatchTimer();

    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeMessage(pendingUserDraftId);
    }

    const round = options.getRound();
    if (round.phase === "streaming") {
      // 停止后原样冻结当前流式画面；后续落盘结果不再回写前台。
      options.updateMessageText(
        round.messageId,
        undefined,
        undefined,
        "",
        normalizeAssistantStreamBlocks(options.streamBlocks?.value || []),
      );
      options.finalizeMessage(round.messageId);
      options.deleteSendStartedAtMs(round.gen);
    } else if (round.phase === "queued") {
      options.finalizeMessage(round.messageId);
      options.deleteSendStartedAtMs(round.gen);
    }

    options.setRound({ phase: "idle" });
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    options.toolStatusState.value = statusState;
    options.toolStatusText.value = statusState
      ? (summarizeToolCallsText(options.streamBlocks?.value || []) || options.t("status.interrupted"))
      : "";
    options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
  }

  async function stopChat() {
    const round = options.getRound();
    if (!options.chatting.value && round.phase !== "queued") return;

    const stopSession = options.getSession();
    const cid = options.getConversationId ? options.getConversationId() : "";
    const activeMessageId = round.phase === "streaming" ? round.messageId : "";
    const activeMessage = activeMessageId
      ? options.allMessages.value.find((message) => String(message?.id || "") === activeMessageId)
      : undefined;
    const partialAssistantText = options.latestAssistantText.value || readMessagePlainText(activeMessage);
    const partialStreamBlocks = normalizeAssistantStreamBlocks(options.streamBlocks?.value || []);

    // 先钉死当前画面，再通知后端打断；后端 partial 只负责落盘，不改前台。
    await finishLocalStoppedRound();

    if (stopSession && options.invokeStopChatMessage) {
      try {
        await options.invokeStopChatMessage({
          session: cid ? { ...stopSession, conversationId: cid } : stopSession,
          partialAssistantText,
          partialStreamBlocks,
        });
      } catch (error) {
        const et = stringifyStopError(error);
        console.warn(`[聊天] 停止消息失败，apiConfigId=${stopSession.apiConfigId}，agentId=${stopSession.agentId}，len=${partialAssistantText.length}，错误=${et}`);
        options.toolStatusState.value = "failed";
        options.toolStatusText.value =
          summarizeToolCallsText(options.streamBlocks?.value || []) || options.t("status.interrupted");
      }
    }
  }

  return {
    stopChat,
  };
}
