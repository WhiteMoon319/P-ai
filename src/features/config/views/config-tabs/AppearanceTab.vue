<template>
  <div class="grid gap-2">
    <div class="card bg-base-100 border border-base-300">
      <div class="card-body p-4">
        <div class="grid grid-cols-1 gap-3">
          <div class="space-y-2">
            <h3 class="card-title text-base">{{ t("appearance.language") }}</h3>
            <select
              class="select select-bordered w-full"
              :value="props.uiLanguage"
              @change="$emit('update:uiLanguage', ($event.target as HTMLSelectElement).value)"
            >
              <option v-for="opt in props.localeOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="card-body gap-3 p-4">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h3 class="card-title text-base">{{ t("appearance.markdownFontScale") }}</h3>
            <p class="mt-1 text-xs text-base-content/60">{{ t("appearance.markdownFontScaleHint") }}</p>
          </div>
        </div>
        <SegmentedControl
          :model-value="markdownFontScale < 1 ? 0 : 1"
          :options="markdownFontScaleOptions"
          @change="setMarkdownFontScale"
        />
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="card-body gap-3 p-4">
        <h3 class="card-title text-base">{{ t("appearance.chatBubble") }}</h3>
        <div class="grid gap-3">
          <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box bg-base-200/40 px-3 py-2">
            <span class="text-sm font-medium">{{ t("appearance.chatBubbleBackground") }}</span>
            <input
              :checked="assistantBubbleBackgroundEnabled"
              type="checkbox"
              class="toggle toggle-sm"
              @change="setAssistantBubbleBackgroundEnabled(($event.target as HTMLInputElement).checked)"
            />
          </label>
          <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box bg-base-200/40 px-3 py-2">
            <span class="text-sm font-medium">{{ t("appearance.chatBubbleSegmentedMarkdown") }}</span>
            <input
              :checked="segmentedMarkdownEnabled"
              type="checkbox"
              class="toggle toggle-sm"
              @change="setSegmentedMarkdownEnabled(($event.target as HTMLInputElement).checked)"
            />
          </label>
          <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box bg-base-200/40 px-3 py-2">
            <span class="text-sm font-medium">{{ t("appearance.chatBubbleFullTime") }}</span>
            <input
              :checked="chatTimeDisplayMode === 'absolute'"
              type="checkbox"
              class="toggle toggle-sm"
              @change="setChatTimeDisplayMode(($event.target as HTMLInputElement).checked ? 'absolute' : 'relative')"
            />
          </label>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="card-body gap-3 p-4">
        <h3 class="card-title text-base">{{ t("appearance.inputPanel") }}</h3>
        <div class="grid gap-3">
          <label v-if="SIDE_FILE_TAGS_AVAILABLE" class="flex cursor-pointer items-center justify-between gap-3 rounded-box bg-base-200/40 px-3 py-2">
            <span class="text-sm font-medium">{{ t("appearance.inputPanelSideFileTags") }}</span>
            <input
              :checked="sideFileTagsEnabled"
              type="checkbox"
              class="toggle toggle-sm"
              @change="setSideFileTagsEnabled(($event.target as HTMLInputElement).checked)"
            />
          </label>
          <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box bg-base-200/40 px-3 py-2">
            <span class="text-sm font-medium">{{ t("appearance.inputPanelIdeBridgeFileTags") }}</span>
            <input
              :checked="ideBridgeFileTagsEnabled"
              type="checkbox"
              class="toggle toggle-sm"
              @change="setIdeBridgeFileTagsEnabled(($event.target as HTMLInputElement).checked)"
            />
          </label>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="card-body gap-3 p-4">
        <div>
          <h3 class="card-title text-base">{{ t("appearance.uiSizeScale") }}</h3>
          <p class="mt-1 text-xs text-base-content/60">{{ t("appearance.uiSizeScaleHint") }}</p>
        </div>
        <div class="grid gap-2">
          <div class="flex items-center gap-3">
            <input
              class="range range-primary flex-1"
              type="range"
              min="75"
              max="150"
              step="1"
              :value="uiSizeScale"
              :aria-label="t('appearance.uiSizeScale')"
              @input="$emit('update:uiSizeScale', Number(($event.target as HTMLInputElement).value))"
            />
            <output class="w-12 text-right text-sm font-medium tabular-nums">{{ uiSizeScale }}%</output>
          </div>
          <div class="flex justify-between px-0.5 text-caption text-base-content/55 tabular-nums">
            <span>75%</span>
            <span>100%</span>
            <span>125%</span>
            <span>150%</span>
          </div>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="card-body gap-4 p-4">
        <h3 class="card-title text-base">{{ t("appearance.theme") }}</h3>

        <div class="tabs tabs-box bg-base-200 p-1">
          <button
            type="button"
            class="tab flex-1 rounded-btn"
            :class="activeTab === 'preset' ? 'tab-active' : ''"
            @click="activeTab = 'preset'"
          >
            {{ t("appearance.themeTabs.preset") }}
          </button>
          <button
            type="button"
            class="tab flex-1 rounded-btn"
            :class="activeTab === 'generated' ? 'tab-active' : ''"
            @click="activateGeneratedTab"
          >
            {{ t("appearance.themeTabs.generated") }}
          </button>
        </div>

        <ThemePreviewGrid
          v-if="activeTab === 'preset'"
          :light-themes="lightThemes"
          :dark-themes="darkThemes"
          :current-theme="props.currentTheme"
          @select="$emit('setTheme', $event)"
        />

        <GeneratedThemeEditor
          v-else
          :controls="props.generatedThemeControls"
          :tokens="props.generatedThemeTokens"
          @update-controls="$emit('updateGeneratedThemeControls', $event)"
          @reset="$emit('resetGeneratedTheme')"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SegmentedControl from "../../components/SegmentedControl.vue";
import ThemePreviewGrid from "../../components/ThemePreviewGrid.vue";
import GeneratedThemeEditor from "../../components/GeneratedThemeEditor.vue";
import {
  APP_THEMES,
  DARK_APP_THEMES,
} from "../../../shell/composables/use-app-theme";
import type { GeneratedThemeControls, GeneratedThemeTokens } from "../../../shell/theme/theme-types";
import {
  GENERATED_THEME_DARK_ID,
  GENERATED_THEME_LIGHT_ID,
} from "../../../shell/theme/theme-generator";
import {
  useMarkdownAppearance,
} from "../../../shell/composables/use-markdown-appearance";
import { useChatMessageAppearance } from "../../../shell/composables/use-chat-message-appearance";
import { SIDE_FILE_TAGS_AVAILABLE, useChatComposerAppearance } from "../../../shell/composables/use-chat-composer-appearance";

const props = defineProps<{
  uiLanguage: "zh-CN" | "en-US" | "zh-TW";
  localeOptions: Array<{ value: "zh-CN" | "en-US" | "zh-TW"; label: string }>;
  currentTheme: string;
  generatedThemeControls: GeneratedThemeControls;
  generatedThemeTokens: GeneratedThemeTokens;
  uiSizeScale: number;
}>();

const emit = defineEmits<{
  (e: "update:uiLanguage", value: string): void;
  (e: "update:uiSizeScale", value: number): void;
  (e: "setTheme", value: string): void;
  (e: "activateGeneratedTheme"): void;
  (e: "updateGeneratedThemeControls", value: Partial<GeneratedThemeControls>): void;
  (e: "resetGeneratedTheme"): void;
}>();

const { t } = useI18n();
const activeTab = ref<"preset" | "generated">("generated");
const markdownFontScaleOptions = computed(() => [
  { value: 0, label: t("appearance.markdownFontScaleLight") },
  { value: 1, label: t("appearance.markdownFontScaleHeavy") },
]);
const lightThemes = computed(() => APP_THEMES.filter((theme) => !DARK_APP_THEMES.has(theme)));
const darkThemes = computed(() => APP_THEMES.filter((theme) => DARK_APP_THEMES.has(theme)));
const {
  markdownFontScale,
  setMarkdownFontScale,
} = useMarkdownAppearance();
const {
  assistantBubbleBackgroundEnabled,
  segmentedMarkdownEnabled,
  chatTimeDisplayMode,
  setAssistantBubbleBackgroundEnabled,
  setSegmentedMarkdownEnabled,
  setChatTimeDisplayMode,
} = useChatMessageAppearance();
const {
  sideFileTagsEnabled,
  ideBridgeFileTagsEnabled,
  setSideFileTagsEnabled,
  setIdeBridgeFileTagsEnabled,
} = useChatComposerAppearance();

function isGeneratedTheme(theme: string) {
  return theme === GENERATED_THEME_LIGHT_ID || theme === GENERATED_THEME_DARK_ID;
}

function activateGeneratedTab() {
  activeTab.value = "generated";
  if (!isGeneratedTheme(props.currentTheme)) {
    emit("activateGeneratedTheme");
  }
}

watch(
  () => props.currentTheme,
  (theme) => {
    activeTab.value = isGeneratedTheme(theme) ? "generated" : "preset";
  },
  { immediate: true },
);
</script>
