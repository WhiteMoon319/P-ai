import { describe, expect, it } from "vitest";
import { consumeClosedMarkdownBlocks } from "../src/features/chat/composables/use-chat-flow-text";

describe("use-chat-flow text streaming", () => {
  it("preserves whitespace when consuming closed markdown blocks", () => {
    const input = "不对。\n准确说法应该是：\n\n- **优先级本来就有**\n- 现在要做的是...\n";

    expect(consumeClosedMarkdownBlocks(input)).toEqual({
      chunks: ["不对。\n准确说法应该是：\n\n"],
      tail: "- **优先级本来就有**\n- 现在要做的是...\n",
    });
  });
});
