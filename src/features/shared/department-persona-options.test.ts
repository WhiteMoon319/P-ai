import { describe, expect, it } from "vitest";
import type { ApiConfigItem, DepartmentConfig, PersonaProfile } from "../../types/app";
import { buildDepartmentPersonaOptions } from "./department-persona-options";

const departments = [
  {
    id: "research",
    name: "研究部",
    agentIds: ["alice"],
    apiConfigId: "role:expert",
  },
  {
    id: "ops",
    name: "运维部",
    agentIds: ["bob"],
    apiConfigId: "model-ops",
  },
] as DepartmentConfig[];

const personas = [
  { id: "alice", name: "爱丽丝", isBuiltInUser: false },
  { id: "bob", name: "鲍勃", isBuiltInUser: false },
] as PersonaProfile[];

const apiConfigs = [
  { id: "model-ops", name: "运维模型", displayName: "ops-1", model: "ops-1", enableText: true },
] as ApiConfigItem[];

describe("buildDepartmentPersonaOptions", () => {
  it("keeps department options when its model is missing, marked as modelMissing", () => {
    const options = buildDepartmentPersonaOptions({
      departments,
      personas,
      apiConfigs,
      assistantDepartmentApiConfigId: "model-gone",
    });
    expect(options).toHaveLength(2);
    const research = options.find((option) => option.departmentId === "research");
    expect(research).toBeDefined();
    expect(research?.modelMissing).toBe(true);
    expect(research?.modelName).toBeUndefined();
    expect(research?.providerName).toBeUndefined();
    const ops = options.find((option) => option.departmentId === "ops");
    expect(ops?.modelMissing).toBe(false);
    expect(ops?.modelName).toBe("ops-1");
  });

  it("keeps department options when role:quick / role:expert resolves to a missing model", () => {
    const options = buildDepartmentPersonaOptions({
      departments,
      personas,
      apiConfigs,
      toolReviewApiConfigId: "model-gone-too",
    });
    const research = options.find((option) => option.departmentId === "research");
    expect(research?.modelMissing).toBe(true);
  });

  it("keeps empty departments visible as personaMissing placeholders", () => {
    const emptyDepartment = { id: "ghost", name: "幽灵部", agentIds: [] as string[], apiConfigId: "model-ops" } as unknown as DepartmentConfig;
    const options = buildDepartmentPersonaOptions({
      departments: [emptyDepartment],
      personas,
      apiConfigs,
    });
    expect(options).toHaveLength(1);
    const ghost = options[0];
    expect(ghost.departmentId).toBe("ghost");
    expect(ghost.personaMissing).toBe(true);
  });

  it("keeps departments with dangling agentIds as personaMissing options", () => {
    const danglingDepartment = { id: "ghost", name: "幽灵部", agentIds: ["no-such-persona"], apiConfigId: "model-ops" } as unknown as DepartmentConfig;
    const options = buildDepartmentPersonaOptions({
      departments: [danglingDepartment],
      personas,
      apiConfigs,
    });
    expect(options).toHaveLength(1);
    const ghost = options[0];
    expect(ghost.personaMissing).toBe(true);
    expect(ghost.agentId).toBe("no-such-persona");
  });
});
