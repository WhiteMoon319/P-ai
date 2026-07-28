import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const messageTextSize = "font-size: var(--app-chat-message-text-size, var(--app-text-sm-size));";

function source(relativePath: string): string {
  return readFileSync(resolve(process.cwd(), relativePath), "utf8");
}

function expectRuleUsesMessageTextSize(relativePath: string, selector: string) {
  const text = source(relativePath);
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const declarations = text.match(new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`))?.[1];
  expect(declarations, `无法读取 ${relativePath} 中的 ${selector} 规则`).toBeTruthy();
  expect(declarations).toContain(messageTextSize);
}

describe("聊天消息字号边界", () => {
  it("用户、助手和所有 Markdown 路径只使用统一消息字号变量", () => {
    expectRuleUsesMessageTextSize(
      "src/features/chat/components/ChatBubbleShell.vue",
      ".ecall-chat-bubble-surface",
    );
    expectRuleUsesMessageTextSize(
      "src/features/chat/components/ChatMessageItem.vue",
      ".ecall-assistant-bubble",
    );
    expectRuleUsesMessageTextSize(
      "src/features/chat/components/ChatMessageItem.vue",
      ".assistant-markdown :deep(.ecall-markdown-content)",
    );
    expectRuleUsesMessageTextSize(
      "src/features/chat/components/PlainMarkdownRenderer.vue",
      ".ecall-plain-markdown-markdown",
    );
    expectRuleUsesMessageTextSize(
      "src/features/chat/markdown/AppMarkdownRenderer.vue",
      ".ecall-md-chat",
    );
  });
});
