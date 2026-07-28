import { ref, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig, ChatMessage } from "../../../types/app";
import { mergeAuthoritativeConversationMessages } from "./chat-message-state-machine";
import { formalizeMessages } from "./use-chat-flow-utils";
import { createLatestTaskRunner } from "./chat-foreground-coordinator";
import {
  recoverForegroundStreaming,
  type ForegroundRuntimeSnapshot,
} from "./foreground-recovery-state-machine";
import { useChatForegroundActivity } from "./use-chat-foreground-activity";
import { useRecordHotkey } from "./use-record-hotkey";

type RecordingActivationSource = "foreground" | "background";

type UseChatWindowRecordingOrchestratorOptions = {
  viewMode: Ref<"chat" | "archives" | "config">;
  config: AppConfig;
  recording: Ref<boolean>;
  currentChatConversationId: Ref<string>;
  chatting: Ref<boolean>;
  recordHotkeyProbeLastSeq: Ref<number>;
  recordHotkeyProbeDown: Ref<boolean>;
  chatWindowActiveSynced: Ref<boolean | null>;
  allMessages: Ref<ChatMessage[]>;
  getChatFlow: () => {
    probeBoundChannel?: (conversationId?: string | null, timeoutMs?: number) => Promise<boolean>;
    bindActiveConversationStream?: (conversationId: string, force?: boolean) => Promise<void>;
    unbindActiveConversationStream?: () => Promise<void>;
    clearForegroundRuntimeState?: () => void;
    resumeForegroundRuntimeRound?: (input?: {
      conversationId?: string | null;
      streamCache?: unknown;
      statusText?: string;
      reason?: string;
    }) => number;
    frontendRoundPhase?: Ref<"idle" | "queued" | "waiting" | "streaming">;
    readConversationStreamCache?: (conversationId?: string | null) => {
      activationId?: string;
      requestId?: string;
      updatedAt?: string;
      persistedAssistantMessageId?: string;
    } | null;
  } | null | undefined;
  applyConversationRuntimeStateUpdated: (payload: {
    conversationId: string;
    runtimeState: "idle" | "assistant_streaming" | "organizing_context";
  }) => void;
  startSpeechRecording: () => Promise<unknown>;
  stopSpeechRecording: (discard: boolean) => Promise<unknown>;
  prewarmMicrophone: () => Promise<unknown>;
  syncUnarchivedConversationOverviewChangedSinceWatermark: (reason?: string) => Promise<void>;
  switchUnarchivedConversation: (conversationId: string) => Promise<void>;
};

export function useChatWindowRecordingOrchestrator(options: UseChatWindowRecordingOrchestratorOptions) {
  const CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS = 260;
  const foregroundRecordingActive = ref(false);
  let chatMicPrewarmTimer: ReturnType<typeof setTimeout> | null = null;

  function clearChatMicPrewarmTimer() {
    if (!chatMicPrewarmTimer) return;
    clearTimeout(chatMicPrewarmTimer);
    chatMicPrewarmTimer = null;
  }

  async function tryPrewarmChatMic(reason: string) {
    if (options.viewMode.value !== "chat") return;
    if (document.visibilityState === "hidden") return;
    if (!document.hasFocus()) return;
    void reason;
    await options.prewarmMicrophone();
  }

  function isChatWindowActiveNow(): boolean {
    return options.viewMode.value === "chat" && document.visibilityState === "visible" && document.hasFocus();
  }

  async function startRecording(source: RecordingActivationSource = "foreground") {
    if (!options.recording.value) {
      foregroundRecordingActive.value = source === "foreground" && isChatWindowActiveNow();
    }
    await options.startSpeechRecording();
    if (!options.recording.value) {
      foregroundRecordingActive.value = false;
    }
  }

  async function stopRecording(discard: boolean) {
    foregroundRecordingActive.value = false;
    await options.stopSpeechRecording(discard);
  }

  const recordHotkey = useRecordHotkey({
    isActive: () => isChatWindowActiveNow(),
    getRecordHotkey: () => options.config.recordHotkey,
    onStartRecording: (source) => startRecording(source),
    onStopRecording: (discard) => stopRecording(discard),
    startDelayMs: 0,
  });

  function cancelForegroundRecordingOnBackground(reason: string) {
    void reason;
    if (!foregroundRecordingActive.value) return;
    foregroundRecordingActive.value = false;
    recordHotkey.resetPressedState();
    void options.stopSpeechRecording(true);
  }

  watch(options.recording, (active) => {
    if (!active) {
      foregroundRecordingActive.value = false;
    }
  });

  function clearRecordHotkeyProbeState() {
    options.recordHotkeyProbeDown.value = false;
    options.recordHotkeyProbeLastSeq.value = 0;
  }

  function scheduleChatMicPrewarm(reason: string, delayMs = 0) {
    clearChatMicPrewarmTimer();
    if (delayMs <= 0) {
      void tryPrewarmChatMic(reason);
      return;
    }
    chatMicPrewarmTimer = setTimeout(() => {
      chatMicPrewarmTimer = null;
      void tryPrewarmChatMic(reason);
    }, delayMs);
  }

  function currentFormalTailMessageId(): string {
    const formalMessages = formalizeMessages(Array.isArray(options.allMessages.value) ? options.allMessages.value : []);
    return String(formalMessages[formalMessages.length - 1]?.id || "").trim();
  }

  async function requestLatestFormalTailMessageId(conversationId: string): Promise<string> {
    const snapshot = await invokeTauri<any>("conversation.freshnessSnapshot", {
      input: {
        conversationId,
        agentId: null,
      },
    });
    return String(snapshot?.lastMessageId || "").trim();
  }

  async function markConversationReadOnForegroundFocus(conversationId: string): Promise<void> {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    await invokeTauri("conversation.markRead", {
      input: { conversationId: normalizedConversationId },
    });
  }

  async function requestConversationRuntimeSnapshot(conversationId: string): Promise<ForegroundRuntimeSnapshot> {
    return invokeTauri<ForegroundRuntimeSnapshot>("conversation.runtimeSnapshot", {
      conversationId,
    });
  }

  async function refreshForegroundTargetMessage(conversationId: string, messageId: string): Promise<boolean> {
    const message = await invokeTauri<ChatMessage | null>("conversation.messageById", {
      input: { conversationId, messageId },
    });
    if (!message || String(options.currentChatConversationId.value || "").trim() !== conversationId) return false;
    const index = options.allMessages.value.findIndex((item) => String(item.id || "").trim() === messageId);
    if (index < 0) return false;
    options.allMessages.value = mergeAuthoritativeConversationMessages(
      options.allMessages.value,
      [message],
      { forceReplace: true },
    );
    return true;
  }

  function frontendConversationIsStreaming(): boolean {
    const chatFlow = options.getChatFlow();
    const phase = String(chatFlow?.frontendRoundPhase?.value || "").trim();
    return !!options.chatting.value || phase === "queued" || phase === "waiting" || phase === "streaming";
  }

  async function reconcileForegroundConversationAfterWake(reason: string) {
    const conversationId = String(options.currentChatConversationId.value || "").trim();
    if (!conversationId) return;
    const snapshot = await requestConversationRuntimeSnapshot(conversationId);
    if (String(options.currentChatConversationId.value || "").trim() !== conversationId) return;
    const chatFlow = options.getChatFlow();
    const frontendStreamCache = chatFlow?.readConversationStreamCache?.(conversationId);
    const outcome = await recoverForegroundStreaming({
      conversationId,
      runtimeSnapshot: snapshot,
      frontendStreaming: frontendConversationIsStreaming(),
      frontendMessageId: frontendStreamCache?.persistedAssistantMessageId,
      frontendActivationId: frontendStreamCache?.activationId,
      frontendRequestId: frontendStreamCache?.requestId,
      frontendRevision: frontendStreamCache?.updatedAt,
    }, {
      probeStream: (targetConversationId) => options.getChatFlow()?.probeBoundChannel?.(targetConversationId) ?? Promise.resolve(false),
      resumeSubscription: async (targetConversationId) => {
        const flow = options.getChatFlow();
        if (!flow?.bindActiveConversationStream) return null;
        await flow.bindActiveConversationStream(targetConversationId, true);
        return requestConversationRuntimeSnapshot(targetConversationId);
      },
      applyRuntimeSnapshot: async (runtimeSnapshot) => {
        if (String(options.currentChatConversationId.value || "").trim() !== conversationId) return false;
        const runtimeState = String(runtimeSnapshot.runtimeState || "").trim();
        if (runtimeState === "idle" || runtimeState === "assistant_streaming" || runtimeState === "organizing_context") {
          options.applyConversationRuntimeStateUpdated({ conversationId, runtimeState });
        }
        return (options.getChatFlow()?.resumeForegroundRuntimeRound?.({
          conversationId,
          streamCache: runtimeSnapshot.streamCache || null,
          reason: `foreground_${reason}`,
        }) || 0) > 0;
      },
      refreshMessageById: refreshForegroundTargetMessage,
      finalizeMessage: async () => {
        const flow = options.getChatFlow();
        flow?.clearForegroundRuntimeState?.();
        await Promise.resolve(flow?.unbindActiveConversationStream?.()).catch(() => {});
        options.applyConversationRuntimeStateUpdated({ conversationId, runtimeState: "idle" });
      },
    });
    if (String(options.currentChatConversationId.value || "").trim() !== conversationId) return;

    if (outcome === "handled") {
      await markConversationReadOnForegroundFocus(conversationId);
      return;
    }
    if (outcome === "reload_conversation") {
      await options.switchUnarchivedConversation(conversationId);
      return;
    }

    const currentTailId = currentFormalTailMessageId();
    const latestTailId = await requestLatestFormalTailMessageId(conversationId);
    if (String(options.currentChatConversationId.value || "").trim() !== conversationId) return;
    if (latestTailId === currentTailId) {
      await markConversationReadOnForegroundFocus(conversationId);
      return;
    }
    if (latestTailId && await refreshForegroundTargetMessage(conversationId, latestTailId)) {
      await markConversationReadOnForegroundFocus(conversationId);
      return;
    }
    await options.switchUnarchivedConversation(conversationId);
  }

  async function recoverChatAfterForegroundWakeOnce(reason: string) {
    try {
      await reconcileForegroundConversationAfterWake(reason);
    } catch (error) {
      console.warn("[聊天前台恢复] 状态机执行失败", { reason, error });
    }
    try {
      await options.syncUnarchivedConversationOverviewChangedSinceWatermark(reason);
    } catch (error) {
      console.warn("[聊天前台恢复] 会话概览同步失败", { reason, error });
    }
  }

  const foregroundRecoveryRunner = createLatestTaskRunner(recoverChatAfterForegroundWakeOnce);

  function recoverChatAfterForegroundWake(reason: string) {
    return foregroundRecoveryRunner.run(reason);
  }

  const foregroundActivity = useChatForegroundActivity({
    activeSynced: options.chatWindowActiveSynced,
    isEnabled: () => options.viewMode.value === "chat",
    onWake: recoverChatAfterForegroundWake,
    onBackground: cancelForegroundRecordingOnBackground,
    onWakeError: (reason, error) => {
      console.warn("[聊天前台恢复] 传输恢复失败", { reason, error });
    },
  });

  const clearChatWindowActiveSyncTimer = foregroundActivity.clearSyncTimer;
  const scheduleChatWindowActiveStateSync = foregroundActivity.schedule;
  const syncChatWindowActiveState = foregroundActivity.sync;
  const handleWindowFocusForStateSync = foregroundActivity.handleFocus;
  const handleWindowBlurForStateSync = foregroundActivity.handleBlur;

  function handleVisibilityForStateSync() {
    clearChatMicPrewarmTimer();
    foregroundActivity.handleVisibilityChange();
  }

  function handleWindowFocusForMicPrewarm() {
    scheduleChatMicPrewarm("focus", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
  }

  function handleVisibilityForMicPrewarm() {
    if (document.visibilityState !== "visible") return;
    scheduleChatMicPrewarm("visibility_visible", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
  }

  function cleanupChatForegroundActivity() {
    foregroundRecoveryRunner.cancel();
    foregroundActivity.cleanup();
    clearRecordHotkeyProbeState();
  }

  return {
    recordHotkey,
    foregroundRecordingActive,
    clearChatWindowActiveSyncTimer,
    clearChatMicPrewarmTimer,
    isChatWindowActiveNow,
    startRecording,
    stopRecording,
    cancelForegroundRecordingOnBackground,
    clearRecordHotkeyProbeState,
    scheduleChatWindowActiveStateSync,
    scheduleChatMicPrewarm,
    syncChatWindowActiveState,
    handleWindowFocusForStateSync,
    handleWindowBlurForStateSync,
    handleVisibilityForStateSync,
    cleanupChatForegroundActivity,
    handleWindowFocusForMicPrewarm,
    handleVisibilityForMicPrewarm,
  };
}
