import { describe, expect, it } from "vitest";
import { directoryPathChain, fileKindFromPath, isUnsupportedFileExtension } from "./utils";

describe("fileKindFromPath", () => {
  it("treats TypeScript .ts as code, not unsupported media", () => {
    expect(fileKindFromPath("E:/repo/src/toolcall-preview.ts")).toBe("code");
    expect(isUnsupportedFileExtension("ts")).toBe(false);
  });

  it("still rejects explicit mpeg-ts containers", () => {
    expect(fileKindFromPath("E:/video/sample.mts")).toBe("unsupported");
    expect(fileKindFromPath("E:/video/sample.m2ts")).toBe("unsupported");
  });
});

describe("directoryPathChain", () => {
  it("returns the directories from root to the target directory", () => {
    expect(directoryPathChain("E:/repo", "E:/repo/src/features")).toEqual([
      "E:/repo",
      "E:/repo/src",
      "E:/repo/src/features",
    ]);
  });

  it("handles Windows drive roots and case-insensitive matching", () => {
    expect(directoryPathChain("e:/", "E:/Repo/src")).toEqual([
      "e:/",
      "e:/Repo",
      "e:/Repo/src",
    ]);
  });

  it("returns an empty chain for paths outside the root", () => {
    expect(directoryPathChain("E:/repo", "E:/other/src")).toEqual([]);
  });
});
