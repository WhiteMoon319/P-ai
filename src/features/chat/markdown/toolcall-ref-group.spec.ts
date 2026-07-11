import { describe, expect, it } from "vitest";
import { parseInlineSegments, parseMarkdownBlocks } from "./parse-markdown";
import {
  consumeCrossParagraphToolGroup,
  consumeGroupedMarkerOnlyParagraphs,
  consumeGroupedToolcallRefs,
  isMarkerOnlyParagraph,
  splitParagraphToolLayout,
} from "./toolcall-ref-group";

describe("consumeGroupedToolcallRefs", () => {
  it("merges directly adjacent toolcall refs without whitespace", () => {
    const segments = parseInlineSegments("[toolcall:a][toolcall:b][toolcall:c]");
    const grouped = consumeGroupedToolcallRefs(segments, 0);
    expect(grouped).toEqual({
      ids: ["a", "b", "c"],
      endIndex: 2,
    });
  });

  it("merges toolcall refs separated by whitespace only", () => {
    const segments = parseInlineSegments("[toolcall:a] [toolcall:b]\n[toolcall:c]");
    const grouped = consumeGroupedToolcallRefs(segments, 0);
    expect(grouped?.ids).toEqual(["a", "b", "c"]);
  });

  it("stops grouping at non-whitespace text boundary", () => {
    const segments = parseInlineSegments("[toolcall:a][toolcall:b] 正文 [toolcall:c]");
    const first = consumeGroupedToolcallRefs(segments, 0);
    expect(first?.ids).toEqual(["a", "b"]);

    const bodyIndex = segments.findIndex((segment) => segment.type === "text" && segment.text.includes("正文"));
    expect(bodyIndex).toBeGreaterThan(0);
    const nextToolIndex = segments.findIndex((segment, index) => index > bodyIndex && segment.type === "toolcall_ref");
    const second = consumeGroupedToolcallRefs(segments, nextToolIndex);
    expect(second?.ids).toEqual(["c"]);
  });

  it("preserves original toolcall id order for preview", () => {
    const segments = parseInlineSegments("[toolcall:call-3][toolcall:call-1] [toolcall:call-2]");
    const grouped = consumeGroupedToolcallRefs(segments, 0);
    expect(grouped?.ids).toEqual(["call-3", "call-1", "call-2"]);
  });
});

describe("consumeGroupedMarkerOnlyParagraphs", () => {
  it("merges consecutive marker-only paragraphs into one group", () => {
    const blocks = parseMarkdownBlocks([
      "[toolcall:a][toolcall:b]",
      "",
      "[toolcall:c]",
      "",
      "后面正文",
    ].join("\n"));

    expect(isMarkerOnlyParagraph(blocks[0])).toBe(true);
    expect(isMarkerOnlyParagraph(blocks[1])).toBe(true);

    const grouped = consumeGroupedMarkerOnlyParagraphs(blocks, 0);
    expect(grouped?.ids).toEqual(["a", "b", "c"]);
    expect(grouped?.endIndex).toBe(1);
  });

  it("does not merge when paragraph contains body text", () => {
    const blocks = parseMarkdownBlocks("前文 [toolcall:a]\n\n[toolcall:b]");
    expect(isMarkerOnlyParagraph(blocks[0])).toBe(false);
    expect(consumeGroupedMarkerOnlyParagraphs(blocks, 0)).toBeNull();
    expect(consumeGroupedMarkerOnlyParagraphs(blocks, 1)?.ids).toEqual(["b"]);
  });
});

describe("consumeCrossParagraphToolGroup", () => {
  it("merges trailing tools with following marker-only paragraphs across blank lines", () => {
    const blocks = parseMarkdownBlocks("前文 [toolcall:a][toolcall:b]\n\n[toolcall:c]\n\n[toolcall:d]\n\n后文");
    const grouped = consumeCrossParagraphToolGroup(blocks, 0);
    expect(grouped?.mode).toBe("trailing");
    expect(grouped?.ids).toEqual(["a", "b", "c", "d"]);
    expect(grouped?.endIndex).toBe(2);
  });

  it("merges trailing tools with next paragraph leading tools across blank lines", () => {
    const blocks = parseMarkdownBlocks("前文 [toolcall:a]\n\n[toolcall:b] 后文");
    const grouped = consumeCrossParagraphToolGroup(blocks, 0);
    expect(grouped?.mode).toBe("trailing");
    expect(grouped?.ids).toEqual(["a", "b"]);
    expect(grouped?.stripLeadingOnEnd).toBe(true);
    expect(grouped?.endBodySegments.some((segment) => segment.type === "text" && segment.text.includes("后文"))).toBe(true);
  });

  it("merges single-newline marker paragraphs that stay in one block", () => {
    const segments = parseInlineSegments("[toolcall:a]\n[toolcall:b]");
    const layout = splitParagraphToolLayout(segments);
    expect(layout.markerOnly).toBe(true);
    expect(layout.allIds).toEqual(["a", "b"]);
  });

  it("does not merge across body text paragraphs", () => {
    const blocks = parseMarkdownBlocks("前文 [toolcall:a]\n\n中间正文\n\n[toolcall:b]");
    const grouped = consumeCrossParagraphToolGroup(blocks, 0);
    expect(grouped).toBeNull();
  });
});
