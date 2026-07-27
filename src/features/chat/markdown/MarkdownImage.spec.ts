import { describe, expect, it } from "vitest";
import { normalizeMarkdownImageCacheKey, resolveMarkdownImageSource } from "./MarkdownImage";
import { parseInlineSegments } from "./parse-markdown";

describe("MarkdownImage", () => {
  it("区分远程图片、本地图片和被禁止的 scheme", () => {
    expect(resolveMarkdownImageSource("https://example.com/a.png", "")).toEqual({
      kind: "remote",
      src: "https://example.com/a.png",
    });
    expect(resolveMarkdownImageSource("javascript:alert(1)", "")).toEqual({
      kind: "blocked",
      label: "javascript:alert(1)",
    });
    expect(resolveMarkdownImageSource("images/a.png", "E:/repo/docs")).toEqual({
      kind: "local",
      path: "E:/repo/docs/images/a.png",
    });
    expect(resolveMarkdownImageSource(
      "{Assistant Space}\\generated-images\\20260727\\a.png",
      "E:/repo/docs",
    )).toEqual({
      kind: "local",
      path: "{Assistant Space}/generated-images/20260727/a.png",
    });
  });

  it("对 Windows 缓存键统一分隔符和盘符大小写，保留其余路径大小写", () => {
    expect(normalizeMarkdownImageCacheKey("E:\\Repo\\Images\\A.PNG"))
      .toBe("e:/Repo/Images/A.PNG");
  });

  it("应把 Assistant Space 稳定引用解析为 Markdown 图片", () => {
    expect(parseInlineSegments(
      "![生成图片]({Assistant Space}/generated-images/20260727/a.png)",
    )).toEqual([{
      type: "image",
      alt: "生成图片",
      src: "{Assistant Space}/generated-images/20260727/a.png",
    }]);
  });
});
