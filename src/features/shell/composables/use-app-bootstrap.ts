import type { AgentWorkSignalPayload, AppConfig } from "../../../types/app";
import type { AppThemeState } from "../theme/theme-types";
import { onTransportNotification } from "../../../services/tauri-api";

type ViewMode = "chat" | "archives" | "config";
type ConversationApiSettingsPayload = {
  assistantDepartmentApiConfigId: string;
  visionApiConfigId?: string;
  toolReviewApiConfigId?: string;
  sttApiConfigId?: string;
  sttAutoSend?: boolean;
};
type ChatSettingsPayload = {
  assistantDepartmentAgentId: string;
  userAlias: string;
  responseStyleId: string;
  pdfReadMode?: "text" | "image";
  backgroundVoiceScreenshotKeywords?: string;
  backgroundVoiceScreenshotMode?: "desktop" | "focused_window";
  instructionPresets?: Array<{ id: string; name: string; prompt: string }>;
};

export type TerminalApprovalRequestPayload = {
  requestId: string;
  title: string;
  message: string;
  approvalKind: string;
  sessionId: string;
  toolName?: string;
  summary?: string;
  callPreview?: string;
  cwd?: string;
  command?: string;
  requestedPath?: string;
  reason?: string;
  existingPaths?: string[];
  targetPaths?: string[];
  reviewOpinion?: string;
  reviewModelName?: string;
};

type AppBootstrapOptions = {
  setViewMode: (mode: ViewMode) => void;
  initWindowMode: () => ViewMode;
  onThemeChanged: (theme: AppThemeState) => void;
  onLocaleChanged: (locale: string) => void;
  onTerminalApprovalRequested?: (payload: TerminalApprovalRequestPayload) => void;
  onConversationApiUpdated?: (payload: ConversationApiSettingsPayload) => void;
  onChatSettingsUpdated?: (payload: ChatSettingsPayload) => void;
  onConfigUpdated?: (payload: AppConfig) => void;
  onAgentWorkStarted?: (payload: AgentWorkSignalPayload) => void;
  onAgentWorkStopped?: (payload: AgentWorkSignalPayload) => void;
  onRecordHotkeyProbe?: (payload: { state: "pressed" | "released"; seq: number }) => void;
  onToolReviewReportsUpdated?: (payload: { conversationId?: string; reportId?: string; status?: string }) => void;
};

export function useAppBootstrap(options: AppBootstrapOptions) {
  const unlisteners: Array<() => void> = [];

  async function mount() {
    const mode = options.initWindowMode();
    options.setViewMode(mode);
    try {
      const subscribe = <T>(method: string, handler: (payload: T) => void) => {
        unlisteners.push(onTransportNotification<T>(method, handler));
      };
      subscribe<AppThemeState>("theme.changed", (payload) => {
        options.onThemeChanged(payload);
      });
      subscribe<string>("locale.changed", (payload) => {
        options.onLocaleChanged(payload);
      });
      subscribe<TerminalApprovalRequestPayload>("terminalApproval.requested", (payload) => {
        options.onTerminalApprovalRequested?.(payload);
      });
      subscribe<ConversationApiSettingsPayload>("conversation.apiUpdated", (payload) => {
        options.onConversationApiUpdated?.(payload);
      });
      subscribe<ChatSettingsPayload>("chat.settingsUpdated", (payload) => {
        options.onChatSettingsUpdated?.(payload);
      });
      subscribe<AppConfig>("config.updated", (payload) => {
        options.onConfigUpdated?.(payload);
      });
      subscribe<AgentWorkSignalPayload>("agentWork.started", (payload) => {
        options.onAgentWorkStarted?.(payload);
      });
      subscribe<AgentWorkSignalPayload>("agentWork.stopped", (payload) => {
        options.onAgentWorkStopped?.(payload);
      });
      subscribe<unknown>("recordHotkey.probe", (payload) => {
          const normalizedPayload = payload as
            | { state?: unknown; seq?: unknown }
            | string
            | null
            | undefined;
          if (typeof normalizedPayload === "string") {
            const text = normalizedPayload.trim().toLowerCase();
            if (text === "pressed" || text === "released") {
              options.onRecordHotkeyProbe?.({ state: text, seq: 0 });
            }
            return;
          }
          const text = String(normalizedPayload?.state || "").trim().toLowerCase();
          if (text !== "pressed" && text !== "released") return;
          const seqRaw = Number(normalizedPayload?.seq);
          const seq = Number.isFinite(seqRaw) && seqRaw > 0 ? Math.floor(seqRaw) : 0;
          options.onRecordHotkeyProbe?.({ state: text, seq });
      });
      subscribe<{ conversationId?: string; reportId?: string; status?: string }>("toolReview.reportsUpdated", (payload) => {
        options.onToolReviewReportsUpdated?.(payload || {});
      });
    } catch (error) {
      unmount();
      throw error;
    }
  }

  function unmount() {
    while (unlisteners.length > 0) {
      const fn = unlisteners.pop();
      if (fn) fn();
    }
  }

  return {
    mount,
    unmount,
  };
}
