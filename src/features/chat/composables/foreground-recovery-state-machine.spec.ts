import { describe, expect, it, vi } from "vitest";
import {
  recoverForegroundStreaming,
  type ForegroundRecoveryDependencies,
} from "./foreground-recovery-state-machine";

function createDependencies(): ForegroundRecoveryDependencies {
  return {
    probeStream: vi.fn(async () => true),
    resumeSubscription: vi.fn(async () => null),
    applyRuntimeSnapshot: vi.fn(() => true),
    refreshMessageById: vi.fn(async () => true),
    finalizeMessage: vi.fn(),
  };
}

describe("foregroundRecoveryStateMachine", () => {
  it("流和身份健康时保留当前投影，不刷新或重载", async () => {
    const dependencies = createDependencies();

    const outcome = await recoverForegroundStreaming({
      conversationId: "conversation-1",
      runtimeSnapshot: {
        runtimeState: "assistant_streaming",
        streamCache: {
          persistedAssistantMessageId: "assistant-1",
          activationId: "activation-1",
          requestId: "request-1",
          updatedAt: "revision-1",
        },
      },
      frontendStreaming: true,
      frontendMessageId: "assistant-1",
      frontendActivationId: "activation-1",
      frontendRequestId: "request-1",
      frontendRevision: "revision-1",
    }, dependencies);

    expect(outcome).toBe("handled");
    expect(dependencies.probeStream).toHaveBeenCalledTimes(1);
    expect(dependencies.resumeSubscription).not.toHaveBeenCalled();
    expect(dependencies.refreshMessageById).not.toHaveBeenCalled();
  });

  it("订阅丢失时只恢复当前流投影", async () => {
    const dependencies = createDependencies();
    dependencies.probeStream = vi.fn()
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);
    dependencies.resumeSubscription = vi.fn(async () => ({
      runtimeState: "assistant_streaming",
      streamCache: { persistedAssistantMessageId: "assistant-1" },
    }));

    const outcome = await recoverForegroundStreaming({
      conversationId: "conversation-1",
      runtimeSnapshot: {
        runtimeState: "assistant_streaming",
        streamCache: { persistedAssistantMessageId: "assistant-1" },
      },
      frontendStreaming: true,
      frontendMessageId: "assistant-1",
    }, dependencies);

    expect(outcome).toBe("handled");
    expect(dependencies.resumeSubscription).toHaveBeenCalledWith("conversation-1");
    expect(dependencies.applyRuntimeSnapshot).toHaveBeenCalledTimes(1);
    expect(dependencies.refreshMessageById).not.toHaveBeenCalled();
  });

  it("后端完成、前端仍流式时只刷新该目标消息", async () => {
    const dependencies = createDependencies();

    const outcome = await recoverForegroundStreaming({
      conversationId: "conversation-1",
      runtimeSnapshot: { runtimeState: "idle" },
      frontendStreaming: true,
      frontendMessageId: "assistant-1",
    }, dependencies);

    expect(outcome).toBe("handled");
    expect(dependencies.refreshMessageById).toHaveBeenCalledWith("conversation-1", "assistant-1");
    expect(dependencies.finalizeMessage).toHaveBeenCalledWith("assistant-1");
  });

  it("双方空闲时只要求宿主检查正式消息尾部", async () => {
    const dependencies = createDependencies();

    const outcome = await recoverForegroundStreaming({
      conversationId: "conversation-1",
      runtimeSnapshot: { runtimeState: "idle" },
      frontendStreaming: false,
    }, dependencies);

    expect(outcome).toBe("check_freshness");
    expect(dependencies.probeStream).not.toHaveBeenCalled();
    expect(dependencies.refreshMessageById).not.toHaveBeenCalled();
  });
});
