import { computed, ref, onMounted, onUnmounted, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";

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
  request?: Ref<(<T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>) | undefined>;
  subscribe?: Ref<((method: string, handler: (payload: unknown) => void) => () => void) | undefined>;
};

function isMainSessionState(value: unknown): value is MainSessionState {
  return value === "idle" || value === "assistant_streaming" || value === "organizing_context";
}

export function useChatQueue(options: UseChatQueueOptions = {}) {
  const queueEvents = ref<ChatQueueEvent[]>([]);
  const sessionState = ref<MainSessionState>("idle");
  const polling = ref(false);
  const unlisteners: Array<UnlistenFn | (() => void)> = [];
  const bridgeRequest = computed(() => options.request?.value);
  const bridgeSubscribe = computed(() => options.subscribe?.value);
  const enabled = computed(() => {
    const configured = options.enabled;
    const configuredValue = typeof configured === "object" && configured && "value" in configured
      ? configured.value
      : configured;
    return configuredValue !== false && (!!bridgeRequest.value || isTauriRuntimeAvailable());
  });

  async function refreshQueue() {
    if (!enabled.value) {
      queueEvents.value = [];
      return;
    }
    try {
      const request = bridgeRequest.value;
      const events = request
        ? await request<ChatQueueEvent[]>("chat.queueSnapshot", {}, 10000)
        : await invokeTauri<ChatQueueEvent[]>("get_chat_queue_snapshot");
      queueEvents.value = events || [];
    } catch (error) {
      console.error("[CHAT-QUEUE] Failed to refresh queue:", error);
      queueEvents.value = [];
    }
  }

  async function refreshSessionState() {
    if (!enabled.value) {
      sessionState.value = "idle";
      return;
    }
    try {
      const request = bridgeRequest.value;
      const state = request
        ? await request<MainSessionState>("chat.sessionStateSnapshot", {}, 10000)
        : await invokeTauri<MainSessionState>("get_main_session_state_snapshot");
      sessionState.value = state || "idle";
    } catch (error) {
      console.error("[CHAT-QUEUE] Failed to refresh session state:", error);
    }
  }

  async function recallQueueEvent(eventId: string): Promise<ChatQueueRecallResult> {
    if (!enabled.value) return { removed: false, messageText: "" };
    try {
      const request = bridgeRequest.value;
      const result = request
        ? await request<ChatQueueRecallResult>("chat.queueRecall", { eventId }, 10000)
        : await invokeTauri<ChatQueueRecallResult>("recall_chat_queue_event", { eventId });
      if (result?.removed) {
        await refreshQueue();
      }
      return result || { removed: false, messageText: "" };
    } catch (error) {
      console.error("[CHAT-QUEUE] Failed to recall queue event:", error);
      return { removed: false, messageText: "" };
    }
  }

  async function markGuided(eventId: string): Promise<boolean> {
    if (!enabled.value) return false;
    try {
      const request = bridgeRequest.value;
      const updated = request
        ? await request<boolean>("chat.queueMarkGuided", { eventId }, 10000)
        : await invokeTauri<boolean>("mark_chat_queue_event_guided", { eventId });
      if (updated) {
        await refreshQueue();
      }
      return updated;
    } catch (error) {
      console.error("[CHAT-QUEUE] Failed to mark event guided:", error);
      return false;
    }
  }

  async function startPolling() {
    if (polling.value || !enabled.value) return;
    polling.value = true;

    try {
      await refreshQueue();
      await refreshSessionState();
      const subscribe = bridgeSubscribe.value;
      const applyQueueSnapshot = (payload: ChatQueueSnapshotPush | undefined | null) => {
        queueEvents.value = Array.isArray(payload?.queueEvents) ? payload.queueEvents : [];
        sessionState.value = isMainSessionState(payload?.sessionState) ? payload.sessionState : "idle";
      };
      const refreshRuntimeSnapshot = () => {
        void refreshQueue();
        void refreshSessionState();
      };
      if (subscribe) {
        unlisteners.push(subscribe("chat.queueSnapshotUpdated", (payload) => {
          applyQueueSnapshot(payload as ChatQueueSnapshotPush);
        }));
        unlisteners.push(subscribe("chat.roundStarted", refreshRuntimeSnapshot));
        unlisteners.push(subscribe("chat.roundFinished", refreshRuntimeSnapshot));
      } else {
        unlisteners.push(await listen<ChatQueueSnapshotPush>("easy-call:chat-queue-snapshot", (event) => {
          applyQueueSnapshot(event.payload);
        }));
        unlisteners.push(await listen("easy-call:round-started", refreshRuntimeSnapshot));
        unlisteners.push(await listen("easy-call:round-completed", refreshRuntimeSnapshot));
        unlisteners.push(await listen("easy-call:round-failed", refreshRuntimeSnapshot));
      }
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
