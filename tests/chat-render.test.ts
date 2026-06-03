import { describe, expect, it } from "vitest";
import type { ChatMessageBlock } from "../src/types/app";
import { blockSizeDependencies } from "../src/features/chat/utils/chat-render";
import { textContentSignature } from "../src/features/chat/utils/text-signature";

function assistantBlock(overrides: Partial<ChatMessageBlock> = {}): ChatMessageBlock {
  return {
    id: "assistant-block",
    role: "assistant",
    text: "",
    images: [],
    audios: [],
    attachmentFiles: [],
    toolCallCount: 0,
    lastToolName: "",
    toolCalls: [],
    activityItems: [],
    activityReasoningCharCount: 0,
    activityToolCountsByName: {},
    activityRunning: false,
    activityStatus: "idle",
    ...overrides,
  };
}

describe("chat render signatures", () => {
  it("changes text signatures for equal-length content corrections", () => {
    expect("result-old").toHaveLength("result-new".length);
    expect(textContentSignature("result-old")).not.toBe(textContentSignature("result-new"));
  });

  it("changes block size dependencies when tool result text changes without growing", () => {
    const first = blockSizeDependencies(assistantBlock({
      activityItems: [{
        kind: "tool",
        id: "tool-1",
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\"}",
        resultText: "result-old",
        status: "done",
      }],
      activityToolCountsByName: { operate: 1 },
      activityStatus: "complete",
    }));
    const second = blockSizeDependencies(assistantBlock({
      activityItems: [{
        kind: "tool",
        id: "tool-1",
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\"}",
        resultText: "result-new",
        status: "done",
      }],
      activityToolCountsByName: { operate: 1 },
      activityStatus: "complete",
    }));

    expect(first).not.toEqual(second);
  });
});
