import { describe, expect, it } from "vitest";
import { commonPrefixLength, mergeAssistantText } from "./use-chat-flow-text";

describe("mergeAssistantText", () => {
  it("appends only the final tail when final extends current", () => {
    expect(mergeAssistantText("hello", "hello world")).toBe("hello world");
  });

  it("keeps current when it already contains final", () => {
    expect(mergeAssistantText("hello world", "hello")).toBe("hello world");
  });

  it("keeps current when middle diverges instead of full rewrite", () => {
    expect(mergeAssistantText("hello world", "hello there")).toBe("hello world");
  });

  it("rejects a final snapshot that changes an already rendered block boundary", () => {
    expect(mergeAssistantText("第一段\n\n第二段", "第一段\n第二段尾部")).toBe("第一段\n\n第二段");
  });

  it("returns final when current is empty", () => {
    expect(mergeAssistantText("", "final")).toBe("final");
  });

  it("returns current when final is empty", () => {
    expect(mergeAssistantText("current", "")).toBe("current");
  });
});

describe("commonPrefixLength", () => {
  it("counts shared prefix characters", () => {
    expect(commonPrefixLength("abcdef", "abcxyz")).toBe(3);
    expect(commonPrefixLength("same", "same")).toBe(4);
    expect(commonPrefixLength("", "x")).toBe(0);
  });
});
