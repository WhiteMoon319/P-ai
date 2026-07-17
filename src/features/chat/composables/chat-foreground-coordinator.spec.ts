import { describe, expect, it, vi } from "vitest";
import {
  createLatestTaskRunner,
  reconcileForegroundConversation,
  runForegroundSnapshotBindingTransaction,
} from "./chat-foreground-coordinator";

describe("chatForegroundCoordinator", () => {
  it("快照事务在解绑完成后才绑定和恢复", async () => {
    const order: string[] = [];
    let finishUnbind: (() => void) | undefined;
    const transaction = runForegroundSnapshotBindingTransaction({
      conversationId: "conversation-a",
      isCurrent: () => true,
      clearRuntime: () => { order.push("clear"); },
      unbind: () => new Promise<void>((resolve) => {
        order.push("unbind-start");
        finishUnbind = () => {
          order.push("unbind-finish");
          resolve();
        };
      }),
      requestSnapshot: async () => {
        order.push("snapshot");
        return { shouldBindStream: true, streamCache: null };
      },
      applySnapshot: () => { order.push("apply"); },
      bind: async () => { order.push("bind"); },
      resume: () => { order.push("resume"); },
    });

    await vi.waitFor(() => expect(order).toContain("apply"));
    expect(order).not.toContain("bind");
    finishUnbind?.();
    await transaction;
    expect(order).toEqual(["clear", "unbind-start", "snapshot", "apply", "unbind-finish", "bind", "resume"]);
  });

  it("后端完成而前端仍流式时只刷新目标消息", async () => {
    const refreshTargetMessage = vi.fn(async () => true);
    const reloadConversation = vi.fn(async () => {});
    const action = await reconcileForegroundConversation({
      conversationId: "conversation-a",
      isCurrent: () => true,
      requestRuntimeSnapshot: async () => ({
        runtimeState: "idle",
        streamCache: { persistedAssistantMessageId: "assistant-1" },
      }),
      applyRuntimeState: () => {},
      frontendStreaming: () => true,
      readFrontendStreamCache: () => ({ persistedAssistantMessageId: "assistant-1" }),
      probeStream: async () => true,
      readCurrentFormalTailMessageId: () => "assistant-1",
      requestLatestFormalTailMessageId: async () => "assistant-1",
      refreshTargetMessage,
      finalizeTargetRefresh: () => {},
      reloadConversation,
    });

    expect(action).toBe("refresh_target_message");
    expect(refreshTargetMessage).toHaveBeenCalledWith("assistant-1");
    expect(reloadConversation).not.toHaveBeenCalled();
  });

  it("前后端均已完成时只回读并刷新最后一条正式消息", async () => {
    const refreshTargetMessage = vi.fn(async () => true);
    const finalizeTargetRefresh = vi.fn();
    const reloadConversation = vi.fn(async () => {});
    const action = await reconcileForegroundConversation({
      conversationId: "conversation-a",
      isCurrent: () => true,
      requestRuntimeSnapshot: async () => ({ runtimeState: "idle" }),
      applyRuntimeState: () => {},
      frontendStreaming: () => false,
      readFrontendStreamCache: () => null,
      probeStream: async () => true,
      readCurrentFormalTailMessageId: () => "assistant-1",
      requestLatestFormalTailMessageId: async () => "assistant-1",
      refreshTargetMessage,
      finalizeTargetRefresh,
      reloadConversation,
    });

    expect(action).toBe("refresh_target_message");
    expect(refreshTargetMessage).toHaveBeenCalledTimes(1);
    expect(refreshTargetMessage).toHaveBeenCalledWith("assistant-1");
    expect(finalizeTargetRefresh).not.toHaveBeenCalled();
    expect(reloadConversation).not.toHaveBeenCalled();
  });

  it("latest runner 在运行期间收到新输入后会再执行最新任务", async () => {
    const values: string[] = [];
    let finishFirst: (() => void) | undefined;
    const runner = createLatestTaskRunner<string>(async (value) => {
      values.push(value);
      if (values.length === 1) {
        await new Promise<void>((resolve) => { finishFirst = resolve; });
      }
    });

    const first = runner.run("first");
    void runner.run("second");
    finishFirst?.();
    await first;
    expect(values).toEqual(["first", "second"]);
  });
});
