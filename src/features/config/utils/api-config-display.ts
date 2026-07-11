import type { ApiConfigItem } from "../../../types/app";

export const LEGAL_REASONING_EFFORTS = [
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
] as const;

export type LegalReasoningEffort = (typeof LEGAL_REASONING_EFFORTS)[number];

type TranslateFn = (key: string, params?: Record<string, unknown>) => string;

const REASONING_SUFFIX_PATTERN = /\s*·\s*(不思考|低|中|高|极高|Off|Low|Medium|High|Extra High|XHigh)$/i;

export function normalizeReasoningEffortValue(value: unknown): string {
  return String(value || "").trim().toLowerCase();
}

export function isLegalReasoningEffort(value: unknown): value is LegalReasoningEffort {
  const normalized = normalizeReasoningEffortValue(value);
  return (LEGAL_REASONING_EFFORTS as readonly string[]).includes(normalized);
}

/** 仅合法值返回标签；非法/空值返回空字符串，调用方不应再拼等级后缀。 */
export function reasoningEffortDisplayLabel(
  value: unknown,
  t?: TranslateFn,
): string {
  const normalized = normalizeReasoningEffortValue(value);
  if (normalized === "none" || normalized === "minimal") {
    return t ? t("config.api.reasoningOff") : "不思考";
  }
  if (normalized === "low") {
    return t ? t("config.api.reasoningLow") : "低";
  }
  if (normalized === "medium") {
    return t ? t("config.api.reasoningMedium") : "中";
  }
  if (normalized === "high") {
    return t ? t("config.api.reasoningHigh") : "高";
  }
  if (normalized === "xhigh") {
    return t ? t("config.api.reasoningXHigh") : "极高";
  }
  return "";
}

export function stripReasoningEffortDisplaySuffix(name: string): string {
  return String(name || "").replace(REASONING_SUFFIX_PATTERN, "").trim();
}

export function apiConfigDisplayName(
  providerName: string,
  modelValue: string,
  reasoningEffort: unknown,
  t?: TranslateFn,
): string {
  const provider = String(providerName || "").trim();
  const model = String(modelValue || "").trim();
  const base = provider && model
    ? `${provider}/${model}`
    : (provider || model);
  const label = reasoningEffortDisplayLabel(reasoningEffort, t);
  if (!base) return label;
  return label ? `${base} · ${label}` : base;
}

/** 聊天下拉/按钮优先按 reasoningEffort 现算，避免历史 name 偶发缺后缀。 */
export function formatApiConfigOptionLabel(
  item: Pick<ApiConfigItem, "name" | "model" | "reasoningEffort"> | null | undefined,
  t?: TranslateFn,
): string {
  if (!item) return "";
  const model = String(item.model || "").trim();
  const rawName = String(item.name || "").trim();
  const baseFromName = stripReasoningEffortDisplaySuffix(rawName);
  let base = baseFromName;
  if (model) {
    if (!base) {
      base = model;
    } else if (!base.includes("/") && base !== model) {
      // name 异常时尽量保住 model
      base = model;
    } else if (base.endsWith(`/${model}`) || base === model) {
      // already good
    } else if (!base.includes(model)) {
      // name 与 model 不一致时，以 provider/model 形态优先
      const provider = base.includes("/") ? base.slice(0, base.lastIndexOf("/")) : base;
      base = provider ? `${provider}/${model}` : model;
    }
  }
  if (!base) base = rawName || model;
  const label = reasoningEffortDisplayLabel(item.reasoningEffort, t);
  return label ? `${base} · ${label}` : base;
}
