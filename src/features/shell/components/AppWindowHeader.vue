<template>
  <div
    class="min-h-10 h-10 shrink-0 relative z-40 overflow-visible select-none"
    :class="viewMode === 'chat' ? 'grid items-center bg-base-200 border-b border-base-300' : 'grid grid-cols-[1fr_auto_1fr] items-center bg-base-200 border-b border-base-300 px-2'"
    :style="viewMode === 'chat' ? chatHeaderGridStyle : undefined"
  >
    <div
      v-if="viewMode !== 'chat'"
      data-tauri-drag-region
      class="absolute inset-0"
      aria-hidden="true"
    ></div>
    <div
      v-else
      data-tauri-drag-region
      class="absolute inset-0 z-10"
      aria-hidden="true"
    ></div>
    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 flex h-full min-w-0 items-center gap-1 px-2"
    >
      <div v-if="!detachedChatWindow" class="relative" @mousedown.stop>
        <div class="indicator">
          <span
            v-if="conversationUnreadTotal > 0"
            class="indicator-item indicator-top indicator-start z-10 h-2.5 w-2.5 -translate-x-0.5 -translate-y-0.5 rounded-full bg-error"
            aria-hidden="true"
          ></span>
          <button
            v-if="sideConversationListVisible"
            class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
            :class="sideConversationListVisible ? 'btn-active' : ''"
            :title="t('chat.conversationList')"
            @click.stop="emit('toggle-side-conversation-list')"
          >
            <PanelLeftOpen class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

    </div>

    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 grid h-full min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-1 px-2"
    >
      <div class="relative z-40 flex min-w-0 items-center gap-1" @mousedown.stop>
        <div
          v-if="!sideConversationListVisible && !detachedChatWindow"
          class="relative"
          @mouseenter="handleHoverSidebarEnter"
          @mouseleave="handleHoverSidebarLeave"
        >
          <button
            class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
            :title="t('chat.conversationList')"
            @click.stop="emit('toggle-side-conversation-list')"
          >
            <PanelLeftClose class="h-3.5 w-3.5" />
          </button>
          <div
            v-if="hoverSidebarOpen"
            class="absolute left-0 top-full z-50 mt-1 h-[90vh] max-h-[90vh] w-72 rounded-box border border-base-300 bg-base-200 shadow-xl overflow-hidden"
            @mouseenter="handleHoverSidebarEnter"
            @mouseleave="handleHoverSidebarLeave"
          >
            <ChatConversationSidebar
              :items="conversationItems"
              :active-conversation-id="activeConversationId"
              :user-alias="userAlias"
              :user-avatar-url="userAvatarUrl"
              :persona-name-map="personaNameMap"
              :persona-avatar-url-map="personaAvatarUrlMap"
              :active-tab="conversationListTab"
              @select="handleHoverConversationSelect"
              @rename="$emit('rename-conversation', $event)"
              @toggle-pin-conversation="$emit('toggle-pin-conversation', $event)"
              @archive-conversation="$emit('archive-conversation', $event)"
              @delete-conversation="$emit('delete-conversation', $event)"
              @update:active-tab="$emit('update:conversation-list-tab', $event)"
            />
          </div>
        </div>
      </div>

      <div
        data-tauri-drag-region
        class="relative z-30 flex min-w-0 flex-1 self-stretch items-center justify-center gap-1 px-2"
        :title="combinedTitleTooltip"
      >
        <button
          v-if="!detachedChatWindow"
          class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
          :title="t('chat.newConversation')"
          @click.stop="handleCreateConversation"
        >
          <SquarePen class="h-4 w-4" />
        </button>
        <span class="pointer-events-none truncate text-sm font-semibold text-base-content">{{ combinedTitle }}</span>
        <button
          class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
          :disabled="forcingArchive || chatting"
          :title="`${t('chat.contextUsageTitle', { percent: normalizedChatUsagePercent })} · ${forceArchiveTip}`"
          @click.stop="$emit('force-archive')"
        >
          <svg
            class="h-5 w-5 -rotate-90"
            viewBox="0 0 36 36"
          >
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              stroke-width="4"
              class="opacity-20"
            />
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              stroke-width="4"
              stroke-linecap="round"
              :stroke-dasharray="circumference"
              :stroke-dashoffset="strokeDashoffset"
            />
          </svg>
        </button>
      </div>

      <div class="relative z-40 flex min-w-0 items-center justify-end gap-1" @mousedown.stop>
        <button
          v-if="!toolReviewPanelOpenVisible"
          type="button"
          class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
          :title="t('chat.toolReview.title')"
          @click.stop="emit('toggle-tool-review-panel')"
        >
          <PanelRightClose class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 flex h-full min-w-0 flex-nowrap items-center justify-end gap-1 px-2"
    >
      <div v-if="toolReviewPanelOpenVisible" role="tablist" class="tabs tabs-border min-w-0 shrink-0" @mousedown.stop>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-2"
          :class="chatRightPanelMode === 'reader' ? 'tab-active font-semibold' : ''"
          @click.stop="emit('update:chat-right-panel-mode', 'reader')"
        >
          {{ t("chat.readerPanelTab") }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-2"
          :class="chatRightPanelMode !== 'reader' ? 'tab-active font-semibold' : ''"
          @click.stop="emit('update:chat-right-panel-mode', 'delegate')"
        >
          {{ t("chat.delegatePanelTab") }}
        </button>
      </div>

      <button
        v-if="toolReviewPanelOpenVisible"
        type="button"
        class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
        :class="toolReviewPanelOpenVisible ? 'btn-active' : ''"
        :title="t('chat.toolReview.title')"
        @mousedown.stop
        @click.stop="emit('toggle-tool-review-panel')"
      >
        <PanelRightOpen class="h-3.5 w-3.5" />
      </button>

      <button
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @mousedown.stop
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @mousedown.stop
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-sm btn-ghost hover:bg-error"
        :title="closeTitle || t('common.close')"
        @mousedown.stop
        @click.stop="$emit('close-window')"
        :disabled="!windowReady"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>

    <div v-if="viewMode !== 'chat'" class="relative z-10 min-w-0 justify-self-start flex items-center gap-2" @mousedown.stop>
      <button
        v-if="viewMode === 'config'"
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 px-0"
        :title="t('about.changelog')"
        @click.stop="openChangelogDialog"
      >
        <ScrollText class="h-3.5 w-3.5" />
      </button>

      <button
        v-if="viewMode === 'config' && showUpdateToLatestButton"
        class="btn btn-success btn-sm h-8 min-h-8 gap-2 px-3 relative shadow-sm"
        :title="updateToLatestTitle || ''"
        @click.stop="$emit('update-to-latest')"
      >
        <span
          v-if="hasAvailableUpdate && !checkingUpdate"
          class="absolute right-1.5 top-1.5 h-2 w-2 rounded-full bg-error"
          aria-hidden="true"
        ></span>
        <span v-if="checkingUpdate" class="loading loading-spinner loading-xs"></span>
        <Download v-else class="h-3.5 w-3.5" />
        <span>{{ updateToLatestLabel }}</span>
      </button>
    </div>

    <div
      v-if="viewMode !== 'chat'"
      class="relative z-10 w-fit min-w-0 flex items-center justify-center justify-self-center"
      @mousedown.stop
    >
      <label
        v-if="viewMode === 'config'"
        ref="configSearchPopoverRef"
        class="input input-bordered input-sm relative flex h-8 w-[min(18rem,calc(100vw-8rem))] items-center gap-2 bg-base-100"
      >
        <Search class="h-3.5 w-3.5 opacity-70" />
        <input
          ref="configSearchInputRef"
          type="text"
          class="w-full bg-transparent outline-none"
          :value="configSearchQuery"
          :placeholder="configSearchPlaceholder"
          @focus="openConfigSearchPopover"
          @input="handleConfigSearchInput"
          @keydown="handleConfigSearchKeydown"
        />
        <div
          v-if="configSearchOpen && String(configSearchQuery || '').trim()"
          class="absolute left-0 top-full mt-2 w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg"
        >
          <button
            v-for="result in configSearchResults"
            :key="result.tab"
            type="button"
            class="flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left hover:bg-base-200"
            @click="selectConfigSearchResult(result.tab)"
          >
            <span class="text-sm font-medium">{{ result.title }}</span>
            <span v-if="result.matchedTexts[0]" class="text-xs opacity-60 truncate w-full">{{ result.matchedTexts[0] }}</span>
          </button>
          <div v-if="(configSearchResults || []).length === 0" class="px-3 py-3 text-sm opacity-60">
            {{ t("config.search.noResults") }}
          </div>
        </div>
      </label>

      <div v-else class="pointer-events-none flex items-center px-2">
        <span class="font-semibold text-sm">{{ titleText }}</span>
      </div>
    </div>

    <div v-if="viewMode !== 'chat'" class="relative z-10 flex shrink-0 flex-nowrap justify-self-end gap-1 px-2" @mousedown.stop>
      <button
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-sm btn-ghost hover:bg-error"
        :title="closeTitle || t('common.close')"
        @click.stop="$emit('close-window')"
        :disabled="!windowReady"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </div>

  <dialog v-if="viewMode === 'chat'" class="modal" :class="{ 'modal-open': createConversationDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="text-base font-semibold">{{ t("chat.newConversation") }}</h3>
      <div class="mt-3 flex flex-col gap-3">
        <div class="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-sm leading-6 text-base-content/80">
          {{ t("chat.unarchivedConversationMemoryReminder") }}
        </div>
        <input
          ref="createConversationInputRef"
          v-model="createConversationTitle"
          type="text"
          class="input input-bordered w-full"
          :placeholder="t('chat.newConversationTopicPlaceholder')"
          @keydown="handleCreateConversationDialogKeydown"
        />
        <select
          v-model="createConversationDepartmentId"
          class="select select-bordered w-full"
        >
          <option
            v-for="department in createConversationDepartmentOptions"
            :key="department.id"
            :value="department.id"
          >
            {{ departmentOptionLabel(department) }}
          </option>
        </select>
        <label class="label cursor-pointer justify-start gap-3 rounded-lg border border-base-300 px-3 py-2">
          <input v-model="createConversationCopyCurrent" type="checkbox" class="checkbox checkbox-sm" />
          <span class="label-text text-sm">{{ t("chat.copyCurrentConversation") }}</span>
        </label>
        <div v-if="recentConversationTopics.length > 0" class="flex flex-col gap-2">
          <div class="text-xs font-medium opacity-70">{{ t("chat.recentConversationTopics") }}</div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="topic in recentConversationTopics"
              :key="topic"
              type="button"
              class="btn btn-sm btn-ghost"
              @click="applyRecentConversationTopic(topic)"
            >
              {{ topic }}
            </button>
          </div>
        </div>
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" @click="closeCreateConversationDialog">{{ t("common.cancel") }}</button>
        <button class="btn btn-sm btn-primary" @click="confirmCreateConversation">{{ t("common.confirm") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeCreateConversationDialog">close</button>
    </form>
  </dialog>

  <dialog v-if="viewMode === 'config'" class="modal" :class="{ 'modal-open': changelogDialogOpen }">
    <div class="modal-box h-[90vh] w-[90vw] max-w-none p-0">
      <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
        <div class="text-sm font-medium">{{ t("about.changelog") }}</div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            :disabled="changelogLoading"
            @click="loadProjectChangelog(true)"
          >
            <span v-if="changelogLoading" class="loading loading-spinner loading-xs"></span>
            {{ t("common.refresh") }}
          </button>
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            @click="closeChangelogDialog"
          >
            {{ t("common.close") }}
          </button>
        </div>
      </div>
      <div class="config-changelog-markdown h-[calc(90vh-118px)] overflow-auto px-5 py-4">
        <div v-if="changelogLoading && !changelogMarkdown" class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          <span class="loading loading-spinner loading-sm mr-2"></span>
          {{ t("about.changelogLoading") }}
        </div>
        <div v-else-if="changelogError" class="rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ changelogError }}
        </div>
        <MarkdownRender
          v-else-if="changelogMarkdown"
          class="ecall-markdown-content max-w-none"
          custom-id="chat-markstream"
          :nodes="changelogNodes"
          :is-dark="markdownIsDark"
          :code-block-props="markdownCodeBlockProps"
          :mermaid-props="markdownMermaidProps"
          :typewriter="false"
        />
        <div v-else class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          {{ t("about.changelogEmpty") }}
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeChangelogDialog">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../services/tauri-api";
import MarkdownRender, { enableKatex, enableMermaid, getMarkdown, parseMarkdownToStructure } from "markstream-vue";
import { Download, FoldVertical, History, Minus, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, ScrollText, Search, Settings, Square, SquarePen, X } from "lucide-vue-next";
import type { ChatConversationOverviewItem } from "../../../types/app";
import { resolveConversationDisplayTitle } from "../../chat/utils/conversation-title";
import ChatConversationSidebar from "../../chat/components/ChatConversationSidebar.vue";
import { registerChatMarkstreamComponents } from "../../chat/markdown/register-chat-markstream";
import type { ConfigSearchResult, ConfigSearchTab } from "../../config/search/config-search";
import { isDarkAppTheme } from "../composables/use-app-theme";
import "markstream-vue/index.css";
import { usePipelineStatus } from "../composables/use-pipeline-status";

const RING_RADIUS = 14;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

type ConversationDepartmentOption = {
  id: string;
  name: string;
  ownerAgentId?: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
};

type CreateConversationInput = {
  title?: string;
  departmentId?: string;
  copyCurrent?: boolean;
};

const RECENT_CONVERSATION_TOPICS_STORAGE_KEY = "easy_call.recent_conversation_topics.v1";
const RECENT_CONVERSATION_TOPICS_LIMIT = 7;

enableMermaid();
enableKatex();
registerChatMarkstreamComponents();

const { markConversationRead } = usePipelineStatus({
  activeConversationId: computed(() => String(props.activeConversationId || "").trim()),
});

const markstreamMarkdown = getMarkdown();
const markdownCodeBlockProps = {
  showHeader: true,
  showCopyButton: true,
  showPreviewButton: false,
  showExpandButton: true,
  showCollapseButton: true,
  showFontSizeButtons: false,
  enableFontSizeControl: false,
  isShowPreview: false,
  showTooltips: false,
};
const markdownMermaidProps = {
  showHeader: true,
  showCopyButton: true,
  showExportButton: false,
  showFullscreenButton: true,
  showCollapseButton: false,
  showZoomControls: true,
  showModeToggle: false,
  enableWheelZoom: true,
  showTooltips: false,
};

const props = defineProps<{
  viewMode: "chat" | "archives" | "config";
  detachedChatWindow?: boolean;
  currentTheme: string;
  titleText: string;
  chatUsagePercent: number;
  forcingArchive: boolean;
  chatting: boolean;
  currentPersonaName: string;
  sideConversationListVisible: boolean;
  toolReviewPanelOpenVisible: boolean;
  chatSidePanelWidths: { leftWidth: number; rightWidth: number };
  conversationListTab: "local" | "contact";
  chatLeftPanelMode: "local" | "contact";
  chatRightPanelMode: "reader" | "review" | "delegate";
  activeConversationId: string;
  conversationItems: ChatConversationOverviewItem[];
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  createConversationDepartmentOptions: ConversationDepartmentOption[];
  defaultCreateConversationDepartmentId: string;
  forceArchiveTip: string;
  maximized: boolean;
  windowReady: boolean;
  openConfigTitle: string;
  closeTitle?: string;
  configSearchQuery?: string;
  configSearchResults?: ConfigSearchResult[];
  configSearchPlaceholder?: string;
  showUpdateToLatestButton?: boolean;
  hasAvailableUpdate?: boolean;
  checkingUpdate?: boolean;
  updateToLatestLabel?: string;
  updateToLatestTitle?: string;
}>();

const emit = defineEmits<{
  (e: "open-config"): void;
  (e: "open-archives"): void;
  (e: "toggle-side-conversation-list"): void;
  (e: "toggle-tool-review-panel"): void;
  (e: "update:conversation-list-tab", value: "local" | "contact"): void;
  (e: "update:chat-left-panel-mode", value: "local" | "contact"): void;
  (e: "update:chat-right-panel-mode", value: "reader" | "review" | "delegate"): void;
  (e: "minimize-window"): void;
  (e: "toggle-maximize-window"): void;
  (e: "switch-conversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "rename-conversation", payload: { conversationId: string; title: string }): void;
  (e: "toggle-pin-conversation", conversationId: string): void;
  (e: "archive-conversation", conversationId: string): void;
  (e: "delete-conversation", conversationId: string): void;
  (e: "create-conversation", input?: CreateConversationInput): void;
  (e: "force-archive"): void;
  (e: "startDrag"): void;
  (e: "close-window"): void;
  (e: "update:config-search-query", value: string): void;
  (e: "select-config-search-result", tab: ConfigSearchTab): void;
  (e: "update-to-latest"): void;
}>();

const { t, locale } = useI18n();

const circumference = RING_CIRCUMFERENCE;

const normalizedChatUsagePercent = computed(() =>
  Math.min(100, Math.max(0, Math.round(Number(props.chatUsagePercent || 0)))),
);

const strokeDashoffset = computed(() => {
  const percent = normalizedChatUsagePercent.value;
  return RING_CIRCUMFERENCE * (1 - percent / 100);
});

const chatHeaderGridStyle = computed(() => {
  const leftWidth = Number(props.chatSidePanelWidths?.leftWidth || 0);
  const rightWidth = Number(props.chatSidePanelWidths?.rightWidth || 0);
  const leftColumn = props.sideConversationListVisible && !props.detachedChatWindow && Number.isFinite(leftWidth) && leftWidth > 0
    ? `${Math.round(leftWidth)}px`
    : "0px";
  const rightColumn = props.toolReviewPanelOpenVisible && Number.isFinite(rightWidth) && rightWidth > 0
    ? `${Math.round(rightWidth)}px`
    : "max-content";
  return {
    gridTemplateColumns: `${leftColumn} minmax(0, 1fr) ${rightColumn}`,
  };
});

const currentConversationTitle = computed(() => {
  const activeId = String(props.activeConversationId || "").trim();
  if (!activeId) return "";
  const item = props.conversationItems.find((i) => i.conversationId === activeId);
  if (!item) return "";
  return resolveConversationDisplayTitle(item, {
    locale: locale.value,
    untitledLabel: t("chat.untitledConversation"),
  });
});

const currentConversationDepartmentName = computed(() => {
  const activeId = String(props.activeConversationId || "").trim();
  if (!activeId) return "";
  const item = props.conversationItems.find((i) => i.conversationId === activeId);
  return item?.departmentName || "";
});

const conversationUnreadTotal = computed(() =>
  props.conversationItems.reduce((total, item) => total + Math.max(0, Number(item.unreadCount || 0)), 0),
);

const hoverSidebarOpen = ref(false);
let hoverSidebarTimer: ReturnType<typeof setTimeout> | null = null;

function handleHoverSidebarEnter() {
  if (hoverSidebarTimer) { clearTimeout(hoverSidebarTimer); hoverSidebarTimer = null; }
  hoverSidebarOpen.value = true;
}

function handleHoverSidebarLeave() {
  if (hoverSidebarTimer) clearTimeout(hoverSidebarTimer);
  hoverSidebarTimer = setTimeout(() => { hoverSidebarOpen.value = false; }, 200);
}

function handleHoverConversationSelect(payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }) {
  hoverSidebarOpen.value = false;
  emit("switch-conversation", payload);
}

const combinedTitle = computed(() => {
  const parts: string[] = [];
  const title = currentConversationTitle.value;
  const dept = currentConversationDepartmentName.value;
  const persona = props.currentPersonaName;

  if (title) parts.push(title);
  if (dept) parts.push(dept);
  if (persona) parts.push(persona);

  return parts.join(" · ");
});

const combinedTitleTooltip = computed(() => {
  return combinedTitle.value || props.currentPersonaName;
});

watch(
  () => props.activeConversationId,
  (conversationId) => markConversationRead(conversationId),
  { immediate: true },
);
const configSearchPopoverRef = ref<HTMLElement | null>(null);
const configSearchInputRef = ref<HTMLInputElement | null>(null);
const createConversationInputRef = ref<HTMLInputElement | null>(null);
const recentConversationTopics = ref<string[]>([]);
const createConversationDialogOpen = ref(false);
const createConversationTitle = ref("");
const createConversationDepartmentId = ref("");
const createConversationCopyCurrent = ref(false);
const configSearchOpen = ref(false);
const changelogDialogOpen = ref(false);
const changelogLoading = ref(false);
const changelogError = ref("");
const changelogMarkdown = ref("");
const changelogLoaded = ref(false);

const changelogNodes = computed(() =>
  parseMarkdownToStructure(changelogMarkdown.value || "", markstreamMarkdown, { final: true }),
);
const markdownIsDark = computed(() => isDarkAppTheme(props.currentTheme));

function loadRecentConversationTopics() {
  try {
    const raw = window.localStorage.getItem(RECENT_CONVERSATION_TOPICS_STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    const normalized: string[] = [];
    const seen = new Set<string>();
    for (const item of parsed) {
      const text = String(item || "").trim();
      if (!text || seen.has(text)) continue;
      seen.add(text);
      normalized.push(text);
      if (normalized.length >= RECENT_CONVERSATION_TOPICS_LIMIT) break;
    }
    recentConversationTopics.value = normalized;
  } catch {
    recentConversationTopics.value = [];
  }
}

function saveRecentConversationTopics() {
  try {
    window.localStorage.setItem(RECENT_CONVERSATION_TOPICS_STORAGE_KEY, JSON.stringify(recentConversationTopics.value));
  } catch {
    // ignore persistence failures
  }
}

function pushRecentConversationTopic(rawText: string) {
  const text = String(rawText || "").trim();
  if (!text) return;
  recentConversationTopics.value = [text, ...recentConversationTopics.value.filter((item) => item !== text)].slice(0, RECENT_CONVERSATION_TOPICS_LIMIT);
  saveRecentConversationTopics();
}

function handleDocumentPointerDown(event: PointerEvent) {
  const target = event.target as Node | null;
  const searchRoot = configSearchPopoverRef.value;
  if (configSearchOpen.value && searchRoot && target && !searchRoot.contains(target)) {
    configSearchOpen.value = false;
  }
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && configSearchOpen.value) {
    configSearchOpen.value = false;
  }
  if (event.key === "Escape" && changelogDialogOpen.value) {
    closeChangelogDialog();
  }
}

function openConfigSearchPopover() {
  if (props.viewMode !== "config") return;
  configSearchOpen.value = true;
}

function handleConfigSearchInput(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  emit("update:config-search-query", value);
  configSearchOpen.value = true;
}

function selectConfigSearchResult(tab: ConfigSearchTab) {
  emit("select-config-search-result", tab);
  configSearchOpen.value = false;
}

function handleConfigSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && props.configSearchResults && props.configSearchResults.length > 0) {
    event.preventDefault();
    selectConfigSearchResult(props.configSearchResults[0].tab);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    configSearchOpen.value = false;
  }
}

function handleCreateConversation() {
  createConversationTitle.value = "";
  const activeConversation = props.conversationItems.find(
    (item) => String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim(),
  );
  createConversationDepartmentId.value =
    String(activeConversation?.departmentId || "").trim()
    || String(props.defaultCreateConversationDepartmentId || "").trim()
    || String(props.createConversationDepartmentOptions[0]?.id || "").trim();
  createConversationCopyCurrent.value = false;
  createConversationDialogOpen.value = true;
  nextTick(() => createConversationInputRef.value?.focus());
}

async function loadProjectChangelog(force = false) {
  if (changelogLoading.value) return;
  if (changelogLoaded.value && !force) return;
  changelogLoading.value = true;
  changelogError.value = "";
  try {
    changelogMarkdown.value = await invokeTauri<string>("fetch_project_changelog_markdown");
    changelogLoaded.value = true;
  } catch (error) {
    changelogError.value = String(error);
  } finally {
    changelogLoading.value = false;
  }
}

function openChangelogDialog() {
  changelogDialogOpen.value = true;
  void loadProjectChangelog();
}

function closeChangelogDialog() {
  changelogDialogOpen.value = false;
}

function closeCreateConversationDialog() {
  createConversationDialogOpen.value = false;
  createConversationTitle.value = "";
  createConversationDepartmentId.value = "";
  createConversationCopyCurrent.value = false;
}

function applyRecentConversationTopic(topic: string) {
  createConversationTitle.value = String(topic || "").trim();
  nextTick(() => createConversationInputRef.value?.focus());
}

function departmentOptionLabel(department: ConversationDepartmentOption): string {
  const departmentName = String(department.name || "").trim();
  const ownerName = String(department.ownerName || "").trim();
  return ownerName ? `${departmentName} / ${ownerName}` : departmentName;
}

function confirmCreateConversation() {
  const title = String(createConversationTitle.value || "").trim();
  const departmentId = String(createConversationDepartmentId.value || "").trim();
  const copyCurrent = !!createConversationCopyCurrent.value;
  if (title) {
    pushRecentConversationTopic(title);
  }
  createConversationDialogOpen.value = false;
  createConversationTitle.value = "";
  createConversationDepartmentId.value = "";
  createConversationCopyCurrent.value = false;
  emit("create-conversation", {
    title,
    departmentId: departmentId || undefined,
    copyCurrent,
  });
}

function handleCreateConversationDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
    event.preventDefault();
    confirmCreateConversation();
  }
}

onMounted(() => {
  loadRecentConversationTopics();
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  window.addEventListener("keydown", handleWindowKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  window.removeEventListener("keydown", handleWindowKeydown);
});
</script>

<style scoped>
.config-changelog-markdown:deep(.ecall-markdown-content.prose) {
  max-width: none;
}

.config-changelog-markdown:deep(.ecall-markdown-content) {
  color: inherit;
  line-height: 1.75;
  font-size: 0.95rem;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(p,ul,ol,blockquote,pre,table,figure,.paragraph-node,.list-node,.blockquote,.table-node-wrapper,.code-block-container,._mermaid,.vmr-container)) {
  margin-top: 0.85rem;
  margin-bottom: 0.85rem;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(h1,h2,h3,h4,.heading-node)) {
  margin-top: 1.25rem;
  margin-bottom: 0.75rem;
  font-weight: 700;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(a,.link-node)) {
  color: hsl(var(--p));
  text-decoration: underline;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(blockquote,.blockquote)) {
  border-left: 3px solid color-mix(in srgb, currentColor 18%, transparent);
  padding-left: 0.9rem;
  opacity: 0.9;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(:not(pre) > code,.inline-code)) {
  border: 1px solid color-mix(in srgb, currentColor 12%, transparent);
  border-radius: 0.45rem;
  padding: 0.08rem 0.35rem;
  background: color-mix(in srgb, currentColor 6%, transparent);
}
</style>
