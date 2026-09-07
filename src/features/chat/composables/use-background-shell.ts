import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { invokeTauri, onTransportNotification } from "../../../services/tauri-api";
import type { BackgroundShellTaskSummary } from "../../../types/app";

interface UseBackgroundShellOptions {
  activeConversationId: Ref<string>;
  /** 监控页「后台任务」tab 是否处于激活状态；只有激活时才拉取，事件到达且激活时才刷新 */
  active: Ref<boolean>;
}

type BackgroundShellUpdatedPayload = {
  conversationId?: string;
  taskId?: string;
  status?: string;
};

export function useBackgroundShell(options: UseBackgroundShellOptions) {
  const { activeConversationId, active } = options;

  const backgroundShells = ref<BackgroundShellTaskSummary[]>([]);
  const backgroundShellsErrorText = ref("");
  const terminatingIds = ref<Set<string>>(new Set());
  let disposed = false;
  let refreshRequestSeq = 0;

  // 事件驱动：仅当「后台任务」tab 打开时收到状态更新事件才拉取一次；平时零请求
  const unlistenUpdated = onTransportNotification<BackgroundShellUpdatedPayload>(
    "backgroundShell.updated",
    (payload) => {
      if (disposed || !active.value) return;
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

  async function terminateBackgroundShell(taskId: string) {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!conversationId || terminatingIds.value.has(taskId)) return;
    terminatingIds.value = new Set(terminatingIds.value).add(taskId);
    try {
      await invokeTauri("backgroundShell.terminate", { conversationId, taskId }, 10000);
      await refreshBackgroundShells();
    } catch (error) {
      backgroundShellsErrorText.value = `终止后台任务失败：${String(error)}`;
    } finally {
      const next = new Set(terminatingIds.value);
      next.delete(taskId);
      terminatingIds.value = next;
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

  onBeforeUnmount(() => {
    disposed = true;
    unlistenUpdated();
  });

  return {
    backgroundShells,
    backgroundShellsErrorText,
    terminatingIds,
    refreshBackgroundShells,
    terminateBackgroundShell,
  };
}
