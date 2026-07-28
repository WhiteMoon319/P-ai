export type ForegroundSnapshotBindingStage =
  | "clear_runtime"
  | "unbind"
  | "request_snapshot"
  | "apply_snapshot"
  | "bind"
  | "resume";

export async function runForegroundSnapshotBindingTransaction<TSnapshot extends {
  shouldBindStream?: boolean;
  streamCache?: unknown;
}>(input: {
  conversationId: string;
  isCurrent: () => boolean;
  clearRuntime: () => void;
  unbind: () => Promise<void>;
  requestSnapshot: () => Promise<TSnapshot | null>;
  applySnapshot: (snapshot: TSnapshot) => void;
  bind: () => Promise<void>;
  resume: (snapshot: TSnapshot) => void;
  onStage?: (stage: ForegroundSnapshotBindingStage) => void;
  onUnbindError?: (error: unknown) => void;
}): Promise<TSnapshot | null> {
  input.onStage?.("clear_runtime");
  input.clearRuntime();
  input.onStage?.("unbind");
  const unbindPromise = input.unbind().catch((error) => {
    input.onUnbindError?.(error);
  });
  input.onStage?.("request_snapshot");
  const snapshot = await input.requestSnapshot();
  if (!snapshot || !input.isCurrent()) {
    await unbindPromise;
    return null;
  }
  input.onStage?.("apply_snapshot");
  input.applySnapshot(snapshot);
  await unbindPromise;
  if (!input.isCurrent()) return null;
  if (!snapshot.shouldBindStream) return snapshot;
  input.onStage?.("bind");
  await input.bind();
  if (!input.isCurrent()) return null;
  input.onStage?.("resume");
  input.resume(snapshot);
  return snapshot;
}

export function createLatestTaskRunner<T>(task: (input: T) => Promise<void>) {
  let latestInput: T | undefined;
  let rerunRequested = false;
  let runningPromise: Promise<void> | null = null;
  let cancelled = false;

  function run(input: T): Promise<void> {
    latestInput = input;
    rerunRequested = true;
    if (runningPromise) return runningPromise;
    runningPromise = (async () => {
      while (rerunRequested && !cancelled) {
        rerunRequested = false;
        await task(latestInput as T);
      }
    })().finally(() => {
      runningPromise = null;
    });
    return runningPromise;
  }

  function cancel() {
    cancelled = true;
    rerunRequested = false;
  }

  return { run, cancel };
}
