<template>
  <ChatView
    class="h-full"
    :user-alias="userAlias"
    :persona-name="personaName"
    :user-avatar-url="userAvatarUrl"
    :assistant-avatar-url="assistantAvatarUrl"
    :persona-name-map="personaNameMap"
    :persona-avatar-url-map="personaAvatarUrlMap"
    :mention-entries="[]"
    :selected-mentions="runtime.selectedMentions.value"
    :latest-user-text="runtime.latestUserText.value"
    :latest-user-images="runtime.latestUserImages.value"
    :latest-assistant-text="runtime.latestAssistantText.value"
    :frontend-round-phase="runtime.flow.frontendRoundPhase.value"
    :submit-pending="runtime.submitPending.value"
    :tool-status-text="runtime.toolStatusText.value"
    :tool-status-state="runtime.toolStatusState.value"
    :chat-error-text="runtime.chatErrorText.value"
    :clipboard-images="runtime.clipboardImages.value"
    :queued-attachment-notices="runtime.queuedAttachmentNotices.value"
    :chat-input="runtime.chatInput.value"
    :instruction-presets="instructionPresets"
    :chat-input-placeholder="chatInputPlaceholder"
    :can-record="false"
    :recording="false"
    :recording-ms="0"
    :transcribing="false"
    record-hotkey=""
    :conversation-call-primary-api-config-id="runtime.preferredApiConfigId.value"
    :preferred-chat-model-id="runtime.preferredApiConfigId.value"
    tool-review-api-config-id=""
    :tool-review-refresh-tick="0"
    :chat-model-options="chatModelOptions"
    :plan-mode-enabled="runtime.planModeEnabled.value"
    :chat-usage-percent="messageBlocks.chatUsagePercent.value"
    :media-drag-active="false"
    :chatting="runtime.chatting.value"
    :trimming="false"
    :compacting-conversation="false"
    :conversation-busy="runtime.conversationBusy.value"
    :frozen="false"
    :message-blocks="messageBlocks.visibleMessageBlocks.value"
    :has-more-history="runtime.hasMoreHistory.value"
    :loading-older-history="runtime.loadingOlderHistory.value"
    :latest-own-message-align-request="0"
    :conversation-scroll-to-bottom-request="0"
    :scroll-to-bottom-behavior="'auto'"
    :current-workspace-name="workspaceName"
    :current-workspace-display-name="workspaceName"
    :current-workspace-root-path="workspaceRootPath"
    :workspaces="workspaces"
    :current-workspace-autonomous-mode="false"
    :current-department-id="departmentId"
    :active-agent-id="agentId"
    :active-conversation-id="conversationId"
    :current-todos="runtime.currentTodos.value"
    :supervision-active="false"
    :supervision-title="''"
    :supervision-dialog-open="false"
    :supervision-task-saving="false"
    :supervision-task-error="''"
    :active-supervision-task="null"
    :recent-supervision-task-history="[]"
    :current-theme="currentTheme"
    :unarchived-conversation-items="conversationItems"
    :remote-im-contact-conversations="[]"
    :conversation-items="conversationItems"
    :side-conversation-list-visible="false"
    :initial-tool-review-panel-open="false"
    :conversation-list-tab="'local'"
    :chat-left-panel-mode="'local'"
    :chat-right-panel-mode="'reader'"
    :chat-monitor-panel-mode="'delegate'"
    :create-conversation-department-options="[]"
    :default-create-conversation-department-id="departmentId"
    :ide-context-groups="[]"
    :terminal-approvals="terminalApprovals"
    :terminal-approval-resolving="terminalApprovalResolving"
    :hide-conversation-control-panel="true"
    :hide-workspace-button="true"
    :workspace-access="workspaceAccess"
    @update:chat-input="updateChatInput"
    @send-chat="runtime.send"
    @stop-chat="runtime.stop"
    @clear-chat-error="clearChatError"
    @load-older-history="runtime.loadOlderHistory"
    @approve-terminal-approval="approveTerminalApproval?.($event)"
    @deny-terminal-approval="denyTerminalApproval?.($event)"
    @approve-terminal-approval-for-session="approveTerminalApprovalForSession?.($event)"
    @approve-terminal-approval-for-workspace="approveTerminalApprovalForWorkspace?.($event)"
  />
</template>

<script setup lang="ts">
import { computed, toRef } from "vue";
import type { ApiConfigItem, PromptCommandPreset, ShellWorkspace } from "../../../types/app";
import ChatView from "./ChatView.vue";
import { useChatMessageBlocks } from "../composables/use-chat-turns";
import { useConversationViewRuntime } from "../composables/use-conversation-view-runtime";
import { useI18n } from "vue-i18n";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import type { ExclusiveChatViewSubscriptionSlot } from "../composables/exclusive-chat-view-subscription-slot";

const props = defineProps<{
  conversationId: string;
  subscriptionSlot?: ExclusiveChatViewSubscriptionSlot;
  apiConfigId: string;
  agentId: string;
  departmentId: string;
  personaName: string;
  userAlias: string;
  userAvatarUrl: string;
  assistantAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  chatModelOptions: ApiConfigItem[];
  instructionPresets: PromptCommandPreset[];
  workspaceName: string;
  workspaceRootPath: string;
  workspaces: ShellWorkspace[];
  workspaceAccess: "read_only" | "approval" | "full_access";
  currentTheme: string;
  chatInputPlaceholder: string;
  terminalApprovals?: TerminalApprovalConversationItem[];
  terminalApprovalResolving?: boolean;
  approveTerminalApproval?: (requestId: string) => void;
  denyTerminalApproval?: (requestId: string) => void;
  approveTerminalApprovalForSession?: (requestId: string) => void;
  approveTerminalApprovalForWorkspace?: (requestId: string) => void;
}>();

const { t } = useI18n();
const conversationId = toRef(props, "conversationId");
const apiConfigId = toRef(props, "apiConfigId");
const agentId = toRef(props, "agentId");
const departmentId = toRef(props, "departmentId");
const runtime = useConversationViewRuntime({
  conversationId,
  apiConfigId,
  agentId,
  departmentId,
  subscriptionSlot: props.subscriptionSlot,
  t,
});
const activeApiConfig = computed(() =>
  props.chatModelOptions.find((item) => item.id === runtime.preferredApiConfigId.value) || null,
);
const messageBlocks = useChatMessageBlocks({
  allMessages: runtime.allMessages,
  activeChatApiConfig: activeApiConfig,
  currentConversationId: conversationId,
  perfDebug: false,
  perfNow: () => performance.now(),
  taskTriggerLabels: { goal: t("config.task.fields.goal"), todo: t("config.task.fields.todo") },
});
const conversationItems = computed(() => conversationId.value ? [{
  conversationId: conversationId.value,
  title: "",
  kind: "local_unarchived" as const,
  messageCount: runtime.allMessages.value.length,
  agentId: agentId.value,
  departmentId: departmentId.value,
}] : []);

function updateChatInput(value: string) {
  runtime.chatInput.value = value;
}

function clearChatError() {
  runtime.chatErrorText.value = "";
}
</script>
