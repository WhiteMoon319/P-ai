import { describe, expect, it, vi } from "vitest";
import { createExclusiveChatViewSubscriptionSlot } from "./exclusive-chat-view-subscription-slot";

describe("exclusiveChatViewSubscriptionSlot", () => {
  it("切换追问所有者时先解绑旧会话再绑定新会话", async () => {
    const order: string[] = [];
    const slot = createExclusiveChatViewSubscriptionSlot();
    await slot.acquire({
      ownerId: "view-a",
      conversationId: "conversation-a",
      bind: async () => { order.push("bind-a"); },
      unbind: async () => { order.push("unbind-a"); },
    });
    await slot.acquire({
      ownerId: "view-b",
      conversationId: "conversation-b",
      bind: async () => { order.push("bind-b"); },
      unbind: async () => { order.push("unbind-b"); },
    });

    expect(order).toEqual(["bind-a", "unbind-a", "bind-b"]);
  });

  it("新标签绑定必须等待旧组件已经发起的异步解绑完成", async () => {
    const order: string[] = [];
    let finishUnbind: (() => void) | undefined;
    const slot = createExclusiveChatViewSubscriptionSlot();
    await slot.acquire({
      ownerId: "view-a",
      conversationId: "conversation-a",
      bind: async () => { order.push("bind-a"); },
      unbind: vi.fn(async () => {}),
    });
    const unbindPromise = new Promise<void>((resolve) => {
      order.push("unbind-a-start");
      finishUnbind = () => {
        order.push("unbind-a-finish");
        resolve();
      };
    });
    const releasing = slot.release("view-a", unbindPromise);
    const acquiring = slot.acquire({
      ownerId: "view-b",
      conversationId: "conversation-b",
      bind: async () => { order.push("bind-b"); },
      unbind: async () => { order.push("unbind-b"); },
    });

    await Promise.resolve();
    expect(order).not.toContain("bind-b");
    finishUnbind?.();
    await Promise.all([releasing, acquiring]);
    expect(order).toEqual(["bind-a", "unbind-a-start", "unbind-a-finish", "bind-b"]);
  });

  it("释放非当前所有者不会解绑当前追问", async () => {
    const unbind = vi.fn(async () => {});
    const slot = createExclusiveChatViewSubscriptionSlot();
    await slot.acquire({
      ownerId: "view-a",
      conversationId: "conversation-a",
      bind: async () => {},
      unbind,
    });

    await slot.release("view-b");
    expect(unbind).not.toHaveBeenCalled();
  });
});
