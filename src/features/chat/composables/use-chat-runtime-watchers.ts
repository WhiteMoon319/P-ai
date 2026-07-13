import { watch } from "vue";

export function useChatRuntimeWatchers(bindings: Record<string, any>) {
  watch(
    () => ({
      mode: bindings.viewMode.value,
      departmentId: String(bindings.currentForegroundDepartmentId.value || "").trim(),
      agentId: String(bindings.currentForegroundAgentId.value || "").trim(),
    }),
    ({ mode }) => {
      if (mode !== "chat" || !bindings.startupDataReady.value) return;
      const syncOverview = typeof bindings.syncUnarchivedConversationOverviewChangedSinceWatermark === "function"
        ? bindings.syncUnarchivedConversationOverviewChangedSinceWatermark
        : bindings.refreshChatUnarchivedConversations;
      void syncOverview("runtime_watcher").catch((error: unknown) => {
        bindings.setStatusError("status.loadMessagesFailed", error);
      });
    },
    { immediate: true },
  );

  watch(
    () => ({
      mode: bindings.viewMode.value,
      conversationId: String(bindings.currentChatConversationId.value || "").trim(),
    }),
    ({ mode, conversationId }) => {
      if (mode !== "chat" || !bindings.startupDataReady.value) return;
      void (async () => {
        try {
          await bindings.getChatFlow().bindActiveConversationStream(conversationId);
        } catch (error) {
          console.warn("[聊天推送] 绑定前台流失败", {
            conversationId,
            error,
          });
        }
      })();
    },
    { immediate: true },
  );
}
