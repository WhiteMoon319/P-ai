import { describe, expect, it } from "vitest";
import { normalizeMarkdownImageCacheKey, resolveMarkdownImageSource } from "./MarkdownImage";

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
  });

  it("对 Windows 缓存键统一分隔符和盘符大小写，保留其余路径大小写", () => {
    expect(normalizeMarkdownImageCacheKey("E:\\Repo\\Images\\A.PNG"))
      .toBe("e:/Repo/Images/A.PNG");
  });
});
