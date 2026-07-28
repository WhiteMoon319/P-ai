import { describe, expect, it, vi } from "vitest";
import { coordinateConversationDelete } from "./conversation-delete-coordinator";

describe("coordinateConversationDelete", () => {
  it("两端共用后端 activeConversationId，并只在删除成功后清理当前投影", async () => {
    let currentConversationId = "conversation-a";
    const calls: string[] = [];
    await coordinateConversationDelete({
      conversationId: "conversation-a",
      currentConversationId: () => currentConversationId,
      deleteConversation: async () => {
        calls.push("delete");
        return {
          activeConversationId: "conversation-b",
          unarchivedConversations: [{ conversationId: "conversation-b" }],
        };
      },
      applyConversationList: () => calls.push("apply-list"),
      conversationIds: () => ["conversation-b"],
      clearCurrentConversation: () => {
        calls.push("clear");
        currentConversationId = "";
      },
      openConversation: async (conversationId) => {
        calls.push(`open:${conversationId}`);
      },
    });

    expect(calls).toEqual(["delete", "apply-list", "clear", "open:conversation-b"]);
  });

  it("删除非当前会话时不触碰前台投影", async () => {
    const clear = vi.fn();
    const open = vi.fn();
    await coordinateConversationDelete({
      conversationId: "conversation-b",
      currentConversationId: () => "conversation-a",
      deleteConversation: async () => ({ unarchivedConversations: [] }),
      conversationIds: () => [],
      clearCurrentConversation: clear,
      openConversation: open,
    });

    expect(clear).not.toHaveBeenCalled();
    expect(open).not.toHaveBeenCalled();
  });
});
