import { computed, ref, watch, onMounted, onBeforeUnmount, type Ref } from "vue";
import { invokeTauri, onTransportNotification, openTransportWindow } from "../../../services/tauri-api";
import type { ConversationDelegateStatusSummary } from "../../../types/app";

const ARCHIVE_FOCUS_REQUEST_STORAGE_KEY = "easy_call.archives.focus_request.v1";

interface UseDelegateStatusOptions {
  activeConversationId: Ref<string>;
  panelOpen: Ref<boolean>;
  enabled?: Ref<boolean>;
}

type DelegateStatusUpdatedPayload = {
  rootConversationId?: string;
  conversationId?: string;
  delegateId?: string;
  status?: string;
  summary?: ConversationDelegateStatusSummary;
};

export function useDelegateStatus(options: UseDelegateStatusOptions) {
  const { activeConversationId, panelOpen } = options;

  const rawDelegateStatuses = ref<ConversationDelegateStatusSummary[]>([]);
  const delegateClockNowMs = ref(Date.now());
  const delegateStatuses = computed<ConversationDelegateStatusSummary[]>(() => {
    const nowMs = delegateClockNowMs.value;
    return rawDelegateStatuses.value.map((status) => ({
      ...status,
      elapsedMs: delegateElapsedMs(status, nowMs),
    }));
  });
  const delegateStatusesErrorText = ref("");
  const enabled = () => options.enabled?.value !== false;

  let delegateStatusUpdatedUnlisten: (() => void) | null = null;
  let delegateClockTimer: ReturnType<typeof window.setInterval> | null = null;
  let disposed = false;
  let hydrateRequestSeq = 0;
  let hydratedConversationId = "";

  function payloadMatchesActiveConversation(payload: DelegateStatusUpdatedPayload | null | undefined) {
    const activeId = String(activeConversationId.value || "").trim();
    if (!activeId) return false;
    const rootConversationId = String(payload?.rootConversationId || "").trim();
    if (rootConversationId) return rootConversationId === activeId;
    return String(payload?.conversationId || "").trim() === activeId;
  }

  function clearStatusesWhenConversationChanges() {
    hydrateRequestSeq += 1;
    hydratedConversationId = "";
    rawDelegateStatuses.value = [];
    delegateStatusesErrorText.value = "";
  }

  async function hydrateStatusesWhenPanelOpens() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!enabled() || !panelOpen.value || !conversationId || hydratedConversationId === conversationId) return;
    const seq = ++hydrateRequestSeq;
    try {
      const statuses = await invokeTauri<ConversationDelegateStatusSummary[]>(
        "delegate.statuses",
        { conversationId },
        10000,
      );
      if (seq !== hydrateRequestSeq || !panelOpen.value || activeConversationId.value.trim() !== conversationId) return;
      rawDelegateStatuses.value = Array.isArray(statuses) ? statuses : [];
      delegateClockNowMs.value = Date.now();
      delegateStatusesErrorText.value = "";
      hydratedConversationId = conversationId;
    } catch (error) {
      if (seq !== hydrateRequestSeq) return;
      delegateStatusesErrorText.value = `委托状态加载失败：${String(error)}`;
    }
  }

  function delegateElapsedMs(status: ConversationDelegateStatusSummary, nowMs: number) {
    const startedAtMs = parseTimeMs(status?.startedAt);
    if (startedAtMs <= 0) {
      return positiveNumber(status?.elapsedMs);
    }
    const completedAtMs = parseTimeMs(status?.completedAt || status?.archivedAt);
    const endAtMs = completedAtMs > 0 ? completedAtMs : nowMs;
    if (endAtMs <= startedAtMs) return 0;
    return endAtMs - startedAtMs;
  }

  function parseTimeMs(value: unknown) {
    const raw = String(value || "").trim();
    if (!raw) return 0;
    const parsed = Date.parse(raw);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
  }

  function positiveNumber(value: unknown) {
    const normalized = Math.round(Number(value) || 0);
    return Number.isFinite(normalized) && normalized > 0 ? normalized : 0;
  }

  function hasRunningDelegates() {
    return rawDelegateStatuses.value.some((status) => {
      const current = String(status?.status || "").trim();
      return status.active && (current === "running" || current === "delivered");
    });
  }

  function syncDelegateClockTimer() {
    const shouldRun = enabled() && panelOpen.value && hasRunningDelegates();
    if (shouldRun && delegateClockTimer == null && typeof window !== "undefined") {
      delegateClockTimer = window.setInterval(() => {
        delegateClockNowMs.value = Date.now();
      }, 1000);
    } else if (!shouldRun && delegateClockTimer != null) {
      window.clearInterval(delegateClockTimer);
      delegateClockTimer = null;
    }
  }

  async function openDelegateArchiveDetail(status: ConversationDelegateStatusSummary) {
    const conversationId = String(status?.conversationId || status?.delegateId || "").trim();
    if (!conversationId) return;
    try {
      if (typeof window !== "undefined") {
        window.localStorage.setItem(ARCHIVE_FOCUS_REQUEST_STORAGE_KEY, JSON.stringify({
          conversationId,
          viewMode: "delegate",
          createdAt: Date.now(),
        }));
      }
      await openTransportWindow("archives");
    } catch (error) {
      delegateStatusesErrorText.value = `打开委托归档失败：${String(error)}`;
    }
  }

  async function abortDelegate(status: ConversationDelegateStatusSummary) {
    const delegateId = String(status?.delegateId || "").trim();
    if (!delegateId) return;
    try {
      await invokeTauri("delegate.abort", { delegateId }, 10000);
    } catch (error) {
      delegateStatusesErrorText.value = `打断委托失败：${String(error)}`;
    }
  }

  watch(
    () => String(activeConversationId.value || "").trim(),
    () => clearStatusesWhenConversationChanges(),
    { immediate: true },
  );

  watch(
    () => [enabled(), panelOpen.value, String(activeConversationId.value || "").trim()],
    () => { void hydrateStatusesWhenPanelOpens(); },
    { immediate: true },
  );

  watch(
    () => [enabled(), panelOpen.value, rawDelegateStatuses.value.map((status) => `${status.delegateId}:${status.active}:${status.status}:${status.startedAt}:${status.completedAt || ""}`).join("|")],
    () => syncDelegateClockTimer(),
    { immediate: true },
  );

  onMounted(() => {
    const unlisten = onTransportNotification<DelegateStatusUpdatedPayload>(
      "conversation.delegateStatusUpdated",
      applyStatusEvent,
    );
    if (disposed) unlisten();
    else delegateStatusUpdatedUnlisten = unlisten;
  });

  onBeforeUnmount(() => {
    disposed = true;
    if (delegateStatusUpdatedUnlisten) {
      delegateStatusUpdatedUnlisten();
      delegateStatusUpdatedUnlisten = null;
    }
    if (delegateClockTimer != null) {
      window.clearInterval(delegateClockTimer);
      delegateClockTimer = null;
    }
  });

  function applyStatusEvent(payload: DelegateStatusUpdatedPayload | null | undefined) {
    if (!enabled() || !payloadMatchesActiveConversation(payload)) return;
    const delegateId = String(payload?.delegateId || "").trim();
    const status = String(payload?.status || "").trim();
    const rootConversationId = String(payload?.rootConversationId || payload?.conversationId || "").trim();
    if (!delegateId || !status || !rootConversationId) return;

    const now = new Date().toISOString();
    const active = status === "running" || status === "delivered";
    const eventSummary = payload?.summary;
    const summary = eventSummary
      && eventSummary.delegateId === delegateId
      && eventSummary.rootConversationId === rootConversationId
      ? {
          ...eventSummary,
          status,
          active,
          updatedAt: now,
          completedAt: active ? undefined : eventSummary.completedAt || now,
        }
      : null;
    const index = rawDelegateStatuses.value.findIndex((item) => item.delegateId === delegateId);
    if (index >= 0) {
      const current = rawDelegateStatuses.value[index];
      rawDelegateStatuses.value.splice(index, 1, {
        ...current,
        ...summary,
        status,
        active,
        updatedAt: now,
        completedAt: active ? undefined : current.completedAt || now,
      });
    } else {
      rawDelegateStatuses.value.push(summary || {
        delegateId,
        kind: "",
        conversationId: delegateId,
        rootConversationId,
        title: "",
        status,
        active,
        startedAt: now,
        updatedAt: now,
        completedAt: active ? undefined : now,
        elapsedMs: 0,
        requestCount: 0,
        toolCallCount: 0,
        lastToolName: "",
        tokenCount: 0,
      });
    }
    delegateClockNowMs.value = Date.now();
    delegateStatusesErrorText.value = "";
  }

  return {
    delegateStatuses,
    delegateStatusesErrorText,
    openDelegateArchiveDetail,
    abortDelegate,
  };
}
