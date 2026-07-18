import { onMounted, onScopeDispose, ref, watch, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import type {
  RemoteImContactDashboardSnapshot,
  RemoteImContactDashboardSyncResult,
} from "../../../types/app";

const REMOTE_IM_CONTACT_DASHBOARD_UPDATED_EVENT = "easy-call:remote-im-contact-dashboard-updated";

type UseRemoteImContactDashboardOptions = {
  contactId: Ref<string>;
  enabled: Ref<boolean>;
};

function normalizedContactId(value: unknown) {
  return String(value || "").trim();
}

function dashboardAvailable(options: UseRemoteImContactDashboardOptions) {
  if (!isTauriRuntimeAvailable() || !options.enabled.value) return false;
  if (typeof document !== "undefined" && document.visibilityState === "hidden") return false;
  return !!normalizedContactId(options.contactId.value);
}

export function useRemoteImContactDashboard(options: UseRemoteImContactDashboardOptions) {
  const snapshot = ref<RemoteImContactDashboardSnapshot | null>(null);
  let unlisten: UnlistenFn | null = null;
  let disposed = false;
  let requestSequence = 0;
  let subscribedContactId = "";

  function applySnapshot(next: RemoteImContactDashboardSnapshot | null | undefined) {
    const contactId = normalizedContactId(options.contactId.value);
    if (!next || normalizedContactId(next.contactId) !== contactId) return;
    snapshot.value = next;
  }

  async function unsubscribe(contactId = subscribedContactId) {
    const target = normalizedContactId(contactId);
    if (!target || !isTauriRuntimeAvailable()) return;
    if (subscribedContactId === target) subscribedContactId = "";
    await invokeTauri("remote_im_unsubscribe_contact_dashboard", {
      input: { contactId: target },
    }).catch((error) => {
      console.debug("[远程会话仪表盘] 取消订阅失败", error);
    });
  }

  async function subscribeCurrentContact() {
    if (!dashboardAvailable(options)) return;
    const contactId = normalizedContactId(options.contactId.value);
    const sequence = ++requestSequence;
    try {
      const next = await invokeTauri<RemoteImContactDashboardSnapshot>(
        "remote_im_subscribe_contact_dashboard",
        { input: { contactId } },
      );
      if (disposed || sequence !== requestSequence || contactId !== normalizedContactId(options.contactId.value)) return;
      subscribedContactId = contactId;
      applySnapshot(next);
    } catch (error) {
      if (!disposed && sequence === requestSequence) {
        console.debug("[远程会话仪表盘] 初始快照读取失败", error);
      }
    }
  }

  async function synchronizeWatermark() {
    if (!dashboardAvailable(options)) return;
    const contactId = normalizedContactId(options.contactId.value);
    const sequence = ++requestSequence;
    try {
      const result = await invokeTauri<RemoteImContactDashboardSyncResult>(
        "remote_im_sync_contact_dashboard",
        {
          input: {
            contactId,
            knownWatermark: snapshot.value?.watermark || undefined,
          },
        },
      );
      if (disposed || sequence !== requestSequence || contactId !== normalizedContactId(options.contactId.value)) return;
      // 即使水位没有变化，也以服务端刚计算的能量刷新显示值，避免前台恢复后展示旧的回能数值。
      applySnapshot(result?.snapshot);
    } catch (error) {
      if (!disposed && sequence === requestSequence) {
        console.debug("[远程会话仪表盘] 前台水位校验失败", error);
      }
    }
  }

  async function syncSubscription() {
    const activeContactId = normalizedContactId(options.contactId.value);
    const shouldSubscribe = dashboardAvailable(options);
    if (!shouldSubscribe) {
      ++requestSequence;
      if (subscribedContactId) void unsubscribe(subscribedContactId);
      snapshot.value = null;
      return;
    }
    if (subscribedContactId && subscribedContactId !== activeContactId) {
      void unsubscribe(subscribedContactId);
      snapshot.value = null;
    }
    if (subscribedContactId === activeContactId) {
      await synchronizeWatermark();
      return;
    }
    await subscribeCurrentContact();
  }

  function handleForegroundWake() {
    if (typeof document !== "undefined" && document.visibilityState === "hidden") return;
    void syncSubscription();
  }

  watch(
    () => [normalizedContactId(options.contactId.value), options.enabled.value],
    () => { void syncSubscription(); },
    { immediate: true },
  );

  onMounted(() => {
    if (isTauriRuntimeAvailable()) {
      void listen<RemoteImContactDashboardSnapshot>(REMOTE_IM_CONTACT_DASHBOARD_UPDATED_EVENT, (event) => {
        if (!dashboardAvailable(options)) return;
        applySnapshot(event.payload);
      }).then((dispose) => {
        if (disposed) {
          dispose();
          return;
        }
        unlisten = dispose;
      }).catch((error) => {
        console.debug("[远程会话仪表盘] 推送监听注册失败", error);
      });
    }
    window.addEventListener("focus", handleForegroundWake);
    document.addEventListener("visibilitychange", handleForegroundWake);
  });

  onScopeDispose(() => {
    disposed = true;
    ++requestSequence;
    if (unlisten) unlisten();
    void unsubscribe(subscribedContactId);
    window.removeEventListener("focus", handleForegroundWake);
    document.removeEventListener("visibilitychange", handleForegroundWake);
  });

  return { snapshot, synchronizeWatermark };
}
