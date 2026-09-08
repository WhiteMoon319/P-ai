import { effectScope, nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeTauriMock = vi.hoisted(() => vi.fn());

vi.mock("../../../services/tauri-api", () => ({
  invokeTauri: invokeTauriMock,
  onTransportNotification: () => () => {},
}));

import {
  CHAT_QUEUE_OUT_OF_SYNC_EVENT,
  useChatQueue,
} from "./use-chat-queue";

const QUEUED_EVENT = {
  id: "event-1",
  source: "user",
  queueMode: "normal",
  createdAt: "2026-09-08T00:00:00+08:00",
  messagePreview: "hello",
  messageText: "hello",
  conversationId: "conversation-a",
} as const;

function stubWindow() {
  const dispatched: Event[] = [];
  vi.stubGlobal("window", {
    dispatchEvent: (event: Event) => {
      dispatched.push(event);
      return true;
    },
  });
  return dispatched;
}

function snapshotCalls() {
  return invokeTauriMock.mock.calls.filter(([method]) => method === "chat.queueSnapshot");
}

function outOfSyncEvents(dispatched: Event[]) {
  return dispatched.filter((event) => event.type === CHAT_QUEUE_OUT_OF_SYNC_EVENT);
}

describe("useChatQueue 失配准确反馈", () => {
  beforeEach(() => {
    invokeTauriMock.mockReset();
    stubWindow();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("撤回命中只刷队列不广播失配", async () => {
    invokeTauriMock.mockImplementation(async (method: string) => {
      if (method === "chat.queueRecall") return { removed: true, messageText: "hello", notInQueue: false };
      if (method === "chat.queueSnapshot") return [];
      if (method === "chat.sessionStateSnapshot") return "idle";
      return null;
    });
    const dispatched = stubWindow();
    const scope = effectScope();
    const queue = scope.run(() => useChatQueue({ enabled: true }))!;
    await queue.startPolling();
    const snapshotBefore = snapshotCalls().length;

    const result = await queue.recallQueueEvent("event-1");
    await nextTick();

    expect(result.removed).toBe(true);
    expect(result.notInQueue).toBe(false);
    expect(snapshotCalls().length).toBeGreaterThan(snapshotBefore);
    expect(outOfSyncEvents(dispatched)).toHaveLength(0);
    queue.stopPolling();
    scope.stop();
  });

  it("撤回拿到准确不在队列时刷队列并广播失配", async () => {
    invokeTauriMock.mockImplementation(async (method: string) => {
      if (method === "chat.queueRecall") return { removed: false, messageText: "", notInQueue: true };
      if (method === "chat.queueSnapshot") return [QUEUED_EVENT];
      if (method === "chat.sessionStateSnapshot") return "idle";
      return null;
    });
    const dispatched = stubWindow();
    const scope = effectScope();
    const queue = scope.run(() => useChatQueue({ enabled: true }))!;
    await queue.startPolling();
    await nextTick();
    const snapshotBefore = snapshotCalls().length;

    const result = await queue.recallQueueEvent("event-1");
    await nextTick();

    expect(result.removed).toBe(false);
    expect(result.notInQueue).toBe(true);
    expect(snapshotCalls().length).toBeGreaterThan(snapshotBefore);
    const syncs = outOfSyncEvents(dispatched);
    expect(syncs).toHaveLength(1);
    expect((syncs[0] as CustomEvent).detail).toEqual({
      eventId: "event-1",
      conversationId: "conversation-a",
      reason: "not_in_queue",
    });
    queue.stopPolling();
    scope.stop();
  });

  it("撤回失败无准确码时不刷队列不广播", async () => {
    invokeTauriMock.mockImplementation(async (method: string) => {
      if (method === "chat.queueRecall") return { removed: false, messageText: "" };
      if (method === "chat.queueSnapshot") return [QUEUED_EVENT];
      if (method === "chat.sessionStateSnapshot") return "idle";
      return null;
    });
    const dispatched = stubWindow();
    const scope = effectScope();
    const queue = scope.run(() => useChatQueue({ enabled: true }))!;
    await queue.startPolling();
    await nextTick();
    const snapshotBefore = snapshotCalls().length;

    const result = await queue.recallQueueEvent("event-1");
    await nextTick();

    expect(result.removed).toBe(false);
    expect(result.notInQueue).toBe(false);
    expect(snapshotCalls().length).toBe(snapshotBefore);
    expect(outOfSyncEvents(dispatched)).toHaveLength(0);
    queue.stopPolling();
    scope.stop();
  });

  it("切引导命中只刷队列不广播失配", async () => {
    invokeTauriMock.mockImplementation(async (method: string) => {
      if (method === "chat.queueMarkGuided") return { updated: true, notInQueue: false };
      if (method === "chat.queueSnapshot") return [];
      if (method === "chat.sessionStateSnapshot") return "idle";
      return null;
    });
    const dispatched = stubWindow();
    const scope = effectScope();
    const queue = scope.run(() => useChatQueue({ enabled: true }))!;
    await queue.startPolling();
    const snapshotBefore = snapshotCalls().length;

    const result = await queue.markGuided("event-1");
    await nextTick();

    expect(result.updated).toBe(true);
    expect(result.notInQueue).toBe(false);
    expect(snapshotCalls().length).toBeGreaterThan(snapshotBefore);
    expect(outOfSyncEvents(dispatched)).toHaveLength(0);
    queue.stopPolling();
    scope.stop();
  });

  it("切引导拿到准确不在队列时刷队列并广播失配", async () => {
    invokeTauriMock.mockImplementation(async (method: string) => {
      if (method === "chat.queueMarkGuided") return { updated: false, notInQueue: true };
      if (method === "chat.queueSnapshot") return [QUEUED_EVENT];
      if (method === "chat.sessionStateSnapshot") return "idle";
      return null;
    });
    const dispatched = stubWindow();
    const scope = effectScope();
    const queue = scope.run(() => useChatQueue({ enabled: true }))!;
    await queue.startPolling();
    await nextTick();
    const snapshotBefore = snapshotCalls().length;

    const result = await queue.markGuided("event-1");
    await nextTick();

    expect(result.updated).toBe(false);
    expect(result.notInQueue).toBe(true);
    expect(snapshotCalls().length).toBeGreaterThan(snapshotBefore);
    const syncs = outOfSyncEvents(dispatched);
    expect(syncs).toHaveLength(1);
    expect((syncs[0] as CustomEvent).detail).toEqual({
      eventId: "event-1",
      conversationId: "conversation-a",
      reason: "not_in_queue",
    });
    queue.stopPolling();
    scope.stop();
  });
});
