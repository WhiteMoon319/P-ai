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
  syncUnarchivedConversationOverviewChangedSinceWatermark: (reason?: string) => Promise<void>;
  freezeForegroundConversation: (reason: string) => void;
  restoreForegroundConversationProjection: (conversationId: string, reason: string) => Promise<void>;
  switchUnarchivedConversation: (conversationId: string) => Promise<void>;
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
    const snapshot = await invokeTauri<any>("get_foreground_conversation_freshness_snapshot", {
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
    await invokeTauri("mark_conversation_read", {
      input: { conversationId: normalizedConversationId },
    });
  }

  async function requestConversationRuntimeSnapshot(conversationId: string): Promise<ConversationRuntimeSnapshot> {
    return invokeTauri<ConversationRuntimeSnapshot>("get_conversation_runtime_snapshot", {
      conversationId,
    });
  }

  async function recoverForegroundConversationBySwitch(conversationId: string) {
    // focus 只负责判断前台是否过时；一旦确认过时，统一走“切到当前会话”的唯一恢复路径，
    // 禁止在 focus 分支里各自补正文/补运行态，否则一定会出现恢复分叉。
    await options.switchUnarchivedConversation(conversationId);
  }

  async function reconcileForegroundConversationAfterFreeze(conversationId: string, _reason: string) {
    const chatFlow = options.getChatFlow();

    // 第一步只判断“流式绑定通道是否还活着”。
    // 这里绝不能把 probe=true 误当成正文健康，只能用来判断是否需要先重绑当前会话的流式通道。
    if (!chatFlow?.probeBoundChannel) {
      if (chatFlow?.bindActiveConversationStream) {
        await chatFlow.bindActiveConversationStream(conversationId, true);
      }
      await recoverForegroundConversationBySwitch(conversationId);
      return;
    }

    const probeHealthy = await chatFlow.probeBoundChannel(conversationId);
    if (!probeHealthy) {
      if (chatFlow?.bindActiveConversationStream) {
        await chatFlow.bindActiveConversationStream(conversationId, true);
      }
      await recoverForegroundConversationBySwitch(conversationId);
      return;
    }

    const runtimeSnapshot = await requestConversationRuntimeSnapshot(conversationId);
    const runtimeState = String(runtimeSnapshot?.runtimeState || "").trim();
    if (runtimeState === "assistant_streaming" || runtimeState === "organizing_context" || runtimeState === "compacting") {
      return;
    }

    // 后端没有明确流式态时，再检查正式消息是否已是最新。
    const currentTailId = currentFormalTailMessageId();
    const latestTailId = await requestLatestFormalTailMessageId(conversationId);
    if (latestTailId === currentTailId) {
      try {
        await markConversationReadOnForegroundFocus(conversationId);
      }
      catch (error) {
        console.warn("[聊天前台恢复] focus 已读同步失败:", error);
      }
      return;
    }

    await recoverForegroundConversationBySwitch(conversationId);
  }

  async function recoverChatAfterForegroundWake(reason: string) {
    const activeConversationId = String(options.currentChatConversationId.value || "").trim();
    if (activeConversationId) {
      await reconcileForegroundConversationAfterFreeze(activeConversationId, reason);
    }
    await options.syncUnarchivedConversationOverviewChangedSinceWatermark(reason);
  }

  async function syncChatWindowActiveState(reason = "unknown") {
    if (!isPrimaryChatWindow()) return;
    const active = isChatWindowActiveNow();
    const activeChanged = options.chatWindowActiveSynced.value !== active;
    if (!activeChanged && !active) return;
    options.chatWindowActiveSynced.value = active;
    if (active) {
      await recoverChatAfterForegroundWake(reason);
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
