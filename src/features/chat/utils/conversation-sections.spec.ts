import { describe, expect, it } from "vitest";
import { applyConversationSectionOrder, type ConversationSection } from "./conversation-sections";

describe("applyConversationSectionOrder", () => {
  it("should append new sections to the end of the saved order", () => {
    const sections = [
      { key: "workspace:b", title: "B", items: [] },
      { key: "workspace:c", title: "C", items: [] },
      { key: "workspace:a", title: "A", items: [] },
    ] satisfies ConversationSection[];

    const result = applyConversationSectionOrder(sections, ["workspace:a", "workspace:b"]);

    expect(result.sections.map((section) => section.key)).toEqual([
      "workspace:a",
      "workspace:b",
      "workspace:c",
    ]);
    expect(result.nextOrder).toEqual([
      "workspace:a",
      "workspace:b",
      "workspace:c",
    ]);
    expect(result.changed).toBe(true);
  });

  it("should ignore missing saved keys and preserve existing stable order", () => {
    const sections = [
      { key: "workspace:b", title: "B", items: [] },
      { key: "workspace:a", title: "A", items: [] },
    ] satisfies ConversationSection[];

    const result = applyConversationSectionOrder(sections, ["workspace:gone", "workspace:a", "workspace:b"]);

    expect(result.sections.map((section) => section.key)).toEqual([
      "workspace:a",
      "workspace:b",
    ]);
    expect(result.nextOrder).toEqual([
      "workspace:a",
      "workspace:b",
    ]);
    expect(result.changed).toBe(true);
  });
});
