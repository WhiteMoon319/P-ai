import { describe, expect, it } from "vitest";
import { resolveSideChatSelectionAfterClose } from "./side-chat-tabs";

describe("resolveSideChatSelectionAfterClose", () => {
  const orderedIds = ["left", "active", "right"];

  it("selects the tab on the right when the active tab closes", () => {
    expect(resolveSideChatSelectionAfterClose(orderedIds, "active", ["active"])).toBe("right");
  });

  it("falls back to the tab on the left when no right tab remains", () => {
    expect(resolveSideChatSelectionAfterClose(orderedIds, "right", ["right"])).toBe("active");
  });

  it("keeps the active tab when only inactive tabs close", () => {
    expect(resolveSideChatSelectionAfterClose(orderedIds, "active", ["left", "right"])).toBe("active");
  });

  it("clears the selection when all tabs close", () => {
    expect(resolveSideChatSelectionAfterClose(orderedIds, "active", orderedIds)).toBe("");
  });
});
