import { describe, expect, it } from "vitest";
import { buildContactLogDisplayItem } from "./contact-log-display";

const t = (key: string, params?: Record<string, unknown>) => params
  ? `${key}:${JSON.stringify(params)}`
  : key;

describe("buildContactLogDisplayItem", () => {
  it("解析收到的联系人消息与附件计数", () => {
    const item = buildContactLogDisplayItem({
      timestamp: "2026-07-13T00:00:00Z",
      level: "info",
      message: "[联系人消息] 收到: sender=小明, preview=你好, image_count=2, audio_count=0, attachment_count=1",
    }, t);

    expect(item?.summary).toBe("[小明]你好");
    expect(item?.detail).toContain("imageCount");
    expect(item?.detail).toContain("attachmentCount");
  });

  it("忽略成功入队这类内部噪声", () => {
    const item = buildContactLogDisplayItem({ timestamp: "now", level: "info", message: "[联系人消息] 入队: reason=ok" }, t);
    expect(item).toBeNull();
  });

  it("展示按消息头过滤的消息，但不暴露原文", () => {
    const item = buildContactLogDisplayItem({
      timestamp: "now",
      level: "info",
      message: "[联系人消息] 过滤跳过: contact=小明, prefix=#, text_len=20",
    }, t);

    expect(item?.kind).toBe("config.remoteIm.logKindFilter");
    expect(item?.summary).toContain("#");
  });

  it("保留未知异常日志的通用展示", () => {
    const item = buildContactLogDisplayItem({ timestamp: "now", level: "error", message: "未知错误" }, t);
    expect(item?.kind).toBe("config.remoteIm.logKindSystem");
    expect(item?.title).toBe("config.remoteIm.logAbnormalTitle");
  });
});
