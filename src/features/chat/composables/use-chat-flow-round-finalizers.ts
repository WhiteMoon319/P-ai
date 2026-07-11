import type { ChatMessage } from "../../../types/app";
import { DRAFT_USER_ID_PREFIX, summarizeToolCallsText as formatToolCallsText } from "./use-chat-flow-drafts";
import { mergeAssistantText } from "./use-chat-flow-text";
import { messageHasVisibleContent } from "./use-chat-flow-utils";

export function useChatFlowRoundFinalizers(bindings: Record<string, any>) {
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

    bindings.latestAssistantText.value = mergeAssistantText(
      bindings.latestAssistantText.value,
      String(result.assistantText || ""),
    );

    bindings.clearChatErrorText();
    if (String(bindings.toolStatusState.value || "") === "running") {
      bindings.toolStatusState.value = "done";
      bindings.toolStatusText.value = formatToolCallsText(
        bindings.streamBlocks?.value || [],
      ) || bindings.t("status.toolCallDone");
    }

    bindings.updateMessageText(messageId);
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
    bindings.latestAssistantText.value = mergeAssistantText(
      bindings.latestAssistantText.value,
      String(result.assistantText || ""),
    );
    bindings.updateMessageText(round.messageId);
    bindings.finalizeMessage(round.messageId, result.assistantMessage);
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
      bindings.updateMessageText(round.messageId);
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
