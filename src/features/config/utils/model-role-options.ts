export const MODEL_ROLE_EXPERT_API_CONFIG_ID = "role:expert";
export const MODEL_ROLE_QUICK_API_CONFIG_ID = "role:quick";

export function isModelRoleApiConfigId(value: unknown): boolean {
  const normalized = String(value || "").trim();
  return normalized === MODEL_ROLE_EXPERT_API_CONFIG_ID || normalized === MODEL_ROLE_QUICK_API_CONFIG_ID;
}

export function resolveModelRoleApiConfigId(
  value: unknown,
  config: { assistantDepartmentApiConfigId?: string; toolReviewApiConfigId?: string | null },
): string {
  const normalized = String(value || "").trim();
  if (normalized === MODEL_ROLE_EXPERT_API_CONFIG_ID) {
    return String(config.assistantDepartmentApiConfigId || "").trim();
  }
  if (normalized === MODEL_ROLE_QUICK_API_CONFIG_ID) {
    return String(config.toolReviewApiConfigId || "").trim();
  }
  return normalized;
}
