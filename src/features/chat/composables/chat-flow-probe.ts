import { invokeTauri } from "../../../services/tauri-api";

/**
 * 聊天流式链路临时探针：把关键节点写进后端 backend.log，便于在生产/发行版里定位
 * 前端处理断点。排查结束后整体移除。
 */
export function probeChatFlow(tag: string, data?: Record<string, unknown>): void {
  let detail = "";
  if (data) {
    try {
      detail = ` ${JSON.stringify(data)}`;
    } catch {
      detail = " [unserializable]";
    }
  }
  // 探针只用于排障，任何环境（含 invoke 未接线的测试/纯浏览器）都不能反过来影响链路
  try {
    void invokeTauri<boolean>("append_runtime_log_probe", {
      message: `[聊天流诊断] ${tag}${detail}`,
    }).catch(() => {});
  } catch {
    // 忽略：诊断探针失败不得影响主链路
  }
}
