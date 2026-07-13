export function useChatConversationOverviewUtils() {
  function unarchivedConversationActivityAt(item: Record<string, any>): string {
    return String(item.lastMessageAt || item.updatedAt || "").trim();
  }

  function sortUnarchivedConversationOverviewItems(items: any[]): any[] {
    return [...items].sort((a, b) => {
      if (!!a.isSystemNotificationConversation !== !!b.isSystemNotificationConversation) {
        return Number(!!b.isSystemNotificationConversation) - Number(!!a.isSystemNotificationConversation);
      }
      if (!!a.isPinned !== !!b.isPinned) {
        return Number(!!b.isPinned) - Number(!!a.isPinned);
      }
      if (a.isPinned && b.isPinned) {
        const aIndex = Number.isFinite(Number(a.pinIndex)) ? Number(a.pinIndex) : Number.MAX_SAFE_INTEGER;
        const bIndex = Number.isFinite(Number(b.pinIndex)) ? Number(b.pinIndex) : Number.MAX_SAFE_INTEGER;
        return aIndex - bIndex || String(a.conversationId || "").localeCompare(String(b.conversationId || ""));
      }
      const aActivity = unarchivedConversationActivityAt(a);
      const bActivity = unarchivedConversationActivityAt(b);
      return bActivity.localeCompare(aActivity) || String(a.conversationId || "").localeCompare(String(b.conversationId || ""));
    });
  }

  return {
    sortUnarchivedConversationOverviewItems,
    unarchivedConversationActivityAt,
  };
}
