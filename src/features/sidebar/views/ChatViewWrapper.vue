<template>
  <ChatView
    :user-alias="userAlias"
    :persona-name="assistantName"
    :user-avatar-url="userAvatarUrl"
    :assistant-avatar-url="assistantAvatarUrl"
    :persona-name-map="personaNameMap"
    :persona-avatar-url-map="personaAvatarUrlMap"
    :mention-entries="[]"
    :selected-mentions="[]"
    latest-user-text=""
    :latest-user-images="[]"
    latest-assistant-text=""
    latest-reasoning-standard-text=""
    latest-reasoning-inline-text=""
    :frontend-round-phase="busy ? 'streaming' : 'idle'"
    tool-status-text=""
    tool-status-state=""
    :stream-tool-calls="[]"
    chat-error-text=""
    :clipboard-images="[]"
    :queued-attachment-notices="[]"
    :chat-input="input"
    :instruction-presets="[]"
    chat-input-placeholder="输入消息"
    :can-record="false"
    :recording="false"
    :recording-ms="0"
    :transcribing="false"
    record-hotkey=""
    :selected-chat-model-id="selectedChatModelId"
    :tool-review-refresh-tick="0"
    :terminal-approvals="[]"
    :terminal-approval-resolving="false"
    :chat-model-options="chatModelOptions"
    :workspace-access="workspaceAccess"
    :plan-mode-enabled="false"
    :chat-usage-percent="0"
    force-archive-tip=""
    :media-drag-active="false"
    :chatting="busy"
    :forcing-archive="false"
    :compacting-conversation="false"
    :conversation-busy="false"
    :frozen="false"
    :message-blocks="visibleMessageBlocks"
    :has-more-history="hasPrevBlock"
    :loading-older-history="false"
    :latest-own-message-align-request="0"
    :conversation-scroll-to-bottom-request="scrollToBottomRequest"
    current-workspace-name=""
    current-workspace-root-path=""
    :workspaces="[]"
    current-department-id=""
    :active-conversation-id="activeConversationId"
    :current-todos="[]"
    :supervision-active="false"
    supervision-title=""
    :supervision-dialog-open="false"
    :supervision-task-saving="false"
    supervision-task-error=""
    :active-supervision-task="null"
    :recent-supervision-task-history="[]"
    :unarchived-conversation-items="[]"
    :conversation-items="[]"
    :create-conversation-department-options="[]"
    :delegate-department-ids="[]"
    default-create-conversation-department-id=""
    :ide-context-groups="[]"
    :attached-ide-context-references="[]"
    :current-theme="vscodeTheme"
    :detached-chat-window="true"
    :sidebar-mode="true"
    :side-conversation-list-visible="false"
    :initial-tool-review-panel-open="false"
    conversation-list-tab="local"
    chat-left-panel-mode="local"
    chat-right-panel-mode="reader"
    @update:chat-input="$emit('update:input', $event)"
    @send-chat="$emit('send')"
    @stop-chat="$emit('stop')"
    @load-older-history="$emit('loadPrevBlock')"
    @clear-chat-error="noop"
    @reached-bottom="noop"
    @jump-to-conversation-bottom="noop"
    @add-mention="noop"
    @remove-mention="noop"
    @side-conversation-list-visible-change="noop"
    @tool-review-panel-open-change="noop"
    @side-panel-widths-change="noop"
    @side-panel-widths-commit="noop"
    @update:conversation-list-tab="noop"
    @update:chat-left-panel-mode="noop"
    @update:chat-right-panel-mode="noop"
    @remove-clipboard-image="noop"
    @remove-queued-attachment-notice="noop"
    @start-recording="noop"
    @stop-recording="noop"
    @pick-attachments="noop"
    @update:selected-chat-model-id="$emit('update:selectedChatModelId', $event)"
    @update-workspace-access="$emit('updateWorkspaceAccess', $event)"
    @update:plan-mode-enabled="noop"
    @force-archive="noop"
    @recall-turn="$emit('recallTurn', $event)"
    @regenerate-turn="noop"
    @confirm-plan="noop"
    @lock-workspace="noop"
    @open-supervision-task="noop"
    @detach-conversation="noop"
    @close-supervision-task="noop"
    @save-supervision-task="noop"
    @switch-conversation="noop"
    @rename-conversation="noop"
    @toggle-pin-conversation="noop"
    @archive-conversation="noop"
    @delete-conversation="noop"
    @create-conversation="noop"
    @refresh-tool-review-message="noop"
    @attach-tool-review-report="noop"
    @selection-action-copy="noop"
    @selection-action-copy-error="noop"
    @selection-action-branch="noop"
    @selection-action-forward="noop"
    @selection-action-delegate="noop"
    @selection-action-share="noop"
    @approve-terminal-approval="noop"
    @deny-terminal-approval="noop"
    @open-sidebar-file-reference="openSidebarFileReference"
  />
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import type { ApiConfigItem, ChatMessage } from "../../../types/app";
import ChatView from "../../chat/views/ChatView.vue";
import { useChatMessageBlocks } from "../../chat/composables/use-chat-turns";

type VsCodeApi = { postMessage: (message: unknown) => void };

let cachedVsCodeApi: VsCodeApi | null | undefined;

function getVsCodeApi(): VsCodeApi | null {
  if (cachedVsCodeApi !== undefined) return cachedVsCodeApi;
  const bridgeWindow = window as Window & { acquireVsCodeApi?: () => VsCodeApi };
  try {
    cachedVsCodeApi = typeof bridgeWindow.acquireVsCodeApi === "function" ? bridgeWindow.acquireVsCodeApi() : null;
  } catch {
    cachedVsCodeApi = null;
  }
  return cachedVsCodeApi;
}

const props = defineProps<{
  activeConversationId: string;
  activeAgentId: string;
  persona: {
    userAlias?: string;
    userAvatarUrl?: string;
    assistantName?: string;
    assistantAvatarUrl?: string;
    personaNameMap?: Record<string, string>;
    personaAvatarUrlMap?: Record<string, string>;
  };
  selectedChatModelId: string;
  chatModelOptions: ApiConfigItem[];
  workspaceAccess: "read_only" | "approval" | "full_access" | "";
  input: string;
  messages: ChatMessage[];
  streamingText: string;
  streamingReasoningStandard: string;
  streamingReasoningInline: string;
  busy: boolean;
  hasPrevBlock: boolean;
}>();

defineEmits<{
  "update:input": [value: string];
  send: [];
  stop: [];
  loadPrevBlock: [];
  "update:selectedChatModelId": [value: string];
  updateWorkspaceAccess: [value: "read_only" | "approval" | "full_access"];
  recallTurn: [payload: { turnId: string }];
}>();

const allMessages = shallowRef<ChatMessage[]>([]);
const activeChatApiConfig = computed<ApiConfigItem | null>(
  () => props.chatModelOptions.find((item) => item.id === props.selectedChatModelId) || null,
);
const userAlias = computed(() => String(props.persona?.userAlias || "我").trim() || "我");
const userAvatarUrl = computed(() => String(props.persona?.userAvatarUrl || "").trim());
const assistantName = computed(() => String(props.persona?.assistantName || "PAI").trim() || "PAI");
const assistantAvatarUrl = computed(() => String(props.persona?.assistantAvatarUrl || "").trim());
const personaNameMap = computed<Record<string, string>>(() => ({
  "user-persona": userAlias.value,
  ...(props.persona?.personaNameMap || {}),
  ...(props.activeAgentId ? { [props.activeAgentId]: assistantName.value } : {}),
}));
const personaAvatarUrlMap = computed<Record<string, string>>(() => {
  const next = { ...(props.persona?.personaAvatarUrlMap || {}) };
  if (props.activeAgentId && assistantAvatarUrl.value) next[props.activeAgentId] = assistantAvatarUrl.value;
  return next;
});
const vscodeTheme = ref(resolveVsCodeTheme());
const scrollToBottomRequest = ref(0);
const streamingDraftCreatedAt = ref("");

function resolveVsCodeTheme(): "dark" | "corporate" {
  if (document.body.classList.contains("vscode-dark") || document.body.classList.contains("vscode-high-contrast")) {
    return "dark";
  }
  return "corporate";
}

let themeObserver: MutationObserver | null = null;

onMounted(() => {
  themeObserver = new MutationObserver(() => {
    vscodeTheme.value = resolveVsCodeTheme();
  });
  themeObserver.observe(document.body, { attributes: true, attributeFilter: ["class"] });
});

onBeforeUnmount(() => {
  themeObserver?.disconnect();
  themeObserver = null;
});

watch(
  () => [props.messages, props.streamingText, props.streamingReasoningStandard, props.streamingReasoningInline] as const,
  () => {
    const next = [...props.messages];
    const text = String(props.streamingText || "");
    const reasoningStandard = String(props.streamingReasoningStandard || "");
    const reasoningInline = String(props.streamingReasoningInline || "");
    if (text.trim() || reasoningStandard.trim() || reasoningInline.trim()) {
      if (!streamingDraftCreatedAt.value) {
        streamingDraftCreatedAt.value = new Date().toISOString();
      }
      next.push({
        id: `sidebar-stream-${props.activeConversationId || "conversation"}`,
        role: "assistant",
        createdAt: streamingDraftCreatedAt.value,
        speakerAgentId: props.activeAgentId || undefined,
        parts: [{ type: "text", text }],
        extraTextBlocks: [],
        providerMeta: {
          _streaming: true,
          _streamSegments: [text],
          reasoningStandard,
          reasoningInline,
        },
        toolCall: [],
      });
    } else {
      streamingDraftCreatedAt.value = "";
    }
    allMessages.value = next;
  },
  { immediate: true, deep: true },
);

watch(
  () => props.activeConversationId,
  () => {
    streamingDraftCreatedAt.value = "";
  },
);

const { visibleMessageBlocks } = useChatMessageBlocks({
  allMessages,
  activeChatApiConfig,
  perfDebug: false,
  perfNow: () => performance.now(),
});

function noop() {}

function openSidebarFileReference(href: string) {
  const normalizedHref = String(href || "").trim();
  if (!normalizedHref) return;
  const vscodeApi = getVsCodeApi();
  if (vscodeApi) {
    vscodeApi.postMessage({ type: "pai-open-file", href: normalizedHref });
    return;
  }
  window.parent.postMessage({ type: "pai-open-file", href: normalizedHref }, "*");
}
</script>
