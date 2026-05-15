<template>
  <SidebarLayout
    :view="view"
    :connected="transport.connected.value"
    :connecting="transport.connecting.value"
    :error-text="transport.errorText.value"
    :active-title="activeTitle"
    :active-conversation-id="activeConversationId"
    :compacting="compacting"
    @show-list="view = 'list'"
    @show-chat="view = 'chat'"
    @new-conversation="openCreateConversationDialog"
    @open-settings="openSettings"
    @compact-conversation="openCompactionDialog"
    @reconnect="bootstrap"
  >
    <ConversationListView
      v-if="view === 'list'"
      :items="conversations"
      :active-conversation-id="activeConversationId"
      :persona="listPersona"
      @select="openConversation"
    />
    <ChatViewWrapper
      v-else
      v-model:input="inputText"
      :active-conversation-id="activeConversationId"
      :active-agent-id="activeAgentId"
      :persona="persona"
      :selected-chat-model-id="selectedChatModelId"
      :chat-model-options="chatModelOptions"
      :workspace-access="workspaceAccess"
      :messages="messages"
      :streaming-text="streamingText"
      :streaming-reasoning-standard="streamingReasoningStandard"
      :streaming-reasoning-inline="streamingReasoningInline"
      :busy="busy"
      :has-prev-block="hasPrevBlock"
      @send="send"
      @stop="stop"
      @load-prev-block="loadPrevBlock"
      @update:selected-chat-model-id="selectChatModel"
      @update-workspace-access="selectWorkspaceAccess"
      @recall-turn="recallTurn"
    />
    <SidebarCompactionDialog
      :open="compactionDialogOpen"
      :loading="compactionPreviewLoading"
      :running="compacting"
      :preview="compactionPreview"
      :error-text="compactionErrorText"
      @close="closeCompactionDialog"
      @confirm="confirmCompaction"
    />
    <CreateConversationDialog
      :open="createConversationDialogOpen"
      :creating="creatingConversation"
      :departments="createConversationDepartmentOptions"
      :default-department-id="defaultCreateConversationDepartmentId"
      :error-text="createConversationErrorText"
      @close="closeCreateConversationDialog"
      @confirm="createConversation"
    />
  </SidebarLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { ApiConfigItem, ChatMessage } from "../../types/app";
import { removeBinaryPlaceholders, messageText } from "../../utils/chat-message";
import SidebarLayout from "./layouts/SidebarLayout.vue";
import ConversationListView from "./views/ConversationListView.vue";
import ChatViewWrapper from "./views/ChatViewWrapper.vue";
import SidebarCompactionDialog from "./views/SidebarCompactionDialog.vue";
import CreateConversationDialog, { type SidebarCreateDepartmentOption } from "./views/CreateConversationDialog.vue";
import { useWsTransport, type SidebarBridgeConfig } from "./composables/use-ws-transport";

type ConversationSummary = {
  conversationId: string;
  title: string;
  summaryTitle?: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount?: number;
  unreadCount?: number;
  agentId?: string;
  departmentId?: string;
  departmentName?: string;
  runtimeState?: string;
  detachedWindowOpen?: boolean;
  detachedWindowLabel?: string;
  previewMessages?: Array<{
    messageId: string;
    role: string;
    speakerAgentId?: string;
    createdAt?: string;
    textPreview?: string;
    hasImage?: boolean;
    hasPdf?: boolean;
    hasAudio?: boolean;
    hasAttachment?: boolean;
  }>;
};

type OpenConversationResult = {
  conversationId: string;
  title: string;
  agentId?: string;
  departmentId?: string;
  messages: ChatMessage[];
  persona?: SidebarPersonaPayload;
  model?: SidebarModelPayload;
};

type SidebarWorkspacePermission = {
  access?: "read_only" | "approval" | "full_access" | "";
  workspaceName?: string;
  rootPath?: string;
};

type RewindConversationResult = {
  conversationId: string;
  removedCount: number;
  remainingCount: number;
  recalledUserMessage?: ChatMessage;
  conversation?: OpenConversationResult;
};

type BlockPageResult = {
  selectedBlockId: number;
  messages: ChatMessage[];
  hasPrevBlock: boolean;
  hasNextBlock: boolean;
};

type CompactionPreviewResult = {
  conversationId: string;
  canCompact: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
};

type SidebarPersonaPayload = {
  userAlias?: string;
  userAvatarUrl?: string;
  assistantName?: string;
  assistantAvatarUrl?: string;
  personaNameMap?: Record<string, string>;
  personaAvatarUrlMap?: Record<string, string>;
};

type SidebarModelPayload = {
  selectedChatModelId?: string;
  chatModelOptions?: ApiConfigItem[];
};

type CreateConversationOptionsResult = {
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
};

type DiscoveryPayload = {
  chatUrl?: string;
  bridgeUrl?: string;
  url?: string;
  token?: string;
  workspaceRoots?: Array<{ path?: string; name?: string }>;
};

const transport = useWsTransport();
const conversations = ref<ConversationSummary[]>([]);
const activeConversationId = ref("");
const activeTitle = ref("");
const activeAgentId = ref("");
const persona = ref<SidebarPersonaPayload>({});
const listPersona = ref<SidebarPersonaPayload>({});
const selectedChatModelId = ref("");
const chatModelOptions = ref<ApiConfigItem[]>([]);
const workspaceAccess = ref<"read_only" | "approval" | "full_access" | "">("approval");
const vscodeWorkspaceRoots = ref<Array<{ path: string; name: string }>>([]);
const messages = ref<ChatMessage[]>([]);
const inputText = ref("");
const streamingText = ref("");
const streamingReasoningStandard = ref("");
const streamingReasoningInline = ref("");
const busy = ref(false);
const compacting = ref(false);
const compactionDialogOpen = ref(false);
const compactionPreviewLoading = ref(false);
const compactionPreview = ref<CompactionPreviewResult | null>(null);
const compactionErrorText = ref("");
const createConversationDialogOpen = ref(false);
const creatingConversation = ref(false);
const createConversationDepartmentOptions = ref<SidebarCreateDepartmentOption[]>([]);
const defaultCreateConversationDepartmentId = ref("");
const createConversationErrorText = ref("");
const selectedBlockId = ref<number | null>(null);
const hasPrevBlock = ref(false);
const view = ref<"list" | "chat">("chat");

const activeSummary = computed(() => conversations.value.find((item) => item.conversationId === activeConversationId.value));

function normalizeDiscovery(payload: DiscoveryPayload): SidebarBridgeConfig | null {
  const chatUrl = String(payload.chatUrl || "").trim() || String(payload.url || "").trim().replace(/\/ide-context$/, "/chat");
  const token = String(payload.token || "").trim();
  if (!chatUrl || !token) return null;
  return { chatUrl, token };
}

async function loadDiscovery(): Promise<SidebarBridgeConfig | null> {
  const injected = (window as unknown as { __PAI_SIDEBAR_BRIDGE__?: DiscoveryPayload }).__PAI_SIDEBAR_BRIDGE__;
  if (injected) {
    applyWorkspaceRoots(injected.workspaceRoots);
    return normalizeDiscovery(injected);
  }
  const params = new URLSearchParams(window.location.search);
  const fromQuery = normalizeDiscovery({
    chatUrl: params.get("chatUrl") || undefined,
    token: params.get("token") || undefined,
  });
  if (fromQuery) return fromQuery;
  return null;
}

function applyWorkspaceRoots(rawRoots: DiscoveryPayload["workspaceRoots"]) {
  vscodeWorkspaceRoots.value = (Array.isArray(rawRoots) ? rawRoots : [])
    .map((item) => ({
      path: String(item?.path || "").trim(),
      name: String(item?.name || "").trim(),
    }))
    .filter((item) => item.path);
}

async function refreshList() {
  const result = await transport.request<{ conversations: ConversationSummary[]; persona?: SidebarPersonaPayload }>("conversation.list");
  conversations.value = Array.isArray(result.conversations) ? result.conversations : [];
  if (result.persona) listPersona.value = result.persona;
  if (!activeConversationId.value && conversations.value[0]) {
    await openConversation(conversations.value[0].conversationId);
  } else if (activeConversationId.value) {
    activeTitle.value = activeSummary.value?.title || activeTitle.value;
  }
}

async function loadCreateConversationOptions() {
  const result = await transport.request<CreateConversationOptionsResult>("conversation.createOptions", {});
  createConversationDepartmentOptions.value = Array.isArray(result.departments) ? result.departments : [];
  defaultCreateConversationDepartmentId.value = String(result.defaultDepartmentId || "").trim()
    || createConversationDepartmentOptions.value[0]?.id
    || "";
}

async function openConversation(conversationId: string) {
  const result = await transport.request<OpenConversationResult>("conversation.open", { conversationId });
  activeConversationId.value = result.conversationId;
  activeTitle.value = result.title || activeSummary.value?.title || "PAI";
  activeAgentId.value = String(result.agentId || "").trim();
  persona.value = result.persona || {};
  if (result.persona) listPersona.value = result.persona;
  applyModelPayload(result.model || {});
  await refreshWorkspacePermission();
  messages.value = Array.isArray(result.messages) ? result.messages : [];
  streamingText.value = "";
  streamingReasoningStandard.value = "";
  streamingReasoningInline.value = "";
  selectedBlockId.value = null;
  hasPrevBlock.value = true;
  view.value = "chat";
}

async function refreshWorkspacePermission() {
  if (!activeConversationId.value) return;
  try {
    const result = await transport.request<SidebarWorkspacePermission>("workspace.permission", {
      conversationId: activeConversationId.value,
    });
    applyWorkspacePermission(result);
  } catch {
    workspaceAccess.value = "approval";
  }
}

function applyWorkspacePermission(payload: SidebarWorkspacePermission) {
  const access = String(payload.access || "").trim();
  workspaceAccess.value = access === "read_only" || access === "full_access" ? access : "approval";
}

async function loadPrevBlock() {
  if (!activeConversationId.value || !hasPrevBlock.value) return;
  const result = await transport.request<BlockPageResult>("conversation.blockPage", {
    conversationId: activeConversationId.value,
    blockId: selectedBlockId.value || undefined,
  });
  selectedBlockId.value = result.selectedBlockId;
  hasPrevBlock.value = result.hasPrevBlock;
  const existingIds = new Set(messages.value.map((item) => item.id));
  const previous = (result.messages || []).filter((item) => !existingIds.has(item.id));
  messages.value = [...previous, ...messages.value];
}

async function openCreateConversationDialog() {
  createConversationErrorText.value = "";
  try {
    await loadCreateConversationOptions();
    createConversationDialogOpen.value = true;
  } catch (error) {
    createConversationErrorText.value = String(error || "加载部门列表失败");
    createConversationDialogOpen.value = true;
  }
}

function closeCreateConversationDialog() {
  if (creatingConversation.value) return;
  createConversationDialogOpen.value = false;
  createConversationErrorText.value = "";
}

async function createConversation(input: { title?: string; departmentId: string }) {
  const departmentId = String(input.departmentId || "").trim();
  if (!departmentId || creatingConversation.value) return;
  creatingConversation.value = true;
  createConversationErrorText.value = "";
  try {
    const result = await transport.request<{ conversationId: string; conversation?: OpenConversationResult }>("conversation.create", {
      title: input.title,
      departmentId,
    });
    await refreshList();
    await openConversation(result.conversationId);
    createConversationDialogOpen.value = false;
  } catch (error) {
    createConversationErrorText.value = String(error || "创建会话失败");
  } finally {
    creatingConversation.value = false;
  }
}

async function openSettings() {
  try {
    await transport.request("settings.open", {});
  } catch (error) {
    transport.errorText.value = String(error || "打开设置失败");
  }
}

function applyModelPayload(payload: SidebarModelPayload) {
  selectedChatModelId.value = String(payload.selectedChatModelId || "").trim();
  chatModelOptions.value = Array.isArray(payload.chatModelOptions) ? payload.chatModelOptions : [];
}

async function selectChatModel(apiConfigId: string) {
  const nextId = String(apiConfigId || "").trim();
  if (!nextId || !activeConversationId.value || nextId === selectedChatModelId.value) return;
  const previousId = selectedChatModelId.value;
  selectedChatModelId.value = nextId;
  try {
    const result = await transport.request<SidebarModelPayload>("model.select", {
      conversationId: activeConversationId.value,
      apiConfigId: nextId,
    });
    applyModelPayload(result);
  } catch (error) {
    selectedChatModelId.value = previousId;
    transport.errorText.value = String(error || "切换模型失败");
  }
}

async function selectWorkspaceAccess(access: "read_only" | "approval" | "full_access") {
  if (!activeConversationId.value || workspaceAccess.value === access) return;
  const previous = workspaceAccess.value;
  workspaceAccess.value = access;
  const root = vscodeWorkspaceRoots.value[0];
  try {
    const result = await transport.request<SidebarWorkspacePermission>("workspace.permission.select", {
      conversationId: activeConversationId.value,
      access,
      workspacePath: root?.path || undefined,
      workspaceName: root?.name || undefined,
    });
    applyWorkspacePermission(result);
  } catch (error) {
    workspaceAccess.value = previous;
    transport.errorText.value = String(error || "切换权限失败");
  }
}

async function openCompactionDialog() {
  if (!activeConversationId.value || compacting.value) return;
  compactionDialogOpen.value = true;
  compactionPreviewLoading.value = true;
  compactionPreview.value = null;
  compactionErrorText.value = "";
  try {
    compactionPreview.value = await transport.request<CompactionPreviewResult>("conversation.compactPreview", {
      conversationId: activeConversationId.value,
    });
  } catch (error) {
    compactionErrorText.value = String(error || "加载压缩预览失败");
  } finally {
    compactionPreviewLoading.value = false;
  }
}

function closeCompactionDialog() {
  if (compacting.value) return;
  compactionDialogOpen.value = false;
  compactionPreview.value = null;
  compactionErrorText.value = "";
}

async function confirmCompaction() {
  if (!activeConversationId.value || compacting.value || !compactionPreview.value?.canCompact) return;
  compacting.value = true;
  compactionErrorText.value = "";
  try {
    const result = await transport.request<{ compactionMessage?: ChatMessage }>("conversation.compact", {
      conversationId: activeConversationId.value,
    });
    if (result.compactionMessage) appendMessages({ conversationId: activeConversationId.value, message: result.compactionMessage });
    await refreshList();
    await openConversation(activeConversationId.value);
    compactionDialogOpen.value = false;
  } catch (error) {
    compactionErrorText.value = String(error || "压缩失败");
  } finally {
    compacting.value = false;
  }
}

async function send() {
  const text = inputText.value.trim();
  if (!text || !activeConversationId.value || busy.value) return;
  inputText.value = "";
  busy.value = true;
  try {
    await transport.request("chat.send", { conversationId: activeConversationId.value, text });
  } catch (error) {
    busy.value = false;
    streamingText.value = "";
    streamingReasoningStandard.value = "";
    streamingReasoningInline.value = "";
    if (!inputText.value.trim()) inputText.value = text;
    transport.errorText.value = String(error || "发送失败");
  }
}

async function stop() {
  if (!activeConversationId.value) return;
  await transport.request("chat.stop", {
    conversationId: activeConversationId.value,
    partialAssistantText: streamingText.value,
    partialReasoningStandard: streamingReasoningStandard.value,
    partialReasoningInline: streamingReasoningInline.value,
  });
  busy.value = false;
}

function resolveRewindTargetUserMessage(turnId: string): { targetUserMessageId: string; keepCount: number } | null {
  const targetId = String(turnId || "").trim();
  if (!targetId) return null;
  const index = messages.value.findIndex((item) => item.id === targetId);
  if (index < 0) return null;
  if (String(messages.value[index]?.role || "").trim() === "user") {
    return { targetUserMessageId: targetId, keepCount: index };
  }
  for (let i = index - 1; i >= 0; i -= 1) {
    if (String(messages.value[i]?.role || "").trim() === "user") {
      return { targetUserMessageId: String(messages.value[i]?.id || "").trim(), keepCount: i };
    }
  }
  return null;
}

async function recallTurn(payload: { turnId: string }) {
  if (!activeConversationId.value || compacting.value) return;
  const target = resolveRewindTargetUserMessage(payload.turnId);
  if (!target?.targetUserMessageId) {
    transport.errorText.value = "撤回失败：未找到可撤回的用户消息";
    return;
  }
  if (busy.value) await stop();
  streamingText.value = "";
  streamingReasoningStandard.value = "";
  streamingReasoningInline.value = "";
  try {
    const result = await transport.request<RewindConversationResult>("conversation.rewind", {
      conversationId: activeConversationId.value,
      agentId: activeAgentId.value,
      messageId: target.targetUserMessageId,
      undoApplyPatch: false,
    });
    const recalled = result.recalledUserMessage || messages.value[target.keepCount];
    inputText.value = recalled ? removeBinaryPlaceholders(messageText(recalled)) : inputText.value;
    if (result.conversation) {
      activeConversationId.value = result.conversation.conversationId;
      activeTitle.value = result.conversation.title || activeTitle.value;
      messages.value = Array.isArray(result.conversation.messages) ? result.conversation.messages : messages.value.slice(0, target.keepCount);
      persona.value = result.conversation.persona || persona.value;
      applyModelPayload(result.conversation.model || {});
    } else {
      messages.value = messages.value.slice(0, target.keepCount);
    }
    selectedBlockId.value = null;
    hasPrevBlock.value = true;
    await refreshList();
  } catch (error) {
    transport.errorText.value = String(error || "撤回失败");
  }
}

function appendMessages(next: unknown) {
  const payload = next as { conversationId?: string; messages?: ChatMessage[]; message?: ChatMessage };
  if (payload.conversationId && payload.conversationId !== activeConversationId.value) return;
  const incoming = payload.messages || (payload.message ? [payload.message] : []);
  if (!incoming.length) return;
  const existingIds = new Set(messages.value.map((item) => item.id));
  messages.value = [...messages.value, ...incoming.filter((item) => !existingIds.has(item.id))];
}

function registerNotifications() {
  transport.onNotification("conversation.overviewUpdated", (payload) => {
    const value = payload as { unarchivedConversations?: ConversationSummary[] };
    if (Array.isArray(value.unarchivedConversations)) conversations.value = value.unarchivedConversations;
  });
  transport.onNotification("conversation.messageAppended", appendMessages);
  transport.onNotification("chat.historyFlushed", appendMessages);
  transport.onNotification("chat.roundStarted", (payload) => {
    const value = payload as { conversationId?: string };
    if (value.conversationId === activeConversationId.value) {
      busy.value = true;
      streamingText.value = "";
      streamingReasoningStandard.value = "";
      streamingReasoningInline.value = "";
    }
  });
  transport.onNotification("chat.assistantDelta", (payload) => {
    const value = payload as { conversationId?: string; event?: { delta?: string; kind?: string } };
    if (value.conversationId !== activeConversationId.value) return;
    const delta = String(value.event?.delta || "");
    const kind = String(value.event?.kind || "").trim();
    if (!delta) return;
    if (kind === "reasoning_standard") streamingReasoningStandard.value += delta;
    else if (kind === "reasoning_inline") streamingReasoningInline.value += delta;
    else streamingText.value += delta;
  });
  transport.onNotification("chat.roundFinished", (payload) => {
    const value = payload as { conversationId?: string; assistantMessage?: ChatMessage };
    if (value.conversationId !== activeConversationId.value) return;
    busy.value = false;
    streamingText.value = "";
    streamingReasoningStandard.value = "";
    streamingReasoningInline.value = "";
    if (value.assistantMessage) appendMessages({ conversationId: value.conversationId, message: value.assistantMessage });
  });
}

async function bootstrap() {
  const config = await loadDiscovery();
  if (!config) {
    transport.errorText.value = "PAI 未运行";
    return;
  }
  await transport.connect(config);
  if (transport.connected.value) await refreshList();
}

onMounted(() => {
  registerNotifications();
  transport.onAuthRefreshNeeded(() => {
    window.parent.postMessage({ type: "pai-refresh-discovery" }, "*");
  });
  window.addEventListener("message", (event) => {
    const data = event.data as { type?: string; discovery?: DiscoveryPayload };
    if (data?.type === "pai-discovery" && data.discovery) {
      applyWorkspaceRoots(data.discovery.workspaceRoots);
      const config = normalizeDiscovery(data.discovery);
      if (config) void transport.connect(config).then(refreshList);
    }
  });
  void bootstrap();
});
</script>
