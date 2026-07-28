import { ref } from "vue";
import { emitTransportEvent, onTransportNotification } from "../../../services/tauri-api";
import type { IdeContextReferenceItem, IdeContextWorkspaceGroup } from "../../../types/app";

const SIDE_FILE_TAGS_STORAGE_KEY = "easy-call.chat.composer-side-file-tags.v1";
const IDE_BRIDGE_FILE_TAGS_STORAGE_KEY = "easy-call.chat.composer-ide-bridge-file-tags.v1";
export const SIDE_FILE_TAGS_AVAILABLE = false;

type ChatComposerAppearancePayload = {
  sideFileTagsEnabled?: boolean;
  ideBridgeFileTagsEnabled?: boolean;
};

type VisibleComposerContextGroupsInput = {
  sideReferences: IdeContextReferenceItem[];
  sideWorkspacePath: string;
  sideWorkspaceName: string;
  ideBridgeGroups: IdeContextWorkspaceGroup[];
  sideFileTagsEnabled: boolean;
  ideBridgeFileTagsEnabled: boolean;
};

const sideFileTagsEnabled = ref(false);
const ideBridgeFileTagsEnabled = ref(readBooleanPreference(IDE_BRIDGE_FILE_TAGS_STORAGE_KEY));
let initialized = false;
let eventUnlisten: (() => void) | null = null;

function readBooleanPreference(storageKey: string): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(storageKey) === "1";
}

function persistBooleanPreference(storageKey: string, enabled: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, enabled ? "1" : "0");
}

function applyPayload(payload: ChatComposerAppearancePayload | undefined) {
  sideFileTagsEnabled.value = false;
  if (typeof payload?.ideBridgeFileTagsEnabled === "boolean") {
    ideBridgeFileTagsEnabled.value = payload.ideBridgeFileTagsEnabled;
  }
}

function restoreFromStorage() {
  sideFileTagsEnabled.value = false;
  ideBridgeFileTagsEnabled.value = readBooleanPreference(IDE_BRIDGE_FILE_TAGS_STORAGE_KEY);
}

function handleStorageEvent(event: StorageEvent) {
  if (
    event.key !== SIDE_FILE_TAGS_STORAGE_KEY
    && event.key !== IDE_BRIDGE_FILE_TAGS_STORAGE_KEY
  ) return;
  restoreFromStorage();
}

export function initChatComposerAppearance() {
  if (initialized) return;
  initialized = true;
  restoreFromStorage();
  if (typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  eventUnlisten = onTransportNotification<ChatComposerAppearancePayload>("chatComposerAppearance.changed", (payload) => {
    applyPayload(payload);
  });
}

function emitAppearanceChanged() {
  void emitTransportEvent("chatComposerAppearance.changed", {
    sideFileTagsEnabled: sideFileTagsEnabled.value,
    ideBridgeFileTagsEnabled: ideBridgeFileTagsEnabled.value,
  } satisfies ChatComposerAppearancePayload).catch((error) => {
    console.warn("[输入面板外观] 同步设置变化失败", error);
  });
}

export function visibleChatComposerContextGroups(
  input: VisibleComposerContextGroupsInput,
): IdeContextWorkspaceGroup[] {
  const groups: IdeContextWorkspaceGroup[] = [];
  if (SIDE_FILE_TAGS_AVAILABLE && input.sideFileTagsEnabled && input.sideReferences.length > 0) {
    groups.push({
      workspacePath: String(input.sideWorkspacePath || "").trim(),
      workspaceName: String(input.sideWorkspaceName || "").trim(),
      references: input.sideReferences,
    });
  }
  if (input.ideBridgeFileTagsEnabled) {
    groups.push(...input.ideBridgeGroups);
  }
  return groups;
}

export function useChatComposerAppearance() {
  initChatComposerAppearance();

  function setSideFileTagsEnabled(_enabled: boolean) {
    sideFileTagsEnabled.value = false;
    persistBooleanPreference(SIDE_FILE_TAGS_STORAGE_KEY, false);
    emitAppearanceChanged();
  }

  function setIdeBridgeFileTagsEnabled(enabled: boolean) {
    ideBridgeFileTagsEnabled.value = enabled;
    persistBooleanPreference(IDE_BRIDGE_FILE_TAGS_STORAGE_KEY, enabled);
    emitAppearanceChanged();
  }

  return {
    sideFileTagsEnabled,
    ideBridgeFileTagsEnabled,
    setSideFileTagsEnabled,
    setIdeBridgeFileTagsEnabled,
  };
}
