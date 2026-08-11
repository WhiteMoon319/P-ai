import { describe, expect, it } from "vitest";
import { buildConversationSections, type ConversationSectionTitles } from "./conversation-sections";
import type { ChatConversationOverviewItem } from "../../../types/app";

const titles: ConversationSectionTitles = {
  recent: "最近",
  pinned: "置顶",
  other: "其他",
  defaultWorkspace: "默认工作区",
};

function item(overrides: Partial<ChatConversationOverviewItem> & { conversationId: string }): ChatConversationOverviewItem {
  return {
    kind: "local_unarchived",
    title: "",
    lastMessageAt: "",
    updatedAt: "",
    unreadCount: 0,
    isPinned: false,
    ...overrides,
  } as ChatConversationOverviewItem;
}

function ids(sections: ReturnType<typeof buildConversationSections>): string[] {
  return sections.flatMap((section) => section.items.map((entry) => entry.conversationId));
}

describe("buildConversationSections", () => {
  it("置顶会话排在最前，其次最近会话，最后按工作区分组", () => {
    const items = [
      item({ conversationId: "old", lastMessageAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" }),
      item({ conversationId: "new", lastMessageAt: "2026-08-01T00:00:00Z", updatedAt: "2026-08-01T00:00:00Z" }),
      item({ conversationId: "pinned-old", lastMessageAt: "2025-01-01T00:00:00Z", updatedAt: "2025-01-01T00:00:00Z", isPinned: true }),
      item({ conversationId: "ws", lastMessageAt: "2026-07-01T00:00:00Z", updatedAt: "2026-07-01T00:00:00Z", workspaceRootPath: "E:/work/proj" }),
    ];
    const sections = buildConversationSections(items, { tab: "local", titles, locale: "zh-CN" });
    const order = ids(sections);
    expect(order[0]).toBe("pinned-old");
    expect(order.slice(1)).toContain("new");
    expect(order.slice(1)).toContain("old");
    expect(order).toContain("ws");
    expect(sections.find((section) => section.key === "pinned")?.items.map((entry) => entry.conversationId)).toEqual(["pinned-old"]);
    expect(sections.find((section) => section.key.startsWith("workspace:"))?.items.map((entry) => entry.conversationId)).toEqual(["ws"]);
  });

  it("contact 标签下只显示远程联系人会话", () => {
    const items = [
      item({ conversationId: "local", lastMessageAt: "2026-08-01T00:00:00Z", updatedAt: "2026-08-01T00:00:00Z" }),
      item({ conversationId: "remote", kind: "remote_im_contact", lastMessageAt: "2026-08-02T00:00:00Z", updatedAt: "2026-08-02T00:00:00Z", channelName: "频道A" }),
    ];
    const sections = buildConversationSections(items, { tab: "contact", titles, locale: "zh-CN" });
    expect(ids(sections)).not.toContain("local");
    expect(ids(sections)).toContain("remote");
  });

  it("非 contact 标签下排除远程联系人会话", () => {
    const items = [
      item({ conversationId: "local", lastMessageAt: "2026-08-01T00:00:00Z", updatedAt: "2026-08-01T00:00:00Z" }),
      item({ conversationId: "remote", kind: "remote_im_contact", lastMessageAt: "2026-08-02T00:00:00Z", updatedAt: "2026-08-02T00:00:00Z" }),
    ];
    const sections = buildConversationSections(items, { tab: "local", titles, locale: "zh-CN" });
    expect(ids(sections)).not.toContain("remote");
    expect(ids(sections)).toContain("local");
  });

  it("空列表不产生任何分区", () => {
    const sections = buildConversationSections([], { tab: "local", titles, locale: "zh-CN" });
    expect(sections).toEqual([]);
  });
});
