/**
 * 设备控制（Shizuku/root 提权）状态派生逻辑——纯函数，便于 vitest 单测。
 *
 * ToolsTab 设备控制卡片直接复用本模块；UI 文案 key 由组件侧 `t()` 渲染。
 */

export type DeviceControlPrivilegeState =
  | "disabled"
  | "shizuku_pending"
  | "shizuku_ready"
  | "root_ready";

export type DeviceControlStatus = {
  shizukuAvailable: boolean;
  shizukuGranted: boolean;
  rootAvailable: boolean;
  privilegeState: DeviceControlPrivilegeState;
};

/** 提权状态 → 状态徽章 label 的 i18n key。 */
export function deviceControlStateLabelKey(
  state: DeviceControlPrivilegeState,
): string {
  switch (state) {
    case "shizuku_ready":
      return "config.tools.deviceControlStateShizukuReady";
    case "root_ready":
      return "config.tools.deviceControlStateRootReady";
    case "shizuku_pending":
      return "config.tools.deviceControlStateShizukuPending";
    default:
      return "config.tools.deviceControlStateDisabled";
  }
}

/** 提权状态 → 徽章样式类。 */
export function deviceControlStateBadgeClass(
  state: DeviceControlPrivilegeState,
): string {
  if (state === "shizuku_ready" || state === "root_ready") return "badge-success";
  if (state === "shizuku_pending") return "badge-info";
  return "badge-warning";
}

/** 状态详情三行文本（Shizuku 可用/已授权/root 可用）。t 由调用方注入以保持纯函数。 */
export function deviceControlStatusLines(
  status: DeviceControlStatus,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string[] {
  const shizuku = t("config.tools.deviceControlDetailShizuku", {
    ok: status.shizukuAvailable ? "✓" : "✗",
  });
  const granted = t("config.tools.deviceControlDetailGranted", {
    ok: status.shizukuGranted ? "✓" : "✗",
  });
  const root = t("config.tools.deviceControlDetailRoot", {
    ok: status.rootAvailable ? "✓" : "✗",
  });
  return [shizuku, granted, root];
}