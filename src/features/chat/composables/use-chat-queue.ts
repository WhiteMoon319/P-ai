import { computed, ref, onMounted, onUnmounted, type Ref } from "vue";
import { invokeTauri, onTransportNotification } from "../../../services/tauri-api";

export type ChatQueueEvent = {
  id: string;
  source: "user" | "task" | "delegate" | "system" | "remote_im";
  queueMode: "normal" | "guided";
  createdAt: string;
  messagePreview: string;
  messageText?: string;
  conversationId: string;
};

export type ChatQueueRecallResult = {
  removed: boolean;
  messageText: string;
};

export type MainSessionState = "idle" | "assistant_streaming" | "organizing_context";

type ChatQueueSnapshotPush = {
  queueEvents: ChatQueueEvent[];
  sessionState: MainSessionState;
};

type UseChatQueueOptions = {
  enabled?: Ref<boolean> | boolean;
};

function isMainSessionState(value: unknown): value is MainSessionState {
  return value === "idle" || value === "assistant_streaming" || value === "organizing_context";
}

export function useChatQueue(options: UseChatQueueOptions = {}) {
  const queueEvents = ref<ChatQueueEvent[]>([]);
  const sessionState = ref<MainSessionState>("idle");
  const polling = ref(false);
  const unlisteners: Array<() => void> = [];
  const enabled = computed(() => {
    const configured = options.enabled;
    const configuredValue = typeof configured === "object" && configured && "value" in configured
      ? configured.value
      : configured;
    return configuredValue !== false;
  });

  async function refreshQueue() {
    if (!enabled.value) {
      queueEvents.value = [];
      return;
    }
    try {
      const events = await invokeTauri<ChatQueueEvent[]>("chat.queueSnapshot", {}, 10000);
      queueEvents.value = events || [];
    } catch (error) {
      console.error("[聊天队列] Failed to refresh queue:", error);
      queueEvents.value = [];
    }
  }

  async function refreshSessionState() {
    if (!enabled.value) {
      sessionState.value = "idle";
      return;
    }
    try {
      const state = await invokeTauri<MainSessionState>("chat.sessionStateSnapshot", {}, 10000);
      sessionState.value = state || "idle";
    } catch (error) {
      console.error("[聊天队列] Failed to refresh session state:", error);
    }
  }

  async function recallQueueEvent(eventId: string): Promise<ChatQueueRecallResult> {
    if (!enabled.value) return { removed: false, messageText: "" };
    try {
      const result = await invokeTauri<ChatQueueRecallResult>("chat.queueRecall", { eventId }, 10000);
      if (result?.removed) {
        await refreshQueue();
      }
      return result || { removed: false, messageText: "" };
    } catch (error) {
      console.error("[聊天队列] Failed to recall queue event:", error);
      return { removed: false, messageText: "" };
    }
  }

  async function markGuided(eventId: string): Promise<boolean> {
    if (!enabled.value) return false;
    try {
      const updated = await invokeTauri<boolean>("chat.queueMarkGuided", { eventId }, 10000);
      if (updated) {
        await refreshQueue();
      }
      return updated;
    } catch (error) {
      console.error("[聊天队列] Failed to mark event guided:", error);
      return false;
    }
  }

  async function startPolling() {
    if (polling.value || !enabled.value) return;
    polling.value = true;

    try {
      await refreshQueue();
      await refreshSessionState();
      const applyQueueSnapshot = (payload: ChatQueueSnapshotPush | undefined | null) => {
        queueEvents.value = Array.isArray(payload?.queueEvents) ? payload.queueEvents : [];
        sessionState.value = isMainSessionState(payload?.sessionState) ? payload.sessionState : "idle";
      };
      const refreshRuntimeSnapshot = () => {
        void refreshQueue();
        void refreshSessionState();
      };
      unlisteners.push(onTransportNotification("chat.queueSnapshotUpdated", (payload) => {
        applyQueueSnapshot(payload as ChatQueueSnapshotPush);
      }));
      unlisteners.push(onTransportNotification("chat.roundStarted", refreshRuntimeSnapshot));
      unlisteners.push(onTransportNotification("chat.roundFinished", refreshRuntimeSnapshot));
    } catch (error) {
      polling.value = false;
      while (unlisteners.length > 0) {
        const stop = unlisteners.pop();
        stop?.();
      }
      throw error;
    }
  }

  function stopPolling() {
    while (unlisteners.length > 0) {
      const stop = unlisteners.pop();
      stop?.();
    }
    polling.value = false;
  }

  onMounted(() => {
    void startPolling();
  });

  onUnmounted(() => {
    stopPolling();
  });

  return {
    queueEvents,
    sessionState,
    polling,
    refreshQueue,
    refreshSessionState,
    recallQueueEvent,
    markGuided,
    startPolling,
    stopPolling,
  };
}
