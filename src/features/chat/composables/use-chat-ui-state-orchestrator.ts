import { computed, ref, watch, type Ref } from "vue";
import type { ConfigSearchTab, ConfigSearchResult } from "../../config/search/config-search";
import type { ChatMentionTarget } from "../../../types/app";
import type { ConversationPipelineStatus } from "../../shell/composables/use-pipeline-status";
import type { searchConfigTabs } from "../../config/search/config-search";
import {
  loadStoredChatLeftPanelMode,
  loadStoredChatRightPanelMode,
  loadStoredChatSidePanelVisibility,
  loadStoredChatSidePanelWidths,
  loadStoredConversationListTab,
  normalizeChatLeftPanelMode,
  normalizeChatRightPanelMode,
  normalizeChatSidePanelWidths,
  storeChatLeftPanelMode,
  storeChatRightPanelMode,
  storeChatSidePanelVisibility,
  storeChatSidePanelWidths,
  storeConversationListTab,
  type ChatLeftPanelMode,
  type ChatRightPanelMode,
} from "./chat-ui-layout-storage";

export type ChatUiStateBindings = {
  viewMode: Ref<"chat" | "archives" | "config">;
  detachedChatWindow: Ref<boolean>;
  currentChatConversationId: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  clearConversationStatus: (conversationId: string, status?: ConversationPipelineStatus) => void;
  searchConfigTabs: typeof searchConfigTabs;
  resolveConfigLocale: () => Parameters<typeof searchConfigTabs>[1];
};

export function useChatUiStateOrchestrator(bindings: ChatUiStateBindings) {
  const configTab = ref<ConfigSearchTab>("hotkey");
  const configSearchQuery = ref("");
  const selectedChatMentions = ref<ChatMentionTarget[]>([]);
  const chatInput = ref("");

  const conversationListTab = ref<ChatLeftPanelMode>(loadStoredConversationListTab());
  const chatLeftPanelMode = ref<ChatLeftPanelMode>(loadStoredChatLeftPanelMode());
  const chatRightPanelMode = ref<ChatRightPanelMode>(loadStoredChatRightPanelMode("delegate"));
  const chatReaderDirectoryOpenRequest = ref(0);
  const sideConversationListVisible = ref(loadStoredChatSidePanelVisibility("left"));
  const toolReviewPanelOpenVisible = ref(loadStoredChatSidePanelVisibility("right"));
  const chatSidePanelWidths = ref(loadStoredChatSidePanelWidths());

  const conversationChatErrorTextMap = ref<Record<string, string>>({});
  const fallbackChatErrorText = ref("");

  function getConversationChatErrorText(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return fallbackChatErrorText.value;
    return conversationChatErrorTextMap.value[cid] || "";
  }

  function setConversationChatErrorText(conversationId: string, text: string) {
    const cid = String(conversationId || "").trim();
    const normalizedText = String(text || "");
    if (!cid) {
      fallbackChatErrorText.value = normalizedText;
      return;
    }
    const next = { ...conversationChatErrorTextMap.value };
    if (normalizedText.trim()) {
      next[cid] = normalizedText;
    } else {
      delete next[cid];
    }
    conversationChatErrorTextMap.value = next;
  }

  function clearMatchingConversationChatErrors(predicate: (text: string) => boolean) {
    let changed = false;
    const next: Record<string, string> = {};
    for (const [conversationId, text] of Object.entries(conversationChatErrorTextMap.value)) {
      if (predicate(text)) {
        changed = true;
        continue;
      }
      next[conversationId] = text;
    }
    if (changed) {
      conversationChatErrorTextMap.value = next;
    }
    if (predicate(fallbackChatErrorText.value)) {
      fallbackChatErrorText.value = "";
    }
  }

  const chatErrorText = computed({
    get: () => getConversationChatErrorText(bindings.currentChatConversationId.value),
    set: (text: string) => {
      setConversationChatErrorText(bindings.currentChatConversationId.value, text);
    },
  });

  function clearChatError() {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    setConversationChatErrorText(conversationId, "");
    bindings.clearConversationStatus(conversationId, "error");
    if (bindings.toolStatusState.value === "failed") {
      bindings.toolStatusState.value = "";
    }
  }

  function handleChatInputUpdate(value: string) {
    chatInput.value = value;
  }

  function updateConfigSearchQuery(value: string) {
    configSearchQuery.value = String(value || "");
  }

  function handleSelectConfigSearchResult(tab: ConfigSearchTab) {
    configTab.value = tab;
    configSearchQuery.value = "";
  }

  function addChatMention(value: ChatMentionTarget) {
    const agentId = String(value?.agentId || "").trim();
    const departmentId = String(value?.departmentId || "").trim();
    const agentName = String(value?.agentName || "").trim();
    if (!agentId || !departmentId || !agentName) return;
    if (selectedChatMentions.value.some((item) => item.agentId === agentId && item.departmentId === departmentId)) return;
    selectedChatMentions.value = [
      ...selectedChatMentions.value,
      {
        agentId,
        agentName,
        departmentId,
        departmentName: String(value?.departmentName || "").trim(),
        avatarUrl: String(value?.avatarUrl || "").trim() || undefined,
      },
    ];
  }

  function removeChatMention(value: string | { agentId?: string; departmentId?: string }) {
    const normalizedAgentId =
      typeof value === "string"
        ? String(value || "").trim()
        : String(value?.agentId || "").trim();
    const normalizedDepartmentId =
      typeof value === "string"
        ? ""
        : String(value?.departmentId || "").trim();
    selectedChatMentions.value = selectedChatMentions.value.filter((item) => {
      if (item.agentId !== normalizedAgentId) return true;
      if (!normalizedDepartmentId) return false;
      return item.departmentId !== normalizedDepartmentId;
    });
  }

  function handleSideConversationListVisibleChange(value: boolean) {
    sideConversationListVisible.value = value;
    storeChatSidePanelVisibility("left", value);
  }

  function handleToolReviewPanelOpenChange(value: boolean) {
    toolReviewPanelOpenVisible.value = value;
    if (value || String(bindings.currentChatConversationId.value || "").trim()) {
      storeChatSidePanelVisibility("right", value);
    }
  }

  function updateConversationListTab(value: ChatLeftPanelMode) {
    conversationListTab.value = normalizeChatLeftPanelMode(value);
    chatLeftPanelMode.value = conversationListTab.value;
    storeConversationListTab(conversationListTab.value);
    storeChatLeftPanelMode(chatLeftPanelMode.value);
  }

  function updateChatLeftPanelMode(value: ChatLeftPanelMode) {
    const nextMode = normalizeChatLeftPanelMode(value);
    chatLeftPanelMode.value = nextMode;
    conversationListTab.value = nextMode;
    storeChatLeftPanelMode(nextMode);
    storeConversationListTab(nextMode);
    if (!sideConversationListVisible.value && bindings.viewMode.value === "chat" && !bindings.detachedChatWindow.value) {
      sideConversationListVisible.value = true;
      storeChatSidePanelVisibility("left", true);
    }
  }

  function updateChatRightPanelMode(value: ChatRightPanelMode) {
    const nextMode = normalizeChatRightPanelMode(value, "delegate");
    chatRightPanelMode.value = nextMode;
    storeChatRightPanelMode(nextMode);
    if (!toolReviewPanelOpenVisible.value && bindings.viewMode.value === "chat") {
      toolReviewPanelOpenVisible.value = true;
      storeChatSidePanelVisibility("right", true);
    }
  }

  function requestChatReaderDirectoryOpenIfEmpty() {
    chatReaderDirectoryOpenRequest.value += 1;
  }

  function handleChatSidePanelWidthsChange(value: { leftWidth: number; rightWidth: number }, options?: { commit?: boolean; syncWindow?: boolean }) {
    chatSidePanelWidths.value = normalizeChatSidePanelWidths(value);
    if (options?.commit) {
      storeChatSidePanelWidths(chatSidePanelWidths.value);
    }
  }

  async function toggleSideConversationList() {
    const nextVisible = !sideConversationListVisible.value;
    sideConversationListVisible.value = nextVisible;
    storeChatSidePanelVisibility("left", nextVisible);
  }

  async function toggleToolReviewPanel() {
    const nextVisible = !toolReviewPanelOpenVisible.value;
    toolReviewPanelOpenVisible.value = nextVisible;
    storeChatSidePanelVisibility("right", nextVisible);
  }

  const configSearchResults = computed<ConfigSearchResult[]>(() => {
    if (bindings.viewMode.value !== "config") return [];
    return bindings.searchConfigTabs(configSearchQuery.value, bindings.resolveConfigLocale());
  });

  watch(
    () => String(bindings.currentChatConversationId.value || "").trim(),
    () => {
      selectedChatMentions.value = [];
    },
  );

  return {
    configTab,
    configSearchQuery,
    configSearchResults,
    selectedChatMentions,
    chatInput,
    conversationListTab,
    chatLeftPanelMode,
    chatRightPanelMode,
    chatReaderDirectoryOpenRequest,
    sideConversationListVisible,
    toolReviewPanelOpenVisible,
    chatSidePanelWidths,
    chatErrorText,
    handleChatInputUpdate,
    updateConfigSearchQuery,
    handleSelectConfigSearchResult,
    addChatMention,
    removeChatMention,
    handleSideConversationListVisibleChange,
    handleToolReviewPanelOpenChange,
    updateConversationListTab,
    updateChatLeftPanelMode,
    updateChatRightPanelMode,
    requestChatReaderDirectoryOpenIfEmpty,
    handleChatSidePanelWidthsChange,
    toggleSideConversationList,
    toggleToolReviewPanel,
    setConversationChatErrorText,
    clearMatchingConversationChatErrors,
    clearChatError,
  };
}
