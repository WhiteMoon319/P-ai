import { describe, expect, it, vi } from "vitest";
import {
  recoverSidebarForegroundStreaming,
  type SidebarForegroundRecoveryDependencies,
} from "./sidebar-foreground-recovery";

function createDependencies() {
  return {
    probeStream: vi.fn<SidebarForegroundRecoveryDependencies["probeStream"]>(async () => true),
    resumeSubscription: vi.fn<SidebarForegroundRecoveryDependencies["resumeSubscription"]>(async () => null),
    applyRuntimeSnapshot: vi.fn<SidebarForegroundRecoveryDependencies["applyRuntimeSnapshot"]>(() => true),
    refreshMessageById: vi.fn<SidebarForegroundRecoveryDependencies["refreshMessageById"]>(async () => true),
    finalizeMessage: vi.fn<SidebarForegroundRecoveryDependencies["finalizeMessage"]>(),
  };
}

describe("recoverSidebarForegroundStreaming", () => {
  it("连接与流身份健康时保持当前投影", async () => {
    const dependencies = createDependencies();
    const outcome = await recoverSidebarForegroundStreaming({
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

  it("订阅丢失后只恢复订阅并更新目标流投影", async () => {
    const dependencies = createDependencies();
    dependencies.probeStream.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    dependencies.resumeSubscription.mockResolvedValueOnce({
      runtimeState: "assistant_streaming",
      streamCache: {
        persistedAssistantMessageId: "assistant-1",
        updatedAt: "revision-2",
      },
    });

    const outcome = await recoverSidebarForegroundStreaming({
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

  it("后端已完成时只读取并收口目标消息", async () => {
    const dependencies = createDependencies();
    const outcome = await recoverSidebarForegroundStreaming({
      conversationId: "conversation-1",
      runtimeSnapshot: { runtimeState: "idle" },
      frontendStreaming: true,
      frontendMessageId: "assistant-1",
    }, dependencies);

    expect(outcome).toBe("handled");
    expect(dependencies.refreshMessageById).toHaveBeenCalledWith("conversation-1", "assistant-1");
    expect(dependencies.finalizeMessage).toHaveBeenCalledWith("assistant-1");
    expect(dependencies.resumeSubscription).not.toHaveBeenCalled();
  });
});
