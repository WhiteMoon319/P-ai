import { describe, expect, it } from "vitest";
import { decideForegroundRecovery } from "./foreground-recovery-decision";

describe("decideForegroundRecovery", () => {
  it("流式状态一致时先探针，不提前刷新", () => {
    expect(decideForegroundRecovery({
      backendStreaming: true,
      frontendStreaming: true,
      backendMessageId: "assistant-1",
      frontendMessageId: "assistant-1",
    })).toBe("probe_stream");
  });

  it("探针健康且身份一致时保持当前页面", () => {
    expect(decideForegroundRecovery({
      backendStreaming: true,
      frontendStreaming: true,
      backendMessageId: "assistant-1",
      frontendMessageId: "assistant-1",
      probeState: "healthy",
    })).toBe("keep");
  });

  it("后端完成而前端仍流式时只刷新目标消息", () => {
    expect(decideForegroundRecovery({
      backendStreaming: false,
      frontendStreaming: true,
      frontendMessageId: "assistant-1",
    })).toBe("refresh_target_message");
  });

  it("订阅断开但仍有正式 assistant ID 时先恢复订阅", () => {
    expect(decideForegroundRecovery({
      backendStreaming: true,
      frontendStreaming: true,
      backendMessageId: "assistant-1",
      frontendMessageId: "assistant-1",
      probeState: "unhealthy",
    })).toBe("resume_stream");
  });

  it("轮次身份变化时恢复目标流而不是保持旧投影", () => {
    expect(decideForegroundRecovery({
      backendStreaming: true,
      frontendStreaming: true,
      backendMessageId: "assistant-1",
      frontendMessageId: "assistant-1",
      backendActivationId: "activation-2",
      frontendActivationId: "activation-1",
      probeState: "healthy",
    })).toBe("resume_stream");
  });

  it("前端只有流式忙碌态但没有目标消息时恢复后端目标", () => {
    expect(decideForegroundRecovery({
      backendStreaming: true,
      frontendStreaming: true,
      backendMessageId: "assistant-1",
      probeState: "healthy",
    })).toBe("resume_stream");
  });
});
