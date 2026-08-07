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

  it("每个视图实例独立维护 freshness 指纹，且只把当前会话标记为待正式尾部对账", async () => {
    const requestFreshness = vi.fn(async (conversationId: string) => ({
      lastMessageId: conversationId === "conversation-b" ? "assistant-b" : "assistant-a",
      updatedAt: "2026-08-07T00:00:00Z",
    }));
    const app = createForegroundTailWatermarkCoordinator({ requestFreshness });
    const web = createForegroundTailWatermarkCoordinator({ requestFreshness });

    await app.observeCurrentConversation("conversation-a");
    await web.observeCurrentConversation("conversation-b");
    await app.observeCurrentConversation("conversation-b");

    expect(requestFreshness.mock.calls.map(([conversationId]) => conversationId)).toEqual([
      "conversation-a",
      "conversation-b",
      "conversation-b",
    ]);
    expect(app.shouldReconcileTail("conversation-a")).toBe(false);
    expect(app.shouldReconcileTail("conversation-b")).toBe(true);
    // web 实例自己观察过 conversation-b，独立建立指纹并标记待对账
    expect(web.shouldReconcileTail("conversation-b")).toBe(true);
    app.markTailReconciled("conversation-b");
    expect(app.shouldReconcileTail("conversation-b")).toBe(false);
    // app 实例清理不影响 web 实例的独立状态
    expect(web.shouldReconcileTail("conversation-b")).toBe(true);
  });

  it("同一会话 freshness 未变化时再次观察不会重复标记待对账", async () => {
    const requestFreshness = vi.fn(async () => ({
      lastMessageId: "assistant-a",
      updatedAt: "2026-08-07T00:00:00Z",
    }));
    const app = createForegroundTailWatermarkCoordinator({ requestFreshness });

    await app.observeCurrentConversation("conversation-a");
    expect(app.shouldReconcileTail("conversation-a")).toBe(true);
    app.markTailReconciled("conversation-a");
    await app.observeCurrentConversation("conversation-a");
    expect(app.shouldReconcileTail("conversation-a")).toBe(false);
  });

  it("会话 updatedAt 变化即使 lastMessageId 相同也会再次标记待对账", async () => {
    const requestFreshness = vi.fn(async () => ({
      lastMessageId: "assistant-a",
      updatedAt: "2026-08-07T00:00:00Z",
    }));
    const app = createForegroundTailWatermarkCoordinator({ requestFreshness });

    await app.observeCurrentConversation("conversation-a");
    expect(app.shouldReconcileTail("conversation-a")).toBe(true);
    app.markTailReconciled("conversation-a");

    requestFreshness.mockResolvedValueOnce({
      lastMessageId: "assistant-a",
      updatedAt: "2026-08-07T00:00:01Z",
    });
    await app.observeCurrentConversation("conversation-a");
    expect(app.shouldReconcileTail("conversation-a")).toBe(true);
  });
});
