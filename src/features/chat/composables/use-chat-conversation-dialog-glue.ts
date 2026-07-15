export function useChatConversationDialogGlue(bindings: Record<string, any>) {
  async function deleteUnarchivedConversationFromArchives(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const deletingCurrentConversation = currentConversationId === normalizedConversationId;
    if (deletingCurrentConversation) {
      bindings.clearForegroundConversation("delete_unarchived_conversation_current");
    }
    const result = await bindings.deleteUnarchivedConversationFromArchivesRaw(normalizedConversationId);
    if (!deletingCurrentConversation) return;
    if (String(bindings.currentChatConversationId.value || "").trim()) return;
    const nextConversationId = String(result?.activeConversationId || "").trim();
    if (nextConversationId) {
      await bindings.switchUnarchivedConversation(nextConversationId);
      return;
    }
    await bindings.recoverForegroundConversationFromOverview(
      "delete_unarchived_conversation_current_missing_replacement",
    );
  }

  async function archiveConversationFromList(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    console.info("[会话归档] 点击归档会话", {
      conversationId: normalizedConversationId,
      source: "conversation_list",
    });
    try {
      await bindings.archiveCurrentConversation(normalizedConversationId);
    } catch (error) {
      console.warn("[会话归档] 归档会话失败", {
        conversationId: normalizedConversationId,
        error,
      });
      bindings.setStatusError("status.trimArchiveFailed", error);
    }
  }

  async function handleConfirmTrimAction() {
    await bindings.getConfirmTrimAction()();
  }

  return {
    deleteUnarchivedConversationFromArchives,
    archiveConversationFromList,
    handleConfirmTrimAction,
  };
}
