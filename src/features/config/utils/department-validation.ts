import type { ApiConfigItem, AppConfig } from "../../../types/app";
import { findDepartmentGraphCycle, normalizeDepartmentChildIds } from "./department-graph";
import { isModelRoleApiConfigId, resolveModelRoleApiConfigId } from "./model-role-options";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

const TEXT_REQUEST_FORMATS = new Set([
  "auto",
  "openai",
  "deepseek",
  "openai_responses",
  "codex",
  "gemini",
  "anthropic",
  "fireworks",
  "together",
  "groq",
  "mimo",
  "moonshot",
  "nebius",
  "xai",
  "zai",
  "bigmodel",
  "aliyun",
  "baidu",
  "cohere",
  "ollama",
  "ollama_cloud",
  "vertex",
  "github_copilot",
  "opencode_go",
  "bedrock_api",
]);

function textCapableApiIds(apiConfigs: ApiConfigItem[]): Set<string> {
  return new Set(
    (apiConfigs || [])
      .filter((api) => {
        const format = String(api.requestFormat || "").trim().toLowerCase();
        return !!api.enableText && (format === "deepseek/kimi" || TEXT_REQUEST_FORMATS.has(format));
      })
      .map((api) => api.id),
  );
}

export function validateDepartmentConfig(
  config: AppConfig,
  apiConfigs: ApiConfigItem[],
  t: TrFn,
): string {
  const departments = config.departments || [];
  const validApiIds = textCapableApiIds(apiConfigs);

  for (const department of departments) {
    const name = String(department.name || "").trim();
    if (!name) {
      return t("config.department.validation.emptyName");
    }
  }

  const seenNames = new Set<string>();
  for (const department of departments) {
    const nameKey = String(department.name || "").trim().toLocaleLowerCase();
    if (!nameKey) continue;
    if (seenNames.has(nameKey)) {
      return t("config.department.validation.duplicateName");
    }
    seenNames.add(nameKey);
  }

  for (const department of departments) {
    const ids = Array.isArray(department.apiConfigIds) && department.apiConfigIds.length > 0
      ? department.apiConfigIds
      : (department.apiConfigId ? [department.apiConfigId] : []);
    for (const id of ids.map((item) => String(item || "").trim()).filter(Boolean)) {
      const resolvedId = resolveModelRoleApiConfigId(id, config);
      if (!isModelRoleApiConfigId(id) && !validApiIds.has(resolvedId)) {
        return t("config.department.validation.invalidModel", { name: department.name || department.id });
      }
      if (isModelRoleApiConfigId(id) && (!resolvedId || !validApiIds.has(resolvedId))) {
        return t("config.department.validation.invalidModel", { name: department.name || department.id });
      }
    }
  }

  if (!departments.find((item) => item.id === "assistant-department" || item.isBuiltInAssistant)) {
    return t("config.department.validation.missingAssistant");
  }

  const existingIds = new Set(departments.map((item) => String(item.id || "").trim()).filter(Boolean));
  for (const department of departments) {
    const departmentId = String(department.id || "").trim();
    for (const childId of normalizeDepartmentChildIds(department.childDepartmentIds, departmentId)) {
      if (!existingIds.has(childId)) {
        return t("config.department.validation.invalidChildDepartment", {
          name: department.name || department.id,
        });
      }
    }
  }

  const cycle = findDepartmentGraphCycle(departments);
  if (cycle) {
    return t("config.department.validation.departmentRelationCycle", {
      path: cycle.join(" -> "),
    });
  }

  return "";
}
