import { ref } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntimeAvailable } from "../../../services/tauri-api";

const CHAT_BUBBLE_BACKGROUND_STORAGE_KEY = "easy-call.chat.bubble-background.v1";
const CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY = "easy-call.chat.segmented-markdown.v1";
const CHAT_TIME_DISPLAY_MODE_STORAGE_KEY = "easy-call.chat.time-display-mode.v1";
const CHAT_MESSAGE_APPEARANCE_CHANGED_EVENT = "easy-call:chat-message-appearance-changed";

type ChatTimeDisplayMode = "relative" | "absolute";

type ChatMessageAppearancePayload = {
  assistantBubbleBackgroundEnabled?: boolean;
  segmentedMarkdownEnabled?: boolean;
  chatTimeDisplayMode?: ChatTimeDisplayMode;
};

const assistantBubbleBackgroundEnabled = ref(readBooleanPreference(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY));
const segmentedMarkdownEnabled = ref(readBooleanPreference(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY));
const chatTimeDisplayMode = ref<ChatTimeDisplayMode>(readChatTimeDisplayModePreference());
let initialized = false;
let eventUnlisten: UnlistenFn | null = null;

function readBooleanPreference(storageKey: string): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(storageKey) === "1";
}

function persistBooleanPreference(storageKey: string, enabled: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, enabled ? "1" : "0");
}

function readChatTimeDisplayModePreference(): ChatTimeDisplayMode {
  if (typeof window === "undefined") return "relative";
  return window.localStorage.getItem(CHAT_TIME_DISPLAY_MODE_STORAGE_KEY) === "absolute" ? "absolute" : "relative";
}

function persistChatTimeDisplayMode(mode: ChatTimeDisplayMode) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_TIME_DISPLAY_MODE_STORAGE_KEY, mode);
}

function applyPayload(payload: ChatMessageAppearancePayload | undefined) {
  if (typeof payload?.assistantBubbleBackgroundEnabled === "boolean") {
    assistantBubbleBackgroundEnabled.value = payload.assistantBubbleBackgroundEnabled;
  }
  if (typeof payload?.segmentedMarkdownEnabled === "boolean") {
    segmentedMarkdownEnabled.value = payload.segmentedMarkdownEnabled;
  }
  if (payload?.chatTimeDisplayMode === "absolute" || payload?.chatTimeDisplayMode === "relative") {
    chatTimeDisplayMode.value = payload.chatTimeDisplayMode;
  }
}

function restoreFromStorage() {
  assistantBubbleBackgroundEnabled.value = readBooleanPreference(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY);
  segmentedMarkdownEnabled.value = readBooleanPreference(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY);
  chatTimeDisplayMode.value = readChatTimeDisplayModePreference();
}

function handleStorageEvent(event: StorageEvent) {
  if (
    event.key !== CHAT_BUBBLE_BACKGROUND_STORAGE_KEY
    && event.key !== CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY
    && event.key !== CHAT_TIME_DISPLAY_MODE_STORAGE_KEY
  ) return;
  restoreFromStorage();
}

export function initChatMessageAppearance() {
  if (initialized) return;
  initialized = true;
  restoreFromStorage();
  if (typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  if (isTauriRuntimeAvailable()) {
    void listen<ChatMessageAppearancePayload>(CHAT_MESSAGE_APPEARANCE_CHANGED_EVENT, (event) => {
      applyPayload(event.payload);
    }).then((unlisten) => {
      eventUnlisten = unlisten;
    }).catch((error) => {
      console.warn("[聊天外观] 监听消息外观变化失败", error);
    });
  }
}

function emitAppearanceChanged() {
  if (!isTauriRuntimeAvailable()) return;
  void emit(CHAT_MESSAGE_APPEARANCE_CHANGED_EVENT, {
    assistantBubbleBackgroundEnabled: assistantBubbleBackgroundEnabled.value,
    segmentedMarkdownEnabled: segmentedMarkdownEnabled.value,
    chatTimeDisplayMode: chatTimeDisplayMode.value,
  } satisfies ChatMessageAppearancePayload).catch((error) => {
    console.warn("[聊天外观] 同步消息外观变化失败", error);
  });
}

export function useChatMessageAppearance() {
  initChatMessageAppearance();

  function setAssistantBubbleBackgroundEnabled(enabled: boolean) {
    assistantBubbleBackgroundEnabled.value = enabled;
    persistBooleanPreference(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY, enabled);
    emitAppearanceChanged();
  }

  function setSegmentedMarkdownEnabled(enabled: boolean) {
    segmentedMarkdownEnabled.value = enabled;
    persistBooleanPreference(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY, enabled);
    emitAppearanceChanged();
  }

  function setChatTimeDisplayMode(mode: ChatTimeDisplayMode) {
    chatTimeDisplayMode.value = mode;
    persistChatTimeDisplayMode(mode);
    emitAppearanceChanged();
  }

  return {
    assistantBubbleBackgroundEnabled,
    segmentedMarkdownEnabled,
    chatTimeDisplayMode,
    setAssistantBubbleBackgroundEnabled,
    setSegmentedMarkdownEnabled,
    setChatTimeDisplayMode,
  };
}
