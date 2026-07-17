export type ExclusiveChatViewSubscriptionLease = {
  ownerId: string;
  conversationId: string;
  bind: () => Promise<void>;
  unbind: () => Promise<void>;
};

export type ExclusiveChatViewSubscriptionSlot = {
  acquire: (lease: ExclusiveChatViewSubscriptionLease) => Promise<void>;
  release: (ownerId: string, unbindPromise?: Promise<void>) => Promise<void>;
};

type ActiveLease = Pick<ExclusiveChatViewSubscriptionLease, "ownerId" | "conversationId" | "unbind">;

function normalized(value: unknown): string {
  return String(value || "").trim();
}

export function createExclusiveChatViewSubscriptionSlot(): ExclusiveChatViewSubscriptionSlot {
  let activeLease: ActiveLease | null = null;
  let operationQueue: Promise<void> = Promise.resolve();

  function enqueue(operation: () => Promise<void>): Promise<void> {
    const task = operationQueue.then(operation, operation);
    operationQueue = task.catch(() => {});
    return task;
  }

  async function unbindActiveLease() {
    const lease = activeLease;
    if (!lease) return;
    activeLease = null;
    await lease.unbind();
  }

  function acquire(input: ExclusiveChatViewSubscriptionLease): Promise<void> {
    const ownerId = normalized(input.ownerId);
    const conversationId = normalized(input.conversationId);
    if (!ownerId || !conversationId) return Promise.resolve();
    return enqueue(async () => {
      const current = activeLease;
      if (
        current
        && (current.ownerId !== ownerId || current.conversationId !== conversationId)
      ) {
        await unbindActiveLease();
      }
      activeLease = {
        ownerId,
        conversationId,
        unbind: input.unbind,
      };
      try {
        await input.bind();
      } catch (error) {
        if (activeLease?.ownerId === ownerId && activeLease.conversationId === conversationId) {
          activeLease = null;
        }
        await input.unbind().catch(() => {});
        throw error;
      }
    });
  }

  function release(ownerId: string, unbindPromise?: Promise<void>): Promise<void> {
    const normalizedOwnerId = normalized(ownerId);
    if (!normalizedOwnerId) return Promise.resolve();
    return enqueue(async () => {
      if (unbindPromise) await unbindPromise;
      if (activeLease?.ownerId !== normalizedOwnerId) return;
      const lease = activeLease;
      activeLease = null;
      if (!unbindPromise) await lease.unbind();
    });
  }

  return {
    acquire,
    release,
  };
}
