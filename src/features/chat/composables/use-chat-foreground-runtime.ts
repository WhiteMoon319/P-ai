import { invokeTauri } from "../../../services/tauri-api";
import type { ChatMessage } from "../../../types/app";
import { mergeAuthoritativeConversationMessages } from "./chat-message-state-machine";
import { createForegroundTailWatermarkCoordinator, createLatestTaskRunner } from "./chat-foreground-coordinator";
import { reconcileForegroundRuntime, type ForegroundRuntimeSnapshot } from "./foreground-recovery-state-machine";
import { useChatForegroundActivity } from "./use-chat-foreground-activity";
import { formalizeMessages } from "./use-chat-flow-utils";

/**
 * 主聊天的唯一前台运行时。APP 与 Web 都从 main-chat 进入这里；
 * IPC/Web bridge 的差异只能由 tauri-api 处理，不能在本流程分支。
 */
export function useChatForegroundRuntime(bindings: Record<string, any>) {
  const foregroundTailWatermark = createForegroundTailWatermarkCoordinator({
    requestChanges: async (since) => {
      const payload = await invokeTauri<{ changed?: Array<{ conversationId?: string }>; serverTime?: string }>("conversation.changedSince", {
        input: { since: since || null },
      });
      return {
        changedConversationIds: (Array.isArray(payload?.changed) ? payload.changed : [])
          .map((item) => String(item?.conversationId || "").trim())
          .filter(Boolean),
        serverTime: String(payload?.serverTime || "").trim(),
      };
    },
  });

  function currentFormalTailMessageId(): string {
    const messages = formalizeMessages(Array.isArray(bindings.allMessages.value) ? bindings.allMessages.value : []);
    return String(messages[messages.length - 1]?.id || "").trim();
  }

  async function requestLatestFormalTailMessageId(conversationId: string): Promise<string> {
    const snapshot = await invokeTauri<{ lastMessageId?: string | null }>("conversation.freshnessSnapshot", {
      input: { conversationId, agentId: null },
    });
    return String(snapshot?.lastMessageId || "").trim();
  }

  async function requestRuntimeSnapshot(conversationId: string): Promise<ForegroundRuntimeSnapshot> {
    return invokeTauri<ForegroundRuntimeSnapshot>("conversation.runtimeSnapshot", { conversationId });
  }

  async function refreshMessageById(conversationId: string, messageId: string): Promise<boolean> {
    const message = await invokeTauri<ChatMessage | null>("conversation.messageById", {
      input: { conversationId, messageId },
    });
    if (!message || String(bindings.currentChatConversationId.value || "").trim() !== conversationId) return false;
    bindings.allMessages.value = mergeAuthoritativeConversationMessages(bindings.allMessages.value, [message], {
      forceReplace: true,
    });
    return true;
  }

  function frontendConversationIsStreaming(): boolean {
    const phase = String(bindings.getChatFlow()?.frontendRoundPhase?.value || "").trim();
    return !!bindings.chatting.value || phase === "queued" || phase === "waiting" || phase === "streaming";
  }

  async function markConversationRead(conversationId: string): Promise<void> {
    if (!conversationId) return;
    await invokeTauri("conversation.markRead", { input: { conversationId } });
  }

  async function reconcileForegroundConversation(reason: string) {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) return;
    try {
      await foregroundTailWatermark.observeCurrentConversation(conversationId);
    } catch (error) {
      console.warn("[聊天前台恢复] 水位查询失败，回退轻量快照", { conversationId, error });
      await bindings.switchUnarchivedConversation(conversationId);
      return;
    }
    if (String(bindings.currentChatConversationId.value || "").trim() !== conversationId) return;
    const snapshot = await requestRuntimeSnapshot(conversationId);
    if (String(bindings.currentChatConversationId.value || "").trim() !== conversationId) return;
    const flow = bindings.getChatFlow();
    const frontendStreamCache = flow?.readConversationStreamCache?.(conversationId);
    const outcome = await reconcileForegroundRuntime({
      conversationId,
      runtimeSnapshot: snapshot,
      frontendStreaming: frontendConversationIsStreaming(),
      frontendMessageId: frontendStreamCache?.persistedAssistantMessageId,
      frontendActivationId: frontendStreamCache?.activationId,
      frontendRequestId: frontendStreamCache?.requestId,
      frontendRevision: frontendStreamCache?.updatedAt,
    }, {
      probeStream: (targetConversationId) => bindings.getChatFlow()?.probeBoundChannel?.(targetConversationId) ?? Promise.resolve(false),
      resumeSubscription: async (targetConversationId) => {
        const currentFlow = bindings.getChatFlow();
        if (!currentFlow?.bindActiveConversationStream) return null;
        await currentFlow.bindActiveConversationStream(targetConversationId, true);
        return requestRuntimeSnapshot(targetConversationId);
      },
      applyRuntimeSnapshot: async (runtimeSnapshot) => {
        if (String(bindings.currentChatConversationId.value || "").trim() !== conversationId) return false;
        const runtimeState = String(runtimeSnapshot.runtimeState || "").trim();
        if (runtimeState === "idle" || runtimeState === "assistant_streaming" || runtimeState === "organizing_context") {
          bindings.applyConversationRuntimeStateUpdated({ conversationId, runtimeState });
        }
        if (runtimeState !== "assistant_streaming" || !String(runtimeSnapshot.streamCache?.persistedAssistantMessageId || "").trim()) {
          return false;
        }
        return (bindings.getChatFlow()?.resumeForegroundRuntimeRound?.({
          conversationId,
          streamCache: runtimeSnapshot.streamCache || null,
          reason: `foreground_${reason}`,
        }) || 0) > 0;
      },
      refreshMessageById,
      finalizeMessage: async () => {
        const currentFlow = bindings.getChatFlow();
        currentFlow?.clearForegroundRuntimeState?.();
        await Promise.resolve(currentFlow?.unbindActiveConversationStream?.()).catch(() => {});
        bindings.applyConversationRuntimeStateUpdated({ conversationId, runtimeState: "idle" });
      },
      applyBackgroundBusy: (runtimeSnapshot) => {
        const runtimeState = String(runtimeSnapshot.runtimeState || "organizing_context").trim();
        if (runtimeState === "organizing_context" || runtimeState === "compacting") {
          bindings.applyConversationRuntimeStateUpdated({ conversationId, runtimeState: runtimeState as "organizing_context" });
        }
      },
      isCurrent: () => String(bindings.currentChatConversationId.value || "").trim() === conversationId,
      currentFormalTailMessageId,
      requestLatestFormalTailMessageId,
      shouldReconcileTail: () => foregroundTailWatermark.shouldReconcileTail(conversationId),
      reloadConversation: () => bindings.switchUnarchivedConversation(conversationId),
    });
    if (String(bindings.currentChatConversationId.value || "").trim() !== conversationId) return;
    if (outcome === "tail_reconciled") foregroundTailWatermark.markTailReconciled(conversationId);
    if (outcome === "handled" || outcome === "tail_reconciled") await markConversationRead(conversationId);
  }

  const foregroundRecoveryRunner = createLatestTaskRunner(async (reason: string) => {
    try {
      await reconcileForegroundConversation(reason);
    } catch (error) {
      console.warn("[聊天前台恢复] 状态机执行失败", { reason, error });
    }
    try {
      await bindings.syncUnarchivedConversationOverviewChangedSinceWatermark(reason);
    } catch (error) {
      console.warn("[聊天前台恢复] 会话概览同步失败", { reason, error });
    }
  });

  const foregroundActivity = useChatForegroundActivity({
    activeSynced: bindings.chatWindowActiveSynced,
    isEnabled: () => bindings.viewMode.value === "chat",
    onWake: (reason) => foregroundRecoveryRunner.run(reason),
    onBackground: bindings.onBackground,
    onWakeError: (reason, error) => {
      console.warn("[聊天前台恢复] 传输恢复失败", { reason, error });
    },
  });

  function handleVisibilityForStateSync() {
    bindings.onVisibilityChange?.();
    foregroundActivity.handleVisibilityChange();
  }

  function cleanupChatForegroundActivity() {
    foregroundRecoveryRunner.cancel();
    foregroundActivity.cleanup();
    bindings.onCleanup?.();
  }

  return {
    recoverForegroundConversation: (reason = "unknown") => foregroundRecoveryRunner.run(reason),
    clearChatWindowActiveSyncTimer: foregroundActivity.clearSyncTimer,
    scheduleChatWindowActiveStateSync: foregroundActivity.schedule,
    syncChatWindowActiveState: foregroundActivity.sync,
    handleWindowFocusForStateSync: foregroundActivity.handleFocus,
    handleWindowBlurForStateSync: foregroundActivity.handleBlur,
    handleVisibilityForStateSync,
    cleanupChatForegroundActivity,
  };
}
