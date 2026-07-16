import type { ChatMessage } from "../../../types/app";
import { DRAFT_USER_ID_PREFIX, summarizeToolCallsText as formatToolCallsText } from "./use-chat-flow-drafts";
import { messageHasVisibleContent } from "./use-chat-flow-utils";

export function useChatFlowRoundFinalizers(bindings: Record<string, any>) {
  function assistantMessageHasCanonicalContent(message?: ChatMessage): boolean {
    if (!message) return false;
    const providerMeta = { ...((message.providerMeta || {}) as Record<string, unknown>) };
    delete providerMeta._preStreamingStatusText;
    delete providerMeta._toolStatusText;
    delete providerMeta._toolStatusState;
    return messageHasVisibleContent({ ...message, providerMeta });
  }

  async function resolveCanonicalAssistantMessage(
    messageId: string,
    resultMessage?: ChatMessage,
  ): Promise<ChatMessage | undefined> {
    if (assistantMessageHasCanonicalContent(resultMessage)) {
      return resultMessage;
    }
    const conversationId = String(
      bindings.getConversationId ? bindings.getConversationId() : "",
    ).trim();
    if (conversationId && bindings.refreshMessageById) {
      try {
        await bindings.refreshMessageById({ conversationId, messageId });
      } catch (error) {
        console.warn("[聊天] 完成态按消息 ID 回读失败", {
          conversationId,
          messageId,
          message: String((error as { message?: string })?.message ?? error ?? ""),
        });
      }
    }
    const refreshedMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === messageId)
      : undefined;
    return assistantMessageHasCanonicalContent(refreshedMessage) ? refreshedMessage : undefined;
  }

  function finalizeDeferredRoundCompletion() {
    const deferredRoundCompletion = bindings.getDeferredRoundCompletion();
    const round = bindings.getRound();
    if (!deferredRoundCompletion) return;
    if (round.phase !== "streaming" || round.gen !== deferredRoundCompletion.gen) {
      bindings.setDeferredRoundCompletion(null);
      return;
    }
    const { messageId } = round;
    const { result } = deferredRoundCompletion;
    bindings.setDeferredRoundCompletion(null);

    bindings.clearChatErrorText();
    if (String(bindings.toolStatusState.value || "") === "running") {
      bindings.toolStatusState.value = "done";
      bindings.toolStatusText.value = formatToolCallsText(
        bindings.streamBlocks?.value || [],
      ) || bindings.t("status.toolCallDone");
    }

    bindings.finalizeMessage(messageId, result.assistantMessage);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    if (bindings.streamBlocks) bindings.streamBlocks.value = [];
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  async function finalizeQueuedRoundWithoutMessage(
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
    },
  ) {
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    const canonicalAssistantMessage = await resolveCanonicalAssistantMessage(
      round.messageId,
      result.assistantMessage,
    );
    const latestRound = bindings.getRound();
    if (latestRound.phase !== "queued" || latestRound.gen !== gen) return;
    if (canonicalAssistantMessage) {
      bindings.finalizeMessage(round.messageId, canonicalAssistantMessage);
    } else {
      console.warn("[聊天] 完成态缺少可见的正式消息内容，保留当前投影", {
        conversationId: String(bindings.getConversationId ? bindings.getConversationId() : "").trim(),
        messageId: round.messageId,
        gen,
      });
    }
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    bindings.clearChatErrorText();
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  async function failQueuedRoundWithoutMessage(gen: number, error: unknown) {
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    const queuedMessage = bindings.allMessages.value.find((message: ChatMessage) => String(message?.id || "").trim() === round.messageId);
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    bindings.latestAssistantText.value = "";
    if (bindings.streamBlocks) bindings.streamBlocks.value = [];
    bindings.setChatErrorText(bindings.formatRequestFailed(error));
    if (!bindings.toolStatusText.value) {
      bindings.toolStatusState.value = "failed";
      bindings.toolStatusText.value = formatToolCallsText(
        bindings.streamBlocks?.value || [],
      ) || bindings.t("status.toolCallFailed");
    }
    // failed 只清理空气泡；一旦已经有可见内容，就保留当前消息并结束流式态。
    if (messageHasVisibleContent(queuedMessage)) {
      bindings.finalizeMessage(round.messageId);
    } else {
      bindings.removeMessage(round.messageId);
    }
    const pendingUserDraftId = bindings.getPendingUserDraftId();
    if (pendingUserDraftId === `${DRAFT_USER_ID_PREFIX}${gen}`) {
      bindings.removeMessage(pendingUserDraftId);
    }
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  function enqueueStreamDelta(gen: number, delta: string) {
    const round = bindings.getRound();
    if (round.phase !== "streaming" || round.gen !== gen || !delta) return;
    bindings.applyAssistantDeltaToMessage(round.messageId, delta);
    finalizeDeferredRoundCompletion();
  }

  return {
    finalizeDeferredRoundCompletion,
    finalizeQueuedRoundWithoutMessage,
    failQueuedRoundWithoutMessage,
    enqueueStreamDelta,
  };
}
