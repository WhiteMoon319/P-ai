import { ref, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig, ChatMessage } from "../../../types/app";
import { formalizeMessages } from "./use-chat-flow-utils";
import { useRecordHotkey } from "./use-record-hotkey";

const CHAT_FOCUS_RECOVERY_DEBUG = (() => {
  if (typeof window === "undefined") return false;
  const stored = window.localStorage.getItem("easy-call.debug.chat-focus-recovery");
  if (stored === "1") return true;
  if (stored === "0") return false;
  return !!import.meta.env.DEV;
})();

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
  let focusReconcileSeq = 0;

  function nextFocusReconcileSeq(): number {
    focusReconcileSeq += 1;
    return focusReconcileSeq;
  }

  function logFocusReconcile(
    seq: number,
    stage: string,
    detail?: Record<string, unknown>,
    level: "info" | "warn" = "info",
  ) {
    if (!CHAT_FOCUS_RECOVERY_DEBUG) return;
    const payload = {
      seq,
      stage,
      ...detail,
    };
    if (level === "warn") {
      console.warn("[聊天前台恢复][状态机]", payload);
      return;
    }
    console.info("[聊天前台恢复][状态机]", payload);
  }

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

  function hasForegroundStreamingBubble(): boolean {
    const messages = Array.isArray(options.allMessages.value) ? options.allMessages.value : [];
    return messages.some((message) => {
      if (String(message?.role || "").trim() !== "assistant") return false;
      const meta = (message?.providerMeta || {}) as Record<string, unknown>;
      return meta._streaming === true;
    });
  }

  async function requestLatestFormalTailMessageId(conversationId: string): Promise<string> {
    const snapshot = await invokeTauri<any>("get_foreground_conversation_light_snapshot", {
      input: {
        conversationId,
        agentId: null,
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

  async function recoverForegroundConversationBySwitch(conversationId: string, reason: string, seq: number, staleReason: string) {
    // focus 只负责判断前台是否过时；一旦确认过时，统一走“切到当前会话”的唯一恢复路径，
    // 禁止在 focus 分支里各自补正文/补运行态，否则一定会出现恢复分叉。
    logFocusReconcile(seq, "判定过时，准备统一切会话", {
      conversationId,
      reason,
      staleReason,
    }, "warn");
    await options.switchUnarchivedConversation(conversationId);
    logFocusReconcile(seq, "统一切会话完成", {
      conversationId,
      reason,
      staleReason,
      restoreMode: "focus_stale_switch_current_conversation",
    }, "warn");
  }

  async function reconcileForegroundConversationAfterFreeze(conversationId: string, reason: string) {
    const seq = nextFocusReconcileSeq();
    const chatFlow = options.getChatFlow();
    const streamingBubblePresentAtStart = hasForegroundStreamingBubble();
    logFocusReconcile(seq, "开始 focus 对账", {
      conversationId,
      reason,
      hasProbeBoundChannel: !!chatFlow?.probeBoundChannel,
      hasBindActiveConversationStream: !!chatFlow?.bindActiveConversationStream,
      currentConversationId: String(options.currentChatConversationId.value || "").trim(),
      currentTailId: currentFormalTailMessageId(),
      streamingBubblePresentAtStart,
    });

    // 第一步只判断“流式绑定通道是否还活着”。
    // 这里绝不能把 probe=true 误当成正文健康，只能用来判断是否需要先重绑当前会话的流式通道。
    if (!chatFlow?.probeBoundChannel) {
      logFocusReconcile(seq, "probe 不可用，先重绑后恢复", {
        conversationId,
        reason,
        restoreMode: "probe_unavailable_rebind_and_switch",
      }, "warn");
      if (chatFlow?.bindActiveConversationStream) {
        await chatFlow.bindActiveConversationStream(conversationId, true);
        logFocusReconcile(seq, "probe 不可用时重绑完成", {
          conversationId,
          reason,
        }, "warn");
      }
      await recoverForegroundConversationBySwitch(
        conversationId,
        reason,
        seq,
        "probe_unavailable",
      );
      return;
    }

    const probeHealthy = await chatFlow.probeBoundChannel(conversationId);
    logFocusReconcile(seq, "probe 完成", {
      conversationId,
      reason,
      probeHealthy,
    });
    if (!probeHealthy) {
      logFocusReconcile(seq, "probe 失败，先重绑后恢复", {
        conversationId,
        reason,
      }, "warn");
      if (chatFlow?.bindActiveConversationStream) {
        await chatFlow.bindActiveConversationStream(conversationId, true);
        logFocusReconcile(seq, "probe 失败后重绑完成", {
          conversationId,
          reason,
        }, "warn");
      }
      await recoverForegroundConversationBySwitch(
        conversationId,
        reason,
        seq,
        "stream_channel_broken",
      );
      return;
    }

    const runtimeSnapshot = await requestConversationRuntimeSnapshot(conversationId);
    const runtimeState = String(runtimeSnapshot?.runtimeState || "").trim();
    const streamingBubblePresent = hasForegroundStreamingBubble();
    logFocusReconcile(seq, "读取运行态快照完成", {
      conversationId,
      reason,
      runtimeState,
      isProcessing: !!runtimeSnapshot?.isProcessing,
      hasPendingQueue: !!runtimeSnapshot?.hasPendingQueue,
      pendingQueueCount: Math.max(0, Number(runtimeSnapshot?.pendingQueueCount || 0)),
      hasVisibleProgress: !!runtimeSnapshot?.streamCache?.hasVisibleProgress,
      toolStatusState: String(runtimeSnapshot?.streamCache?.toolStatusState || "").trim(),
      streamingBubblePresent,
    });

    if (runtimeState === "assistant_streaming" || runtimeState === "organizing_context" || runtimeState === "compacting") {
      logFocusReconcile(seq, "运行态仅记录，不触发强制切回", {
        conversationId,
        reason,
        runtimeState,
        streamingBubblePresent,
      });
      return;
    }

    // 后端没有明确流式态时，再检查正式消息是否已是最新。
    const currentTailId = currentFormalTailMessageId();
    const latestTailId = await requestLatestFormalTailMessageId(conversationId);
    logFocusReconcile(seq, "正式消息尾部比较完成", {
      conversationId,
      reason,
      currentTailId,
      latestTailId,
      tailMatched: latestTailId === currentTailId,
    });
    if (latestTailId === currentTailId) {
      logFocusReconcile(seq, "tail 判定前台未过时", {
        conversationId,
        reason,
        restoreMode: "formal_tail_already_latest",
        currentTailId,
      });
      return;
    }

    await recoverForegroundConversationBySwitch(
      conversationId,
      reason,
      seq,
      "formal_tail_mismatch",
    );
  }

  async function syncChatWindowActiveState(reason = "unknown") {
    if (!isPrimaryChatWindow()) return;
    const active = isChatWindowActiveNow();
    if (CHAT_FOCUS_RECOVERY_DEBUG) {
      console.info("[聊天前台恢复][状态机]", {
        stage: "窗口激活状态同步",
        reason,
        active,
        previousActive: options.chatWindowActiveSynced.value,
        currentConversationId: String(options.currentChatConversationId.value || "").trim(),
      });
    }
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
