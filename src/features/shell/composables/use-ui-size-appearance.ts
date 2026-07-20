import { ref } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntimeAvailable } from "../../../services/tauri-api";

const UI_SIZE_STORAGE_KEY = "easy-call.ui-size.v1";
const UI_SIZE_CHANGED_EVENT = "easy-call:ui-size-changed";

export const UI_SIZE_PRESETS = ["small", "default", "large", "extraLarge"] as const;
export type UiSizePreset = typeof UI_SIZE_PRESETS[number];

type UiSizePayload = {
  preset?: unknown;
};

type UiSizeTokens = {
  textXs: string;
  textSm: string;
  textBase: string;
  textLg: string;
  textXl: string;
  text2Xl: string;
  sizeField: string;
  sizeSelector: string;
  border: string;
};

const UI_SIZE_TOKEN_MAP: Record<UiSizePreset, UiSizeTokens> = {
  small: {
    textXs: "9px",
    textSm: "10.5px",
    textBase: "12px",
    textLg: "13.5px",
    textXl: "15px",
    text2Xl: "18px",
    sizeField: "0.195rem",
    sizeSelector: "0.195rem",
    border: "1px",
  },
  default: {
    textXs: "12px",
    textSm: "14px",
    textBase: "16px",
    textLg: "18px",
    textXl: "20px",
    text2Xl: "24px",
    sizeField: "0.26rem",
    sizeSelector: "0.26rem",
    border: "1px",
  },
  large: {
    textXs: "15px",
    textSm: "17.5px",
    textBase: "20px",
    textLg: "22.5px",
    textXl: "25px",
    text2Xl: "30px",
    sizeField: "0.325rem",
    sizeSelector: "0.325rem",
    border: "1.25px",
  },
  extraLarge: {
    textXs: "18px",
    textSm: "21px",
    textBase: "24px",
    textLg: "27px",
    textXl: "30px",
    text2Xl: "36px",
    sizeField: "0.39rem",
    sizeSelector: "0.39rem",
    border: "1.5px",
  },
};

const uiSizePreset = ref<UiSizePreset>("default");
let initialized = false;
let eventUnlisten: UnlistenFn | null = null;

export function normalizeUiSizePreset(value: unknown): UiSizePreset {
  return UI_SIZE_PRESETS.includes(value as UiSizePreset) ? value as UiSizePreset : "default";
}

function readStoredUiSizePreset(): UiSizePreset {
  if (typeof window === "undefined") return "default";
  return normalizeUiSizePreset(window.localStorage.getItem(UI_SIZE_STORAGE_KEY));
}

function persistUiSizePreset(preset: UiSizePreset) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(UI_SIZE_STORAGE_KEY, preset);
}

export function applyUiSizePreset(value: unknown) {
  const preset = normalizeUiSizePreset(value);
  uiSizePreset.value = preset;
  if (typeof document === "undefined") return preset;
  const tokens = UI_SIZE_TOKEN_MAP[preset];
  const root = document.documentElement.style;
  root.setProperty("--app-text-xs-size", tokens.textXs);
  root.setProperty("--app-text-sm-size", tokens.textSm);
  root.setProperty("--app-text-base-size", tokens.textBase);
  root.setProperty("--app-text-lg-size", tokens.textLg);
  root.setProperty("--app-text-xl-size", tokens.textXl);
  root.setProperty("--app-text-2xl-size", tokens.text2Xl);
  root.setProperty("--size-field", tokens.sizeField);
  root.setProperty("--size-selector", tokens.sizeSelector);
  root.setProperty("--border", tokens.border);
  return preset;
}

function handleStorageEvent(event: StorageEvent) {
  if (event.key === UI_SIZE_STORAGE_KEY) applyUiSizePreset(event.newValue);
}

export function initUiSizeAppearance() {
  if (initialized) return;
  initialized = true;
  applyUiSizePreset(readStoredUiSizePreset());
  if (typeof window !== "undefined") window.addEventListener("storage", handleStorageEvent);
  if (!isTauriRuntimeAvailable()) return;
  void listen<UiSizePayload>(UI_SIZE_CHANGED_EVENT, (event) => {
    applyUiSizePreset(event.payload?.preset);
  }).then((unlisten) => {
    eventUnlisten = unlisten;
  }).catch((error) => {
    console.warn("[界面尺寸] 监听尺寸变化失败", error);
  });
}

export function useUiSizeAppearance() {
  initUiSizeAppearance();

  function setUiSizePreset(value: unknown) {
    const preset = applyUiSizePreset(value);
    persistUiSizePreset(preset);
    if (isTauriRuntimeAvailable()) {
      void emit(UI_SIZE_CHANGED_EVENT, { preset }).catch((error) => {
        console.warn("[界面尺寸] 同步尺寸变化失败", error);
      });
    }
    return preset;
  }

  return { uiSizePreset, setUiSizePreset };
}
