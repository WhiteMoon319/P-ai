import { describe, expect, it } from "vitest";
import { isSameRepoPath } from "./use-workspace-git-status";

describe("isSameRepoPath", () => {
  it("斜杠方向与盘符大小写差异视为同一仓库", () => {
    expect(isSameRepoPath("E:\\github\\easy_call_ai", "e:/github/easy_call_ai")).toBe(true);
  });

  it("尾斜杠不影响比较", () => {
    expect(isSameRepoPath("E:/repo/", "E:/repo")).toBe(true);
  });

  it("不同仓库不视为同一个", () => {
    expect(isSameRepoPath("E:/repo-a", "E:/repo-b")).toBe(false);
  });

  it("空路径不与任何路径（含空路径）视为同一个", () => {
    expect(isSameRepoPath("", "")).toBe(false);
    expect(isSameRepoPath("", "E:/repo")).toBe(false);
    expect(isSameRepoPath("E:/repo", "")).toBe(false);
  });
});
