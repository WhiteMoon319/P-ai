import { describe, expect, it } from "vitest";
import { extractToolcallFilePath } from "./toolcall-preview";

describe("extractToolcallFilePath", () => {
  it("reads path from read args", () => {
    expect(extractToolcallFilePath("read", JSON.stringify({
      path: "E:\\\\github\\\\easy_call_ai\\\\src\\\\a.ts",
      offset: 150,
    }))).toMatch(/easy_call_ai[\\/]+src[\\/]+a\.ts$/i);
  });

  it("reads absolute_path variants", () => {
    expect(extractToolcallFilePath("update", JSON.stringify({
      absolute_path: "E:/repo/file.ts",
    }))).toBe("E:/repo/file.ts");
  });

  it("ignores exec command text", () => {
    expect(extractToolcallFilePath("exec", JSON.stringify({
      command: "pnpm test src/features/chat/markdown/a.ts",
    }))).toBe("");
  });

  it("accepts bare path string args", () => {
    expect(extractToolcallFilePath("write", "E:/repo/x.ts")).toBe("E:/repo/x.ts");
  });
});
