<template>
  <div class="grid gap-3">
    <!-- 唤出对话快捷键 -->
    <div class="card bg-base-100 border border-base-300">
      <div class="card-body p-4">
        <h3 class="card-title text-base mb-3">{{ t("config.hotkey.label") }}</h3>
        <div class="flex flex-col gap-3">
          <div class="flex items-center gap-2">
            <input :value="config.hotkey" class="input input-bordered input-sm flex-1" placeholder="Alt+·" readonly />
            <button
              class="btn btn-sm bg-base-200 shrink-0"
              :class="{ 'btn-primary': hotkeyCapturing }"
              @click="toggleHotkeyCapture"
            >
              {{ hotkeyCapturing ? t("config.hotkey.recording") : t("config.hotkey.recordButton") }}
            </button>
          </div>
          <div class="text-[11px] opacity-70">{{ hotkeyCaptureHint }}</div>
          <div>
            <button class="btn btn-sm btn-primary shrink-0" @click="$emit('summonChatNow')">
              {{ t("config.hotkey.callNowButton") }}
            </button>
          </div>
          <div class="text-[11px] opacity-60">{{ t("config.hotkey.callNowHint") }}</div>
        </div>
      </div>
    </div>

    <!-- 语音 -->
    <div class="card bg-base-100 border border-base-300">
      <div class="card-body p-4 gap-4">
        <h3 class="card-title text-base">{{ t("config.hotkey.voiceTitle") }}</h3>

        <div class="grid gap-4">
          <div class="grid gap-3">
            <div class="text-sm font-semibold">{{ t("config.hotkey.recordKey") }}</div>
            <div class="flex items-center gap-1">
              <input :value="config.recordHotkey" class="input input-bordered input-sm flex-1" readonly />
              <button
                class="btn btn-sm bg-base-200 shrink-0"
                :class="{ 'btn-primary': recordHotkeyCapturing }"
                @click="toggleRecordHotkeyCapture"
              >
                {{ recordHotkeyCapturing ? t("config.hotkey.recording") : t("config.hotkey.recordButton") }}
              </button>
              <button
                type="button"
                class="btn btn-sm shrink-0"
                :class="config.recordBackgroundWakeEnabled ? 'btn-success text-success-content' : 'bg-base-200'"
                :title="t('config.hotkey.backgroundWakeHint')"
                :aria-pressed="config.recordBackgroundWakeEnabled ? 'true' : 'false'"
                @click="$emit('update:recordBackgroundWakeEnabled', !config.recordBackgroundWakeEnabled)"
              >
                {{ config.recordBackgroundWakeEnabled ? t("config.hotkey.backgroundWakeOn") : t("config.hotkey.backgroundWakeOff") }}
              </button>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <label class="flex min-w-0 flex-col gap-1">
                <div class="flex items-center justify-between py-1"><span class="text-sm">{{ t("config.hotkey.minRecordSeconds") }}</span></div>
                <input
                  :value="config.minRecordSeconds"
                  type="number"
                  min="1"
                  max="30"
                  class="input input-bordered input-sm w-full"
                  @input="onMinRecordSecondsInput"
                />
              </label>
              <label class="flex min-w-0 flex-col gap-1">
                <div class="flex items-center justify-between py-1"><span class="text-sm">{{ t("config.hotkey.maxRecordSeconds") }}</span></div>
                <input
                  :value="config.maxRecordSeconds"
                  type="number"
                  min="1"
                  max="600"
                  class="input input-bordered input-sm w-full"
                  @input="onMaxRecordSecondsInput"
                />
              </label>
            </div>
          </div>

          <div class="h-px bg-base-300"></div>

          <div class="grid gap-3">
            <div class="text-sm font-semibold">{{ t("config.hotkey.recordTest") }}</div>
            <div class="flex flex-wrap items-center gap-2">
              <div class="flex items-center gap-2 text-sm">
                <span class="opacity-70">{{ t("config.hotkey.microphonePermissionLabel") }}</span>
                <span class="badge" :class="microphonePermissionBadgeClass">
                  {{ microphonePermissionLabel }}
                </span>
              </div>
              <div class="h-6 w-px shrink-0 bg-base-300"></div>
              <button
                class="btn btn-sm bg-base-200 shrink-0"
                :disabled="microphonePermissionRequesting"
                @click="$emit('requestMicrophonePermission')"
              >
                <span v-if="microphonePermissionRequesting" class="loading loading-spinner loading-xs"></span>
                <span>{{ t("config.hotkey.requestMicrophonePermission") }}</span>
              </button>
              <button
                class="btn btn-sm bg-base-200 shrink-0"
                :class="{ 'btn-error text-error-content': hotkeyTestRecording }"
                :title="hotkeyTestRecording ? t('config.hotkey.releaseToStop') : t('config.hotkey.holdToRecord')"
                @mousedown.prevent="$emit('startHotkeyRecordTest')"
                @mouseup.prevent="$emit('stopHotkeyRecordTest')"
                @mouseleave.prevent="hotkeyTestRecording && $emit('stopHotkeyRecordTest')"
                @touchstart.prevent="$emit('startHotkeyRecordTest')"
                @touchend.prevent="$emit('stopHotkeyRecordTest')"
              >
                {{ hotkeyTestRecording ? t("chat.recording", { seconds: Math.max(1, Math.round(hotkeyTestRecordingMs / 1000)) }) : t("config.hotkey.holdRecordButton") }}
              </button>
              <button
                class="btn btn-sm bg-base-200 shrink-0"
                :disabled="!hotkeyTestAudioReady"
                @click="$emit('playHotkeyRecordTest')"
              >
                {{ t("config.hotkey.playRecord") }}
              </button>
            </div>
          </div>

          <div class="h-px bg-base-300"></div>

          <div class="grid gap-3">
            <div class="text-sm font-semibold">{{ t("config.chatSettings.backgroundVoiceScreenshotKeywords") }}</div>
            <div class="flex items-center gap-2">
              <input
                v-model="backgroundVoiceScreenshotKeywordsDraft"
                type="text"
                class="input input-bordered input-sm flex-1"
                :placeholder="t('config.chatSettings.backgroundVoiceScreenshotKeywordsPlaceholder')"
              />
              <button class="btn btn-sm btn-primary shrink-0" :disabled="!backgroundVoiceScreenshotDirty" @click="saveBackgroundVoiceScreenshotSettings">
                {{ t("common.save") }}
              </button>
            </div>
            <div class="text-xs opacity-70">{{ t("config.chatSettings.backgroundVoiceScreenshotKeywordsHint") }}</div>

            <div class="text-sm font-semibold">{{ t("config.chatSettings.backgroundVoiceScreenshotMode") }}</div>
            <SegmentedControl
              :model-value="backgroundVoiceScreenshotMode"
              :options="backgroundVoiceScreenshotModeOptions"
              size="sm"
              @change="onBackgroundVoiceScreenshotModeChange"
            />
            <div class="text-xs opacity-70">{{ t("config.chatSettings.backgroundVoiceScreenshotModeHint") }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 内置快捷键 -->
    <div class="card bg-base-100 border border-base-300">
      <div class="card-body p-4">
        <h3 class="card-title text-base mb-3">{{ t("config.hotkey.builtinShortcuts") }}</h3>
        <ul class="list bg-base-100 rounded-box">
          <li class="list-row">
            <span class="font-mono">Tab</span>
            <span class="list-col-grow opacity-60 text-right">{{ t("config.hotkey.builtinTabInstruction") }}</span>
          </li>
          <li class="list-row">
            <span class="font-mono">Esc</span>
            <span class="list-col-grow opacity-60 text-right">{{ t("config.hotkey.builtinEscStopReplying") }}</span>
          </li>
          <li class="list-row">
            <span class="font-mono">Shift + Wheel</span>
            <span class="list-col-grow opacity-60 text-right">{{ t("config.hotkey.builtinShiftWheelConversationSwitch") }}</span>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SegmentedControl from "../../components/SegmentedControl.vue";
import type { AppConfig, ChatSettingsPatch } from "../../../../types/app";

const props = defineProps<{
  config: AppConfig;
  hotkeyTestRecording: boolean;
  hotkeyTestRecordingMs: number;
  hotkeyTestAudioReady: boolean;
  microphonePermissionState: "granted" | "denied" | "prompt" | "unsupported" | "unknown";
  microphonePermissionRequesting: boolean;
  backgroundVoiceScreenshotKeywords: string;
  backgroundVoiceScreenshotMode: "desktop" | "focused_window";
}>();

const emit = defineEmits<{
  (e: "summonChatNow"): void;
  (e: "startHotkeyRecordTest"): void;
  (e: "stopHotkeyRecordTest"): void;
  (e: "playHotkeyRecordTest"): void;
  (e: "requestMicrophonePermission"): void;
  (e: "captureHotkey", value: string): void;
  (e: "update:recordHotkey", value: string): void;
  (e: "update:recordBackgroundWakeEnabled", value: boolean): void;
  (e: "update:minRecordSeconds", value: number): void;
  (e: "update:maxRecordSeconds", value: number): void;
  (e: "update:backgroundVoiceScreenshotKeywords", value: string): void;
  (e: "update:backgroundVoiceScreenshotMode", value: "desktop" | "focused_window"): void;
  (e: "patchChatSettings", value: ChatSettingsPatch): void;
}>();

const { t } = useI18n();
const backgroundVoiceScreenshotModeOptions = computed(() => [
  { value: "desktop" as const, label: t("config.chatSettings.backgroundVoiceScreenshotModeDesktop") },
  { value: "focused_window" as const, label: t("config.chatSettings.backgroundVoiceScreenshotModeFocusedWindow") },
]);

const microphonePermissionLabel = computed(() => {
  if (props.microphonePermissionState === "granted") return t("config.hotkey.microphonePermissionGranted");
  if (props.microphonePermissionState === "denied") return t("config.hotkey.microphonePermissionDenied");
  if (props.microphonePermissionState === "prompt") return t("config.hotkey.microphonePermissionPrompt");
  if (props.microphonePermissionState === "unsupported") return t("config.hotkey.microphonePermissionUnsupported");
  return t("config.hotkey.microphonePermissionUnknown");
});

const microphonePermissionBadgeClass = computed(() => {
  if (props.microphonePermissionState === "granted") return "badge-success";
  if (props.microphonePermissionState === "denied") return "badge-error";
  if (props.microphonePermissionState === "prompt") return "badge-warning";
  return "badge-ghost";
});

const hotkeyCapturing = ref(false);
const hotkeyCaptureHint = ref(t("config.hotkey.captureDefaultHint"));
let hotkeyCaptureHandler: ((event: KeyboardEvent) => void) | null = null;
const recordHotkeyCapturing = ref(false);
let recordHotkeyCaptureHandler: ((event: KeyboardEvent) => void) | null = null;
const backgroundVoiceScreenshotKeywordsDraft = ref(String(props.backgroundVoiceScreenshotKeywords || ""));

watch(
  () => props.backgroundVoiceScreenshotKeywords,
  (value) => {
    backgroundVoiceScreenshotKeywordsDraft.value = String(value || "");
  },
);

const backgroundVoiceScreenshotDirty = computed(
  () => backgroundVoiceScreenshotKeywordsDraft.value !== String(props.backgroundVoiceScreenshotKeywords || ""),
);

function onMinRecordSecondsInput(event: Event) {
  const raw = Number((event.target as HTMLInputElement).value);
  emit("update:minRecordSeconds", raw);
}

function onMaxRecordSecondsInput(event: Event) {
  const raw = Number((event.target as HTMLInputElement).value);
  emit("update:maxRecordSeconds", raw);
}

function saveBackgroundVoiceScreenshotSettings() {
  const keywords = backgroundVoiceScreenshotKeywordsDraft.value.replace(/，/g, ",");
  backgroundVoiceScreenshotKeywordsDraft.value = keywords;
  emit("update:backgroundVoiceScreenshotKeywords", keywords);
  emit("patchChatSettings", {
    backgroundVoiceScreenshotKeywords: keywords,
  });
}

function onBackgroundVoiceScreenshotModeChange(value: "desktop" | "focused_window") {
  emit("update:backgroundVoiceScreenshotMode", value);
  emit("patchChatSettings", {
    backgroundVoiceScreenshotMode: value,
  });
}

function isModifierKey(code: string): boolean {
  return code === "AltLeft"
    || code === "AltRight"
    || code === "ControlLeft"
    || code === "ControlRight"
    || code === "ShiftLeft"
    || code === "ShiftRight"
    || code === "MetaLeft"
    || code === "MetaRight";
}

function mainKeyFromEvent(event: KeyboardEvent): string {
  const code = event.code || "";
  if (code === "Backquote") return "·";
  if (code.startsWith("Key") && code.length === 4) return code.slice(3).toUpperCase();
  if (code.startsWith("Digit") && code.length === 6) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (code === "Minus") return "-";
  if (code === "Equal") return "=";
  if (code === "BracketLeft") return "[";
  if (code === "BracketRight") return "]";
  if (code === "Backslash") return "\\";
  if (code === "Semicolon") return ";";
  if (code === "Quote") return "'";
  if (code === "Comma") return ",";
  if (code === "Period") return ".";
  if (code === "Slash") return "/";
  if (code === "Space") return "Space";
  const key = event.key || "";
  if (key.length === 1) return key.toUpperCase();
  return key;
}

function stopHotkeyCapture() {
  hotkeyCapturing.value = false;
  if (hotkeyCaptureHandler) {
    window.removeEventListener("keydown", hotkeyCaptureHandler, true);
    hotkeyCaptureHandler = null;
  }
}

function stopRecordHotkeyCapture() {
  recordHotkeyCapturing.value = false;
  if (recordHotkeyCaptureHandler) {
    window.removeEventListener("keydown", recordHotkeyCaptureHandler, true);
    recordHotkeyCaptureHandler = null;
  }
}

function startHotkeyCapture() {
  if (hotkeyCapturing.value) return;
  hotkeyCapturing.value = true;
  hotkeyCaptureHint.value = t("config.hotkey.captureListeningHint");
  hotkeyCaptureHandler = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      hotkeyCaptureHint.value = t("config.hotkey.captureCancelledHint");
      stopHotkeyCapture();
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");
    if (event.metaKey) modifiers.push("Meta");

    if (isModifierKey(event.code)) {
      hotkeyCaptureHint.value = t("config.hotkey.captureNeedMainKeyHint");
      return;
    }
    if (modifiers.length === 0) {
      hotkeyCaptureHint.value = t("config.hotkey.captureNeedModifierHint");
      return;
    }

    const main = mainKeyFromEvent(event).trim();
    if (!main) {
      hotkeyCaptureHint.value = t("config.hotkey.captureUnrecognizedHint");
      return;
    }
    const combo = `${modifiers.join("+")}+${main}`;
    emit("captureHotkey", combo);
    hotkeyCaptureHint.value = t("config.hotkey.captureCapturedHint", { combo });
    stopHotkeyCapture();
  };
  window.addEventListener("keydown", hotkeyCaptureHandler, true);
}

function startRecordHotkeyCapture() {
  if (recordHotkeyCapturing.value) return;
  recordHotkeyCapturing.value = true;
  recordHotkeyCaptureHandler = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      emit("update:recordHotkey", "");
      stopRecordHotkeyCapture();
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");
    if (event.metaKey) modifiers.push("Meta");

    if (isModifierKey(event.code)) {
      const modifierOnly = modifiers[0];
      if (modifiers.length === 1 && modifierOnly) {
        emit("update:recordHotkey", modifierOnly);
        stopRecordHotkeyCapture();
      }
      return;
    }

    const main = mainKeyFromEvent(event).trim();
    if (!main) return;
    emit("update:recordHotkey", modifiers.length > 0 ? `${modifiers.join("+")}+${main}` : main);
    stopRecordHotkeyCapture();
  };
  window.addEventListener("keydown", recordHotkeyCaptureHandler, true);
}

function toggleHotkeyCapture() {
  if (hotkeyCapturing.value) {
    hotkeyCaptureHint.value = t("config.hotkey.captureCancelledHint");
    stopHotkeyCapture();
    return;
  }
  startHotkeyCapture();
}

function toggleRecordHotkeyCapture() {
  if (recordHotkeyCapturing.value) {
    stopRecordHotkeyCapture();
    return;
  }
  startRecordHotkeyCapture();
}

onBeforeUnmount(() => {
  stopHotkeyCapture();
  stopRecordHotkeyCapture();
});
</script>
