import { describe, expect, it } from "vitest";
import { IncrementalMarkdownBlockParser } from "./incremental-markdown";
import { parseInlineSegments, parseMarkdownBlocks, type InlineSegment, type MarkdownBlock } from "./parse-markdown";

function stripKeys(blocks: MarkdownBlock[]): unknown[] {
  return blocks.map((block) => {
    const { key: _key, ...rest } = block;
    return rest;
  });
}

describe("IncrementalMarkdownBlockParser", () => {
  it("matches full streaming parse after append-only chunks", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = [
      "# 标题\n\n",
      "第一段带 [toolcall:call-1]。\n\n",
      "- 项目 A\n",
      "- 项目 B\n\n",
      "```ts\n",
      "console.log('hi')\n",
      "```\n\n",
      "| A | B |\n",
      "|---|---|\n",
      "| 1 | 2 |\n\n",
      "结束。",
    ];

    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("resets when the input is replaced instead of appended", () => {
    const parser = new IncrementalMarkdownBlockParser();
    parser.parse("旧内容\n\n- A");

    const text = "新内容\n\n- B";
    expect(stripKeys(parser.parse(text))).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("keeps streaming footnotes at the end", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = ["脚注引用[^a]", "\n\n继续正文\n\n", "[^a]: 说明"];
    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(actual[actual.length - 1]?.type).toBe("footnotes");
    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });
});

describe("parseInlineSegments", () => {
  it("renders strong text across inline code spans", () => {
    const segments = parseInlineSegments("**Issue 1: `heading_h1` 一直为空**");

    expect(segments).toEqual<InlineSegment[]>([
      {
        type: "strong",
        children: [
          { type: "text", text: "Issue 1: " },
          { type: "code", text: "heading_h1" },
          { type: "text", text: " 一直为空" },
        ],
      },
    ]);
  });

  it("keeps emphasis markers inside inline code literal", () => {
    const segments = parseInlineSegments("`**not strong**`");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "code", text: "**not strong**" },
    ]);
  });

  it("supports whitelisted inline html tags", () => {
    const segments = parseInlineSegments("按 <kbd>Ctrl</kbd>+<kbd>K</kbd><br>H<sub>2</sub>O 与 x<sup>2</sup>，<mark>重点</mark>");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "按 " },
      { type: "html_kbd", children: [{ type: "text", text: "Ctrl" }] },
      { type: "text", text: "+" },
      { type: "html_kbd", children: [{ type: "text", text: "K" }] },
      { type: "html_br" },
      { type: "text", text: "H" },
      { type: "html_sub", children: [{ type: "text", text: "2" }] },
      { type: "text", text: "O 与 x" },
      { type: "html_sup", children: [{ type: "text", text: "2" }] },
      { type: "text", text: "，" },
      { type: "html_mark", children: [{ type: "text", text: "重点" }] },
    ]);
  });
});

describe("parseMarkdownBlocks", () => {
  it("supports whitelisted details blocks", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "<details open>",
      "<summary>展开看 <mark>说明</mark></summary>",
      "",
      "正文第一行",
      "",
      "- 项目 A",
      "</details>",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "details",
        summary: "展开看 <mark>说明</mark>",
        body: "正文第一行\n\n- 项目 A",
        open: true,
      },
    ]);
  });
});
