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
  onReloadMessages: () => Promise<void>;
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
  removeDraft: (draftId: string) => void;
  finalizeDraft: (draftId: string, finalMessage?: ChatMessage) => void;
  updateDraftText: (
    draftId: string,
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
    preserveAssistantDraft?: boolean;
  }) {
    const statusState = input?.statusState || "";
    const preserveAssistantDraft = !!input?.preserveAssistantDraft;
    options.advanceGeneration();
    options.setSendChatActiveGen(0);
    options.clearDeferredRoundCompletion();
    options.clearPendingTerminalEvent();
    options.setActiveActivationId("");
    options.setActiveRoundAgentId("");
    options.clearFrontendDispatchTimer();

    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeDraft(pendingUserDraftId);
    }

    const round = options.getRound();
    if (round.phase === "streaming") {
      if (preserveAssistantDraft) {
        // stop = 保留现状：直接冻结当前前端可见内容，不额外向后端重取。
        options.updateDraftText(round.draftId, undefined, undefined, "", normalizeAssistantStreamBlocks(options.streamBlocks?.value || []));
        options.finalizeDraft(round.draftId);
      } else {
        options.removeDraft(round.draftId);
      }
      options.deleteSendStartedAtMs(round.gen);
    } else if (round.phase === "queued") {
      // stop 同样保留现状；即便还是空气泡，也保留当前消息本身，只结束流式态。
      options.finalizeDraft(round.draftId);
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
    const activeDraftId = round.phase === "streaming" ? round.draftId : "";
    const activeDraft = activeDraftId
      ? options.allMessages.value.find((message) => String(message?.id || "") === activeDraftId)
      : undefined;
    const partialAssistantText = options.latestAssistantText.value || readMessagePlainText(activeDraft);
    const partialStreamBlocks = normalizeAssistantStreamBlocks(options.streamBlocks?.value || []);
    const localStopSucceeded = async () => finishLocalStoppedRound({
      preserveAssistantDraft: round.phase === "streaming",
    });
    if (round.phase === "queued") {
      if (stopSession && options.invokeStopChatMessage) {
        try {
          await options
          .invokeStopChatMessage({
            session: cid ? { ...stopSession, conversationId: cid } : stopSession,
            partialAssistantText,
            partialStreamBlocks,
          });
          await localStopSucceeded();
          return;
        } catch (error) {
          const et = stringifyStopError(error);
          console.warn(`[聊天] queued 停止后端中断失败，apiConfigId=${stopSession.apiConfigId}，agentId=${stopSession.agentId}，错误=${et}`);
        }
      } else {
        await localStopSucceeded();
        return;
      }
    }

    if (stopSession && options.invokeStopChatMessage) {
      try {
        await options.invokeStopChatMessage({
          session: cid ? { ...stopSession, conversationId: cid } : stopSession,
          partialAssistantText,
          partialStreamBlocks,
        });
        await localStopSucceeded();
        return;
      } catch (error) {
        const et = stringifyStopError(error);
        console.warn(`[聊天] 停止消息失败，apiConfigId=${stopSession.apiConfigId}，agentId=${stopSession.agentId}，len=${partialAssistantText.length}，错误=${et}`);
      }
    }

    await finishLocalStoppedRound({ statusState: "failed" });
  }

  return {
    stopChat,
  };
}
