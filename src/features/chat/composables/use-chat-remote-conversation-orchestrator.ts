export function useChatRemoteConversationOrchestrator(bindings: Record<string, any>) {
  async function switchRemoteImContactConversation(contactId: string) {
    const normalizedContactId = String(contactId || "").trim();
    if (!normalizedContactId) return;
    const targetOverview = bindings.remoteImContactConversations.value.find((item: any) =>
      String(item.contactId || "").trim() === normalizedContactId,
    );
    const conversationId = String(targetOverview?.conversationId || "").trim();
    if (!conversationId) return;
    await bindings.switchUnarchivedConversation(conversationId);
  }

  async function switchChatConversation(payload: { kind?: string; conversationId: string; remoteContactId?: string }) {
    const kind = payload.kind === "remote_im_contact" ? "remote_im_contact" : "local_unarchived";
    if (kind === "remote_im_contact") {
      const contactId = String(payload.remoteContactId || "").trim();
      if (contactId) {
        await switchRemoteImContactConversation(contactId);
      } else {
        await bindings.switchUnarchivedConversation(payload.conversationId);
      }
      return;
    }
    await bindings.switchUnarchivedConversation(payload.conversationId);
  }

  return {
    switchRemoteImContactConversation,
    switchChatConversation,
  };
}
