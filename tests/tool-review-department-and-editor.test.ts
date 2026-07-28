import { describe, expect, it } from "vitest";
import type { DepartmentConfig } from "../src/types/app";
import { resolveRetryToolReviewDepartmentId } from "../src/features/chat/utils/tool-review-department";
import {
  buildDepartmentBasicSnapshot,
  mergeDepartmentChildIdsFromSource,
} from "../src/features/config/utils/department-basic-editor";

function createDepartment(overrides: Partial<DepartmentConfig> = {}): DepartmentConfig {
  return {
    id: "department-a",
    name: "Department A",
    summary: "",
    guide: "",
    apiConfigId: "",
    apiConfigIds: [],
    agentIds: [],
    childDepartmentIds: [],
    createdAt: "2026-05-05T00:00:00.000Z",
    updatedAt: "2026-05-05T00:00:00.000Z",
    orderIndex: 1,
    isBuiltInAssistant: false,
    source: "main_config",
    scope: "global",
    permissionControl: {
      enabled: false,
      mode: "blacklist",
      builtinToolNames: [],
      skillNames: [],
      mcpToolNames: [],
    },
    ...overrides,
  };
}

describe("tool review retry department", () => {
  it("prefers the stored review department when it is still allowed", () => {
    expect(
      resolveRetryToolReviewDepartmentId({
        reportDepartmentId: "child-b",
        currentDepartmentId: "assistant-department",
        departmentOptions: [{ id: "child-a" }, { id: "child-b" }],
      }),
    ).toBe("child-b");
  });

  it("falls back to the first allowed child when stored and current departments are unavailable", () => {
    expect(
      resolveRetryToolReviewDepartmentId({
        reportDepartmentId: "assistant-department",
        currentDepartmentId: "assistant-department",
        departmentOptions: [{ id: "child-a" }, { id: "child-b" }],
      }),
    ).toBe("child-a");
  });
});

describe("department basic editor helpers", () => {
  it("ignores child department edges in the basic editor snapshot", () => {
    const left = [createDepartment({ childDepartmentIds: ["child-a"] })];
    const right = [createDepartment({ childDepartmentIds: ["child-b"] })];

    expect(buildDepartmentBasicSnapshot(left)).toBe(buildDepartmentBasicSnapshot(right));
  });

  it("preserves saved child edges for existing departments while keeping new drafts intact", () => {
    const drafts = [
      createDepartment({ id: "department-a", childDepartmentIds: ["stale-child"] }),
      createDepartment({ id: "department-new", childDepartmentIds: ["department-new", "child-new", "child-new"] }),
    ];
    const source = [
      createDepartment({ id: "department-a", childDepartmentIds: ["child-a", "child-b"] }),
    ];

    expect(mergeDepartmentChildIdsFromSource(drafts, source)).toEqual([
      createDepartment({ id: "department-a", childDepartmentIds: ["child-a", "child-b"] }),
      createDepartment({ id: "department-new", childDepartmentIds: ["child-new"] }),
    ]);
  });

  it("prunes saved child edges that point to a deleted department", () => {
    const drafts = [createDepartment({ id: "department-a" })];
    const source = [createDepartment({ id: "department-a", childDepartmentIds: ["department-removed"] })];

    expect(
      mergeDepartmentChildIdsFromSource(drafts, source, ["department-removed"]),
    ).toEqual([
      createDepartment({ id: "department-a", childDepartmentIds: [] }),
    ]);
  });
});
