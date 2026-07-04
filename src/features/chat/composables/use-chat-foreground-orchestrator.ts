import { nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { i18n } from "../../../i18n";
import { invokeTauri } from "../../../services/tauri-api";
import { readLastActiveConversationId } from "../utils/last-active-conversation";

const t = i18n.global.t;

export function useChatForegroundOrchestrator(bindings: Record<string, any>) {
  async function requestConversationLightSnapshot(
    conversationId?: string | null,
    options?: { resumeProjection?: boolean },
  ) {
    const targetConversationId = String(conversationId || "").trim();
    return invokeTauri<any>("get_foreground_conversation_light_snapshot", {
      input: {
        conversationId: targetConversationId || null,
        agentId: targetConversationId
          ? null
          : String(bindings.currentForegroundAgentId.value || "").trim() || null,
        limit: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
        resumeProjection: !!options?.resumeProjection,
      },
    });
  }

  async function requestUnarchivedConversationOverview() {
    return invokeTauri<any[]>("list_unarchived_conversations");
  }

  async function refreshRemoteImConversationOverview() {
    bindings.remoteImContactConversations.value = await invokeTauri<any[]>("remote_im_list_contact_conversations");
  }

  async function refreshUnarchivedConversationOverview() {
    const items = await requestUnarchivedConversationOverview();
    bindings.unarchivedConversations.value = Array.isArray(items) ? items : [];
  }

  function pickForegroundConversationId(candidates: any[]): string {
    const storedConversationId = readLastActiveConversationId();
    if (storedConversationId) {
      const stored = candidates.find((item) => String(item?.conversationId || "").trim() === storedConversationId);
      if (stored) return storedConversationId;
    }
    const target =
      candidates.find((item) => !!item.isSystemNotificationConversation)
      || candidates.find((item) => !!item.isActive)
      || candidates[0];
    return String(target?.conversationId || "").trim();
  }

  function clearForegroundConversation(reason: string) {
    const previousConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!previousConversationId) return;
    bindings.cacheConversationMessages(previousConversationId, bindings.allMessages.value);
    bindings.currentChatConversationId.value = "";
    bindings.currentChatPreferredApiConfigId.value = "";
    bindings.currentChatTodos.value = [];
    if (bindings.trimmingConversationId.value === previousConversationId) {
      bindings.trimmingConversationId.value = "";
      bindings.trimming.value = false;
    }
    if (bindings.compactingConversationId.value === previousConversationId) {
      bindings.compactingConversationId.value = "";
      bindings.compactingConversation.value = false;
    }
    bindings.allMessages.value = [];
    bindings.hasMoreBackendHistory.value = false;
    bindings.foregroundTailLatestReady.value = true;
    bindings.clearPendingManualScrollToBottom();
    bindings.getChatFlow().clearForegroundRuntimeState();
    void reason;
  }

  async function recoverForegroundConversationFromOverview(reason: string, preferredConversationId?: string | null) {
    if (bindings.conversationForegroundSyncing.value) return;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (currentConversationId) return;
    const nextConversationId =
      String(preferredConversationId || "").trim()
      || pickForegroundConversationId(bindings.unarchivedConversations.value);
    if (!nextConversationId) {
      clearForegroundConversation(reason);
      return;
    }
    await switchUnarchivedConversation(nextConversationId);
  }

  function syncCurrentConversationWorkspaceLabel() {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return;
    const nextLabel = String(bindings.chatWorkspaceName.value || "").trim() || t('chat.foregroundOrchestrator.defaultWorkspace');
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== currentConversationId) {
        return item;
      }
      if (String(item.workspaceLabel || "").trim() === nextLabel) {
        return item;
      }
      changed = true;
      return {
        ...item,
        workspaceLabel: nextLabel,
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = nextItems;
    }
  }

  async function switchUnarchivedConversation(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const previousConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const startedAt = bindings.perfNow();
    try {
      bindings.conversationForegroundSyncing.value = true;
      if (previousConversationId) {
        bindings.cacheConversationMessages(previousConversationId, bindings.allMessages.value);
        bindings.clearConversationBadge(previousConversationId);
        bindings.markConversationReadPersisted(previousConversationId);
      }
      bindings.getChatFlow().clearForegroundRuntimeState();
      bindings.clearPendingManualScrollToBottom();
      bindings.foregroundTailLatestReady.value = false;
      const trace = bindings.beginForegroundPaintTrace(cid);
      // 切会话恢复时，先让后端把“持久消息 + 当前运行中投影”合成为一份权威快照，
      // 前端一次性接管正文，再只负责接后续流式增量，避免先显示持久消息再二次补流式。
      const snapshot = await requestConversationLightSnapshot(cid, {
        resumeProjection: true,
      });
      bindings.applyConversationSnapshot(snapshot);
      if (String(bindings.currentChatConversationId.value || "").trim() === cid && snapshot?.shouldBindStream) {
        await bindings.getChatFlow()?.bindActiveConversationStream?.(cid, true);
        if (String(bindings.currentChatConversationId.value || "").trim() === cid) {
          bindings.getChatFlow()?.resumeForegroundRuntimeRound?.({
            conversationId: cid,
            streamCache: snapshot?.streamCache || null,
            statusText: t('chat.statusWaitingReply'),
            reason: "switch_conversation_snapshot_ready",
          });
        }
      }
      bindings.clearConversationBadge(cid);
      bindings.markConversationReadPersisted(cid);
      await nextTick();
      bindings.triggerConversationScrollToBottom(cid, "switch_snapshot_ready");
      bindings.logForegroundPaintTrace(trace, "前台轻量快照已接管最新消息", {
        conversationId: cid,
        snapshotCount: Array.isArray(snapshot?.messages) ? snapshot.messages.length : 0,
        hasMoreHistory: !!snapshot?.hasMoreHistory,
        shouldBindStream: !!snapshot?.shouldBindStream,
        fromConversationId: previousConversationId,
        syncCostMs: Math.round((bindings.perfNow() - startedAt) * 10) / 10,
      });
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
  }

  async function ensureLatestForegroundTailThenScrollToBottom() {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) return;
    if (bindings.foregroundTailLatestReady.value) {
      bindings.triggerConversationScrollToBottom(conversationId, "manual_ready");
      return;
    }
    try {
      const result = await invokeTauri<{ accepted: boolean; requestId: string }>("request_conversation_messages_after_async", {
        input: {
          conversationId,
          afterMessageId: bindings.buildConversationMessagesAfterAnchor(conversationId),
          fallbackLimit: bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT,
        },
      });
      if (!result?.accepted) {
        bindings.triggerConversationScrollToBottom(conversationId, "manual_request_rejected");
        return;
      }
      bindings.setPendingManualScrollState(conversationId, String(result.requestId || "").trim());
      if (!String(result.requestId || "").trim()) {
        bindings.triggerConversationScrollToBottom(conversationId, "manual_request_missing_id");
      }
    } catch (error) {
      console.warn("[会话切换] 手动滚到底前请求尾部增量失败", {
        conversationId,
        error,
      });
      bindings.triggerConversationScrollToBottom(conversationId, "manual_request_failed");
    }
  }

  async function refreshChatUnarchivedConversations() {
    if (bindings.conversationForegroundSyncing.value) return;
    if (bindings.detachedChatWindow.value) {
      await refreshRemoteImConversationOverview();
      return;
    }
    try {
      bindings.conversationForegroundSyncing.value = true;
      await refreshUnarchivedConversationOverview();
      await refreshRemoteImConversationOverview();
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
    if (!String(bindings.currentChatConversationId.value || "").trim()) {
      await recoverForegroundConversationFromOverview("refresh_unarchived_conversations");
    }
  }

  async function initializeDetachedChatWindow() {
    if (!bindings.detachedChatWindow.value) return;
    try {
      const info = await invokeTauri<any>("get_detached_chat_window_info");
      const conversationId = String(info?.conversationId || "").trim();
      if (!info?.detached || !conversationId) {
        bindings.setStatus(t('chat.foregroundOrchestrator.missingBinding'));
        try {
          await getCurrentWindow().close();
        } catch (closeError) {
          console.error("[独立聊天窗口] 缺少绑定会话时关闭窗口失败", closeError);
          bindings.setStatusError("status.requestFailed", closeError);
        }
        return;
      }
      bindings.detachedChatConversationId.value = conversationId;
      bindings.currentChatConversationId.value = conversationId;
      bindings.sideConversationListVisible.value = false;
      await refreshRemoteImConversationOverview();
      await nextTick();
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    }
  }

  async function handleCloseWindow() {
    if (bindings.detachedChatWindow.value) {
      await getCurrentWindow().close();
      return;
    }
    bindings.freezeForegroundConversation("close_window");
    await bindings.closeWindow();
  }

  async function detachCurrentConversationToWindow() {
    console.info("[独立聊天窗口][前端链路] ChatWindowApp 进入 detachCurrentConversationToWindow", {
      windowLabel: bindings.tauriWindowLabel.value,
      detachedChatWindow: bindings.detachedChatWindow.value,
      currentConversationId: String(bindings.currentChatConversationId.value || "").trim(),
      chatting: bindings.chatting.value,
      trimming: bindings.trimming.value,
      compactingConversation: bindings.compactingConversation.value,
      isSystemNotificationConversation: !!bindings.currentForegroundConversationSummary.value?.isSystemNotificationConversation,
    });
    bindings.setStatus(t('chat.foregroundOrchestrator.openingDetached'));
    if (bindings.detachedChatWindow.value) {
      console.warn("[独立聊天窗口][前端链路] 当前已经是独立窗口，忽略独立窗口请求");
      return;
    }
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId || bindings.chatting.value || bindings.trimming.value || bindings.compactingConversation.value) {
      console.warn("[独立聊天窗口][前端链路] 当前状态不允许独立窗口", {
        conversationId,
        chatting: bindings.chatting.value,
        trimming: bindings.trimming.value,
        compactingConversation: bindings.compactingConversation.value,
      });
      return;
    }
    if (bindings.currentForegroundConversationSummary.value?.isSystemNotificationConversation) {
      console.warn("[独立聊天窗口][前端链路] 系统通知会话不允许独立窗口", { conversationId });
      bindings.setStatus(t('chat.foregroundOrchestrator.mainConversationNotAllowed'));
      return;
    }
    try {
      console.info("[独立聊天窗口][前端链路] 准备 invoke detach_current_conversation_to_window", {
        conversationId,
      });
      void invokeTauri<{ conversationId: string; windowLabel: string; systemNotificationConversationId?: string | null }>("detach_current_conversation_to_window", {
        input: { conversationId },
      }).then((result) => {
        console.info("[独立聊天窗口][前端链路] invoke detach_current_conversation_to_window 已返回", result);
        void refreshUnarchivedConversationOverview();
      }).catch((error) => {
        console.error("[独立聊天窗口][前端链路] 打开独立窗口失败", error);
        bindings.setStatusError("status.loadMessagesFailed", error);
        void refreshUnarchivedConversationOverview();
      });
      clearForegroundConversation("detach_current_conversation");
      const systemNotificationConversationId = String(bindings.unarchivedConversations.value.find((item: any) => !!item.isSystemNotificationConversation)?.conversationId || "").trim();
      if (systemNotificationConversationId) {
        await switchUnarchivedConversation(systemNotificationConversationId);
      } else {
        await refreshUnarchivedConversationOverview();
      }
      bindings.setStatus(t('chat.foregroundOrchestrator.detachedRequestSent'));
    } catch (error) {
      console.error("[独立聊天窗口][前端链路] 打开独立窗口失败", error);
      bindings.setStatusError("status.loadMessagesFailed", error);
    }
  }

  async function sendChatFromCurrentWindow(overrides?: { extraTextBlocks?: string[] }) {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (bindings.waitPendingConversationPreferredModelPersist) {
      const modelPersisted = await bindings.waitPendingConversationPreferredModelPersist(conversationId);
      if (!modelPersisted) return;
    }
    await bindings.getChatFlow().sendChat(overrides);
  }

  function freezeForegroundConversation(reason: string) {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (currentConversationId) {
      bindings.cacheConversationMessages(currentConversationId, bindings.allMessages.value);
    }
    bindings.getChatFlow().freezeForegroundRoundState();
    void reason;
  }

  function hasActiveForegroundConversation(conversationId?: string | null): boolean {
    if (!bindings.isChatWindowActiveNow()) return false;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return false;
    const targetConversationId = String(conversationId || "").trim();
    return !targetConversationId || targetConversationId === currentConversationId;
  }

  return {
    requestConversationLightSnapshot,
    requestUnarchivedConversationOverview,
    refreshRemoteImConversationOverview,
    refreshUnarchivedConversationOverview,
    pickForegroundConversationId,
    clearForegroundConversation,
    recoverForegroundConversationFromOverview,
    syncCurrentConversationWorkspaceLabel,
    switchUnarchivedConversation,
    ensureLatestForegroundTailThenScrollToBottom,
    refreshChatUnarchivedConversations,
    initializeDetachedChatWindow,
    handleCloseWindow,
    detachCurrentConversationToWindow,
    sendChatFromCurrentWindow,
    freezeForegroundConversation,
    hasActiveForegroundConversation,
  };
}
