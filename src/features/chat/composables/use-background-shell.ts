import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { invokeTauri, onTransportNotification, onTransportRecovered } from "../../../services/tauri-api";
import type { BackgroundShellTaskSummary } from "../../../types/app";

interface UseBackgroundShellOptions {
  activeConversationId: Ref<string>;
  /** 监控页「后台任务」tab 是否处于激活状态；只有激活时才拉取，事件到达且激活时才刷新 */
  active: Ref<boolean>;
}

type MonitorChangedPayload = {
  domain?: string;
  kind?: string;
  conversationId?: string;
  entityId?: string;
};

export function useBackgroundShell(options: UseBackgroundShellOptions) {
  const { activeConversationId, active } = options;

  const backgroundShells = ref<BackgroundShellTaskSummary[]>([]);
  const backgroundShellsErrorText = ref("");
  let disposed = false;
  let refreshRequestSeq = 0;

  // 事件驱动：统一监控事件只是脏标记，确认 domain 与会话匹配后拉取快照；平时零请求
  const unlistenUpdated = onTransportNotification<MonitorChangedPayload>(
    "monitor.changed",
    (payload) => {
      if (disposed || !active.value) return;
      if (String(payload?.domain || "").trim() !== "backgroundShell") return;
      const payloadConversationId = String(payload?.conversationId || "").trim();
      const activeId = String(activeConversationId.value || "").trim();
      if (!activeId || (payloadConversationId && payloadConversationId !== activeId)) return;
      void refreshBackgroundShells();
    },
  );

  async function refreshBackgroundShells() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!conversationId) return;
    const seq = ++refreshRequestSeq;
    try {
      const tasks = await invokeTauri<BackgroundShellTaskSummary[]>(
        "backgroundShell.list",
        { conversationId },
        10000,
      );
      if (seq !== refreshRequestSeq) return;
      backgroundShells.value = Array.isArray(tasks) ? tasks : [];
      backgroundShellsErrorText.value = "";
    } catch (error) {
      if (seq !== refreshRequestSeq) return;
      backgroundShellsErrorText.value = `后台任务加载失败：${String(error)}`;
    }
  }

  watch(
    [activeConversationId, active],
    ([conversationId, isActive], [prevConversationId]) => {
      const conversationChanged = conversationId !== prevConversationId;
      if (conversationChanged) {
        refreshRequestSeq += 1;
        backgroundShells.value = [];
        backgroundShellsErrorText.value = "";
      }
      // 打开 tab（或会话切换时 tab 已打开）才拉取；关闭 tab 不再请求，列表保留但不更新
      if (isActive && conversationId) {
        void refreshBackgroundShells();
      }
    },
    { immediate: true },
  );

  // 恢复信号兜底：断线重连/前台焦点恢复后重拉快照，补偿期间漏掉的事件
  const unlistenRecovered = onTransportRecovered(() => {
    if (disposed || !active.value) return;
    void refreshBackgroundShells();
  });

  onBeforeUnmount(() => {
    disposed = true;
    unlistenUpdated();
    unlistenRecovered();
  });

  return {
    backgroundShells,
    backgroundShellsErrorText,
    refreshBackgroundShells,
  };
}
