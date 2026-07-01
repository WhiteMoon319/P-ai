import { ref, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig, ChatMessage } from "../../../types/app";
import { formalizeMessages } from "./use-chat-flow-utils";
import { useRecordHotkey } from "./use-record-hotkey";

type RecordingActivationSource = "foreground" | "background";

type UseChatWindowRecordingOrchestratorOptions = {
  viewMode: Ref<"chat" | "archives" | "config">;
  config: AppConfig;
  recording: Ref<boolean>;
  tauriWindowLabel: Ref<string>;
  isChatTauriWindow: Ref<boolean>;
  detachedChatWindow: Ref<boolean>;
  currentChatConversationId: Ref<string>;
  currentForegroundAgentId: Ref<string>;
  startupDataReady: Ref<boolean>;
  recordHotkeyProbeLastSeq: Ref<number>;
  recordHotkeyProbeDown: Ref<boolean>;
  chatWindowActiveSynced: Ref<boolean | null>;
  allMessages: Ref<ChatMessage[]>;
  foregroundSnapshotRecentLimit: number;
  backgroundConversationCacheLimit: number;
  getChatFlow: () => {
    probeBoundChannel?: (conversationId?: string | null, timeoutMs?: number) => Promise<boolean>;
    bindActiveConversationStream?: (conversationId: string, force?: boolean) => Promise<void>;
    resumeForegroundRuntimeRound?: (input?: {
      conversationId?: string | null;
      streamCache?: unknown;
      statusText?: string;
      reason?: string;
    }) => number;
  } | null | undefined;
  applyConversationRuntimeStateUpdated: (payload: {
    conversationId: string;
    runtimeState: "idle" | "assistant_streaming" | "organizing_context";
  }) => void;
  startSpeechRecording: () => Promise<unknown>;
  stopSpeechRecording: (discard: boolean) => Promise<unknown>;
  prewarmMicrophone: () => Promise<unknown>;
  refreshChatUnarchivedConversations: () => Promise<void>;
  freezeForegroundConversation: (reason: string) => void;
  restoreForegroundConversationProjection: (conversationId: string, reason: string) => Promise<void>;
};

type ConversationRuntimeSnapshot = {
  runtimeState?: string;
  isProcessing?: boolean;
  hasPendingQueue?: boolean;
  pendingQueueCount?: number;
  streamCache?: {
    hasVisibleProgress?: boolean;
    toolStatusState?: string;
  } | null;
};

export function useChatWindowRecordingOrchestrator(options: UseChatWindowRecordingOrchestratorOptions) {
  const CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS = 260;
  const foregroundRecordingActive = ref(false);
  let chatWindowActiveSyncTimer: ReturnType<typeof setTimeout> | null = null;
  let chatMicPrewarmTimer: ReturnType<typeof setTimeout> | null = null;

  function clearChatWindowActiveSyncTimer() {
    if (!chatWindowActiveSyncTimer) return;
    clearTimeout(chatWindowActiveSyncTimer);
    chatWindowActiveSyncTimer = null;
  }

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
    onStartRecording: () => startRecording("foreground"),
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

  function isPrimaryChatWindow(): boolean {
    return options.tauriWindowLabel.value === "chat" && !options.detachedChatWindow.value;
  }

  function clearRecordHotkeyProbeState() {
    options.recordHotkeyProbeDown.value = false;
    options.recordHotkeyProbeLastSeq.value = 0;
  }

  function scheduleChatWindowActiveStateSync(reason: string, delayMs = 0) {
    if (!isPrimaryChatWindow()) return;
    clearChatWindowActiveSyncTimer();
    if (delayMs <= 0) {
      void syncChatWindowActiveState(reason);
      return;
    }
    chatWindowActiveSyncTimer = setTimeout(() => {
      chatWindowActiveSyncTimer = null;
      void syncChatWindowActiveState(reason);
    }, delayMs);
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
    const snapshot = await invokeTauri<any>("get_foreground_conversation_light_snapshot", {
      input: {
        conversationId,
        agentId: String(options.currentForegroundAgentId.value || "").trim() || null,
        limit: options.foregroundSnapshotRecentLimit,
      },
    });
    const messages = formalizeMessages(Array.isArray(snapshot?.messages) ? snapshot.messages : []);
    return String(messages[messages.length - 1]?.id || "").trim();
  }

  async function requestConversationRuntimeSnapshot(conversationId: string): Promise<ConversationRuntimeSnapshot> {
    return invokeTauri<ConversationRuntimeSnapshot>("get_conversation_runtime_snapshot", {
      conversationId,
    });
  }

  async function requestMissingFormalMessages(conversationId: string, afterMessageId: string | null) {
    await invokeTauri("request_conversation_messages_after_async", {
      input: {
        conversationId,
        afterMessageId,
        fallbackLimit: options.backgroundConversationCacheLimit,
      },
    });
  }

  async function reconcileForegroundConversationAfterFreeze(conversationId: string, reason: string) {
    const chatFlow = options.getChatFlow();
    if (!chatFlow?.probeBoundChannel) {
      console.warn("[聊天前台恢复][诊断] focus 对账跳过：probe 不可用", {
        conversationId,
        reason,
        restoreMode: "probe_unavailable_skip",
      });
      return;
    }

    // 别在 focus 上写任何“先恢复一下”的狗屁降级。
    // 聊天窗口恢复焦点的真正原因是 WebView 可能冻结过；channel 还活着时前台就是健康的，乱刷只会把正确画面刷坏。
    const probeHealthy = await chatFlow.probeBoundChannel(conversationId);
    if (probeHealthy) {
      console.info("[聊天前台恢复][诊断] focus 对账完成：channel 仍然健康，禁止恢复", {
        conversationId,
        reason,
        restoreMode: "probe_success_skip",
      });
      return;
    }

    const runtimeSnapshot = await requestConversationRuntimeSnapshot(conversationId);
    const runtimeState = String(runtimeSnapshot?.runtimeState || "").trim();
    const isProcessing = !!runtimeSnapshot?.isProcessing;
    const hasPendingQueue = !!runtimeSnapshot?.hasPendingQueue
      || Math.max(0, Number(runtimeSnapshot?.pendingQueueCount || 0)) > 0;
    const hasVisibleProgress = !!runtimeSnapshot?.streamCache?.hasVisibleProgress;

    if (runtimeState === "assistant_streaming" || isProcessing || hasPendingQueue) {
      await chatFlow.bindActiveConversationStream?.(conversationId, true);
      if (runtimeState === "assistant_streaming") {
        options.applyConversationRuntimeStateUpdated({
          conversationId,
          runtimeState: "assistant_streaming",
        });
      }
      chatFlow.resumeForegroundRuntimeRound?.({
        conversationId,
        streamCache: runtimeSnapshot?.streamCache || null,
        reason,
      });
      console.warn("[聊天前台恢复][诊断] focus 对账命中运行中恢复路径", {
        conversationId,
        reason,
        runtimeState,
        isProcessing,
        hasPendingQueue,
        hasVisibleProgress,
        restoreMode: hasVisibleProgress
          ? "probe_failed_resume_streaming"
          : "probe_failed_resume_waiting",
      });
      return;
    }

    if (runtimeState === "organizing_context") {
      options.applyConversationRuntimeStateUpdated({
        conversationId,
        runtimeState: "organizing_context",
      });
      console.warn("[聊天前台恢复][诊断] focus 对账命中整理上下文恢复路径", {
        conversationId,
        reason,
        restoreMode: "probe_failed_resume_compacting",
      });
      return;
    }

    // 最新正式消息 ID 只能在彻底空闲时比较。流式、等待工具、整理上下文时拿消息 ID 硬判落后，等于主动把没坏的画面刷坏。
    const currentTailId = currentFormalTailMessageId();
    const latestTailId = await requestLatestFormalTailMessageId(conversationId);
    if (latestTailId === currentTailId) {
      console.info("[聊天前台恢复][诊断] focus 对账完成：当前前台已经是最新", {
        conversationId,
        reason,
        restoreMode: "probe_failed_but_already_latest",
        currentTailId,
      });
      return;
    }

    await requestMissingFormalMessages(conversationId, currentTailId || null);
    console.warn("[聊天前台恢复][诊断] focus 对账命中正式消息补缺路径", {
      conversationId,
      reason,
      restoreMode: "probe_failed_append_missing_messages",
      currentTailId,
      latestTailId,
    });
  }

  async function syncChatWindowActiveState(reason = "unknown") {
    if (!isPrimaryChatWindow()) return;
    const active = isChatWindowActiveNow();
    if (options.chatWindowActiveSynced.value === active) return;
    options.chatWindowActiveSynced.value = active;
    if (active) {
      void stopRecording(false);
      const activeConversationId = String(options.currentChatConversationId.value || "").trim();
      if (activeConversationId) {
        await reconcileForegroundConversationAfterFreeze(activeConversationId, reason);
      }
    }
    clearRecordHotkeyProbeState();
    void invokeTauri("set_chat_window_active", { active }).catch((error) => {
      console.warn("[热键] 设置聊天窗口激活状态失败:", error);
    });
  }

  function handleWindowFocusForStateSync() {
    scheduleChatWindowActiveStateSync("focus");
  }

  function handleWindowBlurForStateSync() {
    cancelForegroundRecordingOnBackground("blur");
    scheduleChatWindowActiveStateSync("blur");
  }

  function handleVisibilityForStateSync() {
    clearChatWindowActiveSyncTimer();
    clearChatMicPrewarmTimer();
    if (options.isChatTauriWindow.value && document.visibilityState !== "visible") {
      cancelForegroundRecordingOnBackground("visibility_hidden");
    }
    void syncChatWindowActiveState("visibilitychange");
  }

  function handleWindowFocusForMicPrewarm() {
    scheduleChatMicPrewarm("focus", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
  }

  function handleVisibilityForMicPrewarm() {
    if (document.visibilityState !== "visible") return;
    scheduleChatMicPrewarm("visibility_visible", CHAT_WINDOW_MIC_PREWARM_DEBOUNCE_MS);
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
    isPrimaryChatWindow,
    clearRecordHotkeyProbeState,
    scheduleChatWindowActiveStateSync,
    scheduleChatMicPrewarm,
    syncChatWindowActiveState,
    handleWindowFocusForStateSync,
    handleWindowBlurForStateSync,
    handleVisibilityForStateSync,
    handleWindowFocusForMicPrewarm,
    handleVisibilityForMicPrewarm,
  };
}
