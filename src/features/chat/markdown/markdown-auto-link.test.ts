import { describe, expect, it } from "vitest";
import { parseInlineSegments } from "./parse-markdown";
import { findNextMarkdownAutoLink } from "./markdown-auto-link";

describe("Markdown 自动链接", () => {
  it("识别尖括号包裹的 URL、file URI 与 Windows 文件路径", () => {
    const input = "<https://example.com/docs?q=1> <file:///E:/My%20Project/readme.md> <E:/My Project/readme.md:12>";

    expect(parseInlineSegments(input)).toEqual([
      { type: "link", text: "https://example.com/docs?q=1", href: "https://example.com/docs?q=1" },
      { type: "text", text: " " },
      { type: "link", text: "file:///E:/My%20Project/readme.md", href: "file:///E:/My%20Project/readme.md" },
      { type: "text", text: " " },
      { type: "link", text: "E:/My Project/readme.md:12", href: "E:/My Project/readme.md:12" },
    ]);
  });

  it("不会将尖括号 URL 后的空格正文吞入链接", () => {
    expect(findNextMarkdownAutoLink("<https://example.com 后文>", 0)).toBeNull();
  });

  it("保留裸 URL 的既有自动链接行为，并允许尖括号目标包含右括号", () => {
    expect(findNextMarkdownAutoLink("见 https://example.com/a(b)，以及 <https://example.com/a(b)>", 0)).toEqual({
      start: 2,
      end: 23,
      href: "https://example.com/a",
    });
    expect(findNextMarkdownAutoLink("见 https://example.com/a(b)，以及 <https://example.com/a(b)>", 23)).toEqual({
      start: 30,
      end: 56,
      href: "https://example.com/a(b)",
    });
  });
});
