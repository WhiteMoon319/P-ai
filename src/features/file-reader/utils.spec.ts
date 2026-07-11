import { describe, expect, it } from "vitest";
import { fileKindFromPath, isUnsupportedFileExtension } from "./utils";

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
