import { describe, expect, it, vi } from "vitest";
import {
  createLatestTaskRunner,
  createForegroundTailWatermarkCoordinator,
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
        return {
          shouldBindStream: true,
          runtimeState: "assistant_streaming",
          streamCache: { persistedAssistantMessageId: "assistant-1" },
        };
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

  it("水位游标由各宿主独立维护，且只把当前会话标记为待正式尾部对账", async () => {
    const requestChanges = vi.fn(async (since: string) => ({
      changedConversationIds: since ? ["conversation-b"] : ["conversation-a"],
      serverTime: since ? "watermark-2" : "watermark-1",
    }));
    const app = createForegroundTailWatermarkCoordinator({ requestChanges });
    const web = createForegroundTailWatermarkCoordinator({ requestChanges });

    await app.observeCurrentConversation("conversation-a");
    await web.observeCurrentConversation("conversation-b");
    await app.observeCurrentConversation("conversation-b");

    expect(requestChanges.mock.calls.map(([since]) => since)).toEqual(["", "", "watermark-1"]);
    expect(app.shouldReconcileTail("conversation-a")).toBe(false);
    expect(app.shouldReconcileTail("conversation-b")).toBe(true);
    expect(web.shouldReconcileTail("conversation-b")).toBe(false);
    app.markTailReconciled("conversation-b");
    expect(app.shouldReconcileTail("conversation-b")).toBe(false);
  });
});
