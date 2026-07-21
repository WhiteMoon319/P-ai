import { ref } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntimeAvailable } from "../../../services/tauri-api";

const UI_SIZE_STORAGE_KEY = "easy-call.ui-size.v1";
const UI_SIZE_CHANGED_EVENT = "easy-call:ui-size-changed";

export const UI_SIZE_MIN_SCALE = 75;
export const UI_SIZE_MAX_SCALE = 150;
export const UI_SIZE_DEFAULT_SCALE = 100;
export const UI_SIZE_STEP_SCALE = 10;
export const UI_SIZE_SCALE_MARKS = [75, 100, 125, 150] as const;
export type UiSizeScale = number;

type UiSizePayload = {
  scale?: unknown;
  preset?: unknown;
};

type UiSizeTokens = {
  textMicro: string;
  textCaption: string;
  textXs: string;
  textSm: string;
  textBase: string;
  textLg: string;
  textXl: string;
  text2Xl: string;
  markdownHeading1: string;
  markdownHeading2: string;
  markdownHeading3: string;
  markdownHeading4: string;
  markdownDocumentHeading1: string;
  markdownDocumentHeading2: string;
  markdownDocumentHeading3: string;
  markdownDocumentHeading4: string;
  sizeField: string;
  sizeSelector: string;
  border: string;
};

const LEGACY_UI_SIZE_SCALE_MAP: Record<string, UiSizeScale> = {
  small: 75,
  default: 100,
  large: 125,
  extraLarge: 150,
};

const uiSizeScale = ref<UiSizeScale>(UI_SIZE_DEFAULT_SCALE);
let initialized = false;
let eventUnlisten: UnlistenFn | null = null;

function scaledPx(value: number, scale: UiSizeScale): string {
  return `${Math.round(value * scale) / 100}px`;
}

export function normalizeUiSizeScale(value: unknown): UiSizeScale {
  if (value == null || (typeof value === "string" && !value.trim())) {
    return UI_SIZE_DEFAULT_SCALE;
  }
  if (typeof value === "string" && value.trim() in LEGACY_UI_SIZE_SCALE_MAP) {
    return LEGACY_UI_SIZE_SCALE_MAP[value.trim()];
  }
  const numeric = Math.round(Number(value));
  if (!Number.isFinite(numeric)) return UI_SIZE_DEFAULT_SCALE;
  return Math.min(UI_SIZE_MAX_SCALE, Math.max(UI_SIZE_MIN_SCALE, numeric));
}

export function uiSizeTokensFor(value: unknown): UiSizeTokens {
  const scale = normalizeUiSizeScale(value);
  return {
    textMicro: scaledPx(9, scale),
    textCaption: scaledPx(11, scale),
    textXs: scaledPx(12, scale),
    textSm: scaledPx(14, scale),
    textBase: scaledPx(16, scale),
    textLg: scaledPx(18, scale),
    textXl: scaledPx(20, scale),
    text2Xl: scaledPx(24, scale),
    markdownHeading1: scaledPx(16.32, scale),
    markdownHeading2: scaledPx(15.68, scale),
    markdownHeading3: scaledPx(15.04, scale),
    markdownHeading4: scaledPx(14.4, scale),
    markdownDocumentHeading1: scaledPx(24, scale),
    markdownDocumentHeading2: scaledPx(20.48, scale),
    markdownDocumentHeading3: scaledPx(17.92, scale),
    markdownDocumentHeading4: scaledPx(16.32, scale),
    sizeField: scaledPx(4.16, scale),
    sizeSelector: scaledPx(4.16, scale),
    border: scaledPx(1, scale),
  };
}

function readStoredUiSizeScale(): UiSizeScale {
  if (typeof window === "undefined") return UI_SIZE_DEFAULT_SCALE;
  return normalizeUiSizeScale(window.localStorage.getItem(UI_SIZE_STORAGE_KEY));
}

function persistUiSizeScale(scale: UiSizeScale) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(UI_SIZE_STORAGE_KEY, String(scale));
}

export function applyUiSizeScale(value: unknown): UiSizeScale {
  const scale = normalizeUiSizeScale(value);
  uiSizeScale.value = scale;
  if (typeof document === "undefined") return scale;
  const tokens = uiSizeTokensFor(scale);
  const root = document.documentElement.style;
  root.setProperty("--app-text-micro-size", tokens.textMicro);
  root.setProperty("--app-text-caption-size", tokens.textCaption);
  root.setProperty("--app-text-xs-size", tokens.textXs);
  root.setProperty("--app-text-sm-size", tokens.textSm);
  root.setProperty("--app-text-base-size", tokens.textBase);
  root.setProperty("--app-text-lg-size", tokens.textLg);
  root.setProperty("--app-text-xl-size", tokens.textXl);
  root.setProperty("--app-text-2xl-size", tokens.text2Xl);
  root.setProperty("--app-text-markdown-heading-1-size", tokens.markdownHeading1);
  root.setProperty("--app-text-markdown-heading-2-size", tokens.markdownHeading2);
  root.setProperty("--app-text-markdown-heading-3-size", tokens.markdownHeading3);
  root.setProperty("--app-text-markdown-heading-4-size", tokens.markdownHeading4);
  root.setProperty("--app-text-markdown-document-heading-1-size", tokens.markdownDocumentHeading1);
  root.setProperty("--app-text-markdown-document-heading-2-size", tokens.markdownDocumentHeading2);
  root.setProperty("--app-text-markdown-document-heading-3-size", tokens.markdownDocumentHeading3);
  root.setProperty("--app-text-markdown-document-heading-4-size", tokens.markdownDocumentHeading4);
  root.setProperty("--size-field", tokens.sizeField);
  root.setProperty("--size-selector", tokens.sizeSelector);
  root.setProperty("--border", tokens.border);
  return scale;
}

function handleStorageEvent(event: StorageEvent) {
  if (event.key === UI_SIZE_STORAGE_KEY) applyUiSizeScale(event.newValue);
}

function hasUiSizeZoomModifier(event: WheelEvent | KeyboardEvent) {
  return !!event.ctrlKey || !!event.metaKey;
}

export function stepUiSizeScale(direction: number): UiSizeScale {
  const delta = direction > 0 ? UI_SIZE_STEP_SCALE : -UI_SIZE_STEP_SCALE;
  return setUiSizeScale(uiSizeScale.value + delta);
}

function handleGlobalUiSizeWheel(event: WheelEvent) {
  if (!hasUiSizeZoomModifier(event)) return;
  event.preventDefault();
  event.stopPropagation();
  // 向上滚放大，向下滚缩小，每步 10%。
  stepUiSizeScale(event.deltaY < 0 ? 1 : -1);
}

function setUiSizeScale(value: unknown): UiSizeScale {
  const scale = normalizeUiSizeScale(value);
  const changed = uiSizeScale.value !== scale;
  applyUiSizeScale(scale);
  persistUiSizeScale(scale);
  if (changed && isTauriRuntimeAvailable()) {
    void emit(UI_SIZE_CHANGED_EVENT, { scale }).catch((error) => {
      console.warn("[界面尺寸] 同步尺寸变化失败", error);
    });
  }
  return scale;
}

export function initUiSizeAppearance() {
  if (initialized) return;
  initialized = true;
  applyUiSizeScale(readStoredUiSizeScale());
  if (typeof window === "undefined") return;
  window.addEventListener("storage", handleStorageEvent);
  window.addEventListener("wheel", handleGlobalUiSizeWheel, { passive: false, capture: true });
  if (!isTauriRuntimeAvailable()) return;
  void listen<UiSizePayload>(UI_SIZE_CHANGED_EVENT, (event) => {
    applyUiSizeScale(event.payload?.scale ?? event.payload?.preset);
  }).then((unlisten) => {
    eventUnlisten = unlisten;
  }).catch((error) => {
    console.warn("[界面尺寸] 监听尺寸变化失败", error);
  });
}

export function useUiSizeAppearance() {
  initUiSizeAppearance();
  return { uiSizeScale, setUiSizeScale, stepUiSizeScale };
}
