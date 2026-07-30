import { describe, expect, it } from "vitest";
import source from "./main-sidebar.ts?raw";

describe("sidebar chat entry", () => {
  it("只复用 main-chat，不维护 Web 专用聊天运行时", () => {
    expect(source).toMatch(/import\s+["']\.\.\/\.\.\/main-chat["'];/);
    expect(source).not.toMatch(/useChatFlow|useChatForegroundRuntime|useConversationViewRuntime/);
  });
});
