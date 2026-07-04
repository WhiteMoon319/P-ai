export type ChatLeftPanelMode = "local" | "contact" | "task";
export type ChatRightPanelMode = "reader" | "review" | "delegate";
export type ChatSidePanelSide = "left" | "right";
export type ChatSidePanelWidths = { leftWidth: number; rightWidth: number };

const CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY = "easy_call.chat_conversation_list_tab.v1";
const CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy_call.chat_left_panel_mode.v1";
const CHAT_RIGHT_PANEL_MODE_STORAGE_KEY = "easy_call.chat_right_panel_mode.v1";
const LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy-call.chat.left-panel-mode";
const LEGACY_CHAT_RIGHT_PANEL_MODE_STORAGE_KEY = "easy-call.chat.right-panel-mode";
const CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_visible.v1",
  right: "easy_call.chat_right_sidebar_visible.v1",
} as const;
const LEGACY_CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-visible",
  right: "easy-call.chat.right-sidebar-visible",
} as const;
const CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_width.v1",
  right: "easy_call.chat_right_sidebar_width.v1",
} as const;
const LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-width",
  right: "easy-call.chat.right-sidebar-width",
} as const;

export function normalizeChatLeftPanelMode(value: string): ChatLeftPanelMode {
  if (value === "contact" || value === "task") return value;
  return "local";
}

export function normalizeChatRightPanelMode(value: string, fallback: ChatRightPanelMode = "delegate"): ChatRightPanelMode {
  if (value === "reader" || value === "review" || value === "delegate") return value;
  return fallback;
}

export function normalizeChatSidePanelWidths(value: Partial<ChatSidePanelWidths> | null | undefined): ChatSidePanelWidths {
  const leftWidth = Number(value?.leftWidth);
  const rightWidth = Number(value?.rightWidth);
  return {
    leftWidth: Number.isFinite(leftWidth) ? leftWidth : 320,
    rightWidth: Number.isFinite(rightWidth) ? rightWidth : 320,
  };
}

export function loadStoredConversationListTab(): ChatLeftPanelMode {
  if (typeof window === "undefined") return "local";
  const stored = String(window.localStorage.getItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY) || "").trim();
  return normalizeChatLeftPanelMode(stored);
}

export function storeConversationListTab(value: ChatLeftPanelMode) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY, normalizeChatLeftPanelMode(value));
}

export function loadStoredChatLeftPanelMode(): ChatLeftPanelMode {
  if (typeof window === "undefined") return loadStoredConversationListTab();
  const stored = String(
    window.localStorage.getItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY)
    || window.localStorage.getItem(LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY)
    || "",
  ).trim();
  return stored ? normalizeChatLeftPanelMode(stored) : loadStoredConversationListTab();
}

export function storeChatLeftPanelMode(value: ChatLeftPanelMode) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY, normalizeChatLeftPanelMode(value));
}

export function loadStoredChatRightPanelMode(fallback: ChatRightPanelMode = "delegate"): ChatRightPanelMode {
  if (typeof window === "undefined") return fallback;
  const stored = String(
    window.localStorage.getItem(CHAT_RIGHT_PANEL_MODE_STORAGE_KEY)
    || window.localStorage.getItem(LEGACY_CHAT_RIGHT_PANEL_MODE_STORAGE_KEY)
    || "",
  ).trim();
  return normalizeChatRightPanelMode(stored, fallback);
}

export function storeChatRightPanelMode(value: ChatRightPanelMode) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_RIGHT_PANEL_MODE_STORAGE_KEY, normalizeChatRightPanelMode(value));
}

export function loadStoredChatSidePanelVisibility(side: ChatSidePanelSide): boolean {
  if (typeof window === "undefined") return false;
  const stored = window.localStorage.getItem(CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side])
    ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side]);
  return stored === "true";
}

export function storeChatSidePanelVisibility(side: ChatSidePanelSide, visible: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side], visible ? "true" : "false");
}

export function loadStoredChatSidePanelWidths(): ChatSidePanelWidths {
  if (typeof window === "undefined") {
    return { leftWidth: 320, rightWidth: 320 };
  }
  const leftWidth = Number(
    window.localStorage.getItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.left)
    ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.left),
  );
  const rightWidth = Number(
    window.localStorage.getItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.right)
    ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.right),
  );
  return normalizeChatSidePanelWidths({ leftWidth, rightWidth });
}

export function storeChatSidePanelWidths(value: Partial<ChatSidePanelWidths> | null | undefined) {
  if (typeof window === "undefined") return;
  const widths = normalizeChatSidePanelWidths(value);
  window.localStorage.setItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.left, String(widths.leftWidth));
  window.localStorage.setItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.right, String(widths.rightWidth));
}
