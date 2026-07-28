import { onBeforeUnmount, reactive } from "vue";
import { invokeTauri, onTransportNotification } from "../../../services/tauri-api";

export type MessageStoreMigrationGateMode = "idle" | "checking" | "migrating" | "blocked" | "error";

type MessageStoreMigrationPreflightItem = {
  conversationId: string;
  title: string;
  status: string;
  messageCount: number;
  reason?: string | null;
};

type MessageStoreMigrationPreflightReport = {
  migrationRequired?: boolean;
  totalConversations: number;
  readyCount: number;
  legacyCount: number;
  busyCount?: number;
  discardedCount?: number;
  blockedCount?: number;
  canAutoMigrate: boolean;
  items: MessageStoreMigrationPreflightItem[];
};

type MessageStoreMigrationProgressPayload = {
  current: number;
  total: number;
  conversationId: string;
  title: string;
  status: string;
  detail?: string | null;
};

export type MessageStoreMigrationGateBindings = {
  formatRequestFailed: (error: unknown) => string;
};

export function useMessageStoreMigrationGate(bindings: MessageStoreMigrationGateBindings) {
  const messageStoreMigration = reactive<{
    visible: boolean;
    mode: MessageStoreMigrationGateMode;
    message: string;
    current: number;
    total: number;
    blockedItems: MessageStoreMigrationPreflightItem[];
  }>({
    visible: false,
    mode: "idle",
    message: "",
    current: 0,
    total: 0,
    blockedItems: [],
  });

  let messageStoreMigrationProgressUnlisten: (() => void) | null = null;

  function resetMessageStoreMigrationGate() {
    messageStoreMigration.visible = false;
    messageStoreMigration.mode = "idle";
    messageStoreMigration.message = "";
    messageStoreMigration.current = 0;
    messageStoreMigration.total = 0;
    messageStoreMigration.blockedItems = [];
  }

  async function ensureMessageStoreMigrationProgressListener() {
    if (messageStoreMigrationProgressUnlisten) return;
    messageStoreMigrationProgressUnlisten = onTransportNotification<MessageStoreMigrationProgressPayload>(
      "messageStore.migrationProgress",
      (payload) => {
        messageStoreMigration.visible = true;
        messageStoreMigration.mode = payload.status === "failed" ? "error" : "migrating";
        messageStoreMigration.current = Number(payload.current || 0);
        messageStoreMigration.total = Number(payload.total || 0);
        const title = String(payload.title || payload.conversationId || "").trim();
        const detail = String(payload.detail || "").trim();
        messageStoreMigration.message = detail || `正在迁移：${title || "会话"}`;
      },
    );
  }

  async function runMessageStoreMigrationFromGate() {
    await ensureMessageStoreMigrationProgressListener();
    messageStoreMigration.visible = true;
    messageStoreMigration.mode = "migrating";
    messageStoreMigration.message = "正在迁移会话消息仓库...";
    await invokeTauri("messageStore.migration.run", {});
    resetMessageStoreMigrationGate();
  }

  async function ensureMessageStoreMigrationGate() {
    await ensureMessageStoreMigrationProgressListener();
    const report = await invokeTauri<MessageStoreMigrationPreflightReport>(
      "messageStore.migration.check",
    );
    if (report.migrationRequired) {
      messageStoreMigration.visible = true;
      messageStoreMigration.mode = "checking";
      messageStoreMigration.message = "正在迁移会话消息仓库...";
      await runMessageStoreMigrationFromGate();
      return;
    }
    if (report.legacyCount > 0) {
      messageStoreMigration.visible = true;
      messageStoreMigration.mode = "checking";
      messageStoreMigration.message = `发现 ${report.legacyCount} 个旧会话，正在迁移...`;
      await runMessageStoreMigrationFromGate();
      return;
    }
  }

  function cancelMessageStoreMigration() {
    resetMessageStoreMigrationGate();
  }

  async function continueMessageStoreMigrationWithDiscard() {
    try {
      await runMessageStoreMigrationFromGate();
    } catch (error) {
      messageStoreMigration.mode = "error";
      messageStoreMigration.message = bindings.formatRequestFailed(error);
    }
  }

  onBeforeUnmount(() => {
    if (messageStoreMigrationProgressUnlisten) {
      messageStoreMigrationProgressUnlisten();
      messageStoreMigrationProgressUnlisten = null;
    }
  });

  return {
    messageStoreMigration,
    ensureMessageStoreMigrationGate,
    cancelMessageStoreMigration,
    continueMessageStoreMigrationWithDiscard,
  };
}
