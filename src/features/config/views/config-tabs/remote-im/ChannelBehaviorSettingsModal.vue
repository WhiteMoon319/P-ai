<template>
  <button
    type="button"
    class="btn btn-square btn-ghost"
    :title="t('config.remoteIm.channelBehaviorSettings')"
    :disabled="!channel"
    @click="openModal"
  >
    <SlidersHorizontal class="h-3.5 w-3.5" />
  </button>

  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-[100] flex items-center justify-center bg-black/40 p-4" @mousedown.self="closeModal">
      <section class="flex h-[82vh] w-[82vw] max-w-none flex-col overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-2xl" role="dialog" aria-modal="true">
        <header class="flex shrink-0 items-start justify-between gap-4 border-b border-base-300 px-5 py-4">
          <div class="min-w-0">
            <h3 class="font-semibold">{{ t('config.remoteIm.channelBehaviorSettings') }}</h3>
            <p class="mt-1 text-xs opacity-60">{{ t('config.remoteIm.channelBehaviorSettingsHint') }}</p>
          </div>
          <button type="button" class="btn btn-circle btn-sm btn-ghost" :title="t('common.close')" @click="closeModal">×</button>
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          <div v-if="error" class="alert alert-error mb-4 py-2 text-xs">{{ error }}</div>

          <section class="space-y-3">
            <h4 class="font-semibold">{{ t('config.remoteIm.channelBehaviorGeneralSection') }}</h4>
            <div class="grid gap-3 md:grid-cols-2">
              <label class="form-control md:col-span-2">
                <span class="label-text text-xs">{{ t('config.remoteIm.blockedMessagePrefixes') }}</span>
                <input v-model="draft.blockedMessagePrefixesText" class="input input-sm input-bordered" :placeholder="t('config.remoteIm.blockedMessagePrefixesPlaceholder')" />
                <span class="label-text-alt text-xs opacity-60">{{ t('config.remoteIm.blockedMessagePrefixesHint') }}</span>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ t('config.remoteIm.muteKeywords') }}</span>
                <input v-model="draft.muteKeywordsText" class="input input-sm input-bordered" :placeholder="t('config.remoteIm.muteKeywordsPlaceholder')" />
                <span class="label-text-alt text-xs opacity-60">{{ t('config.remoteIm.muteKeywordsHint') }}</span>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ t('config.remoteIm.unmuteKeywords') }}</span>
                <input v-model="draft.unmuteKeywordsText" class="input input-sm input-bordered" :placeholder="t('config.remoteIm.unmuteKeywordsPlaceholder')" />
                <span class="label-text-alt text-xs opacity-60">{{ t('config.remoteIm.unmuteKeywordsHint') }}</span>
              </label>
              <NumberField v-model="draft.patienceSeconds" :label="t('config.remoteIm.patienceExit')" :min="0" />
              <NumberField v-model="draft.muteDurationSeconds" :label="t('config.remoteIm.muteDuration')" :min="0" />
              <NumberField v-model="draft.activationCooldownSeconds" :label="t('config.remoteIm.activationCooldownSeconds')" :min="0" />
            </div>
          </section>

          <div class="divider my-6">{{ t('config.remoteIm.channelBehaviorGroupSection') }}</div>

          <section class="space-y-5">
            <div>
              <h4 class="mb-3 font-semibold">{{ t('config.remoteIm.behaviorInspectionSection') }}</h4>
              <div class="grid gap-3 md:grid-cols-2">
                <NumberField v-model="draft.pacing.assistantDebounceSeconds" :label="t('config.remoteIm.assistantDebounceSeconds')" :min="1" />
                <NumberField v-model="draft.pacing.secretaryInspectionSeconds" :label="t('config.remoteIm.secretaryInspectionSeconds')" :min="1" />
                <NumberField v-model="draft.pacing.replyCooldownSeconds" :label="t('config.remoteIm.replyCooldownSeconds')" :min="0" />
                <NumberField v-model="draft.pacing.inspectionJitterRatio" :label="t('config.remoteIm.inspectionJitterRatio')" :min="0" :max="1" :step="0.05" />
              </div>
            </div>

            <div>
              <h4 class="mb-3 font-semibold">{{ t('config.remoteIm.behaviorEnergySection') }}</h4>
              <div class="grid gap-3 md:grid-cols-2">
                <NumberField v-model="draft.pacing.maximumEnergy" :label="t('config.remoteIm.maximumEnergy')" :min="0.01" :step="1" />
                <NumberField v-model="draft.pacing.baseReplyEnergyCost" :label="t('config.remoteIm.baseReplyEnergyCost')" :min="0" :step="0.1" />
                <NumberField v-model="draft.pacing.energyCostPerCharacter" :label="t('config.remoteIm.energyCostPerCharacter')" :min="0" :step="0.01" />
                <NumberField v-model="draft.pacing.energyRecoveryPerSecond" :label="t('config.remoteIm.energyRecoveryPerSecond')" :min="0" :step="0.01" />
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.positiveEnergyPhrases') }}</span>
                  <input v-model="draft.positiveEnergyPhrasesText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="draft.pacing.positiveEnergyDelta" :label="t('config.remoteIm.positiveEnergyDelta')" :min="0" :step="0.1" />
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.negativeEnergyPhrases') }}</span>
                  <input v-model="draft.negativeEnergyPhrasesText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="draft.pacing.negativeEnergyDelta" :label="t('config.remoteIm.negativeEnergyDelta')" :max="0" :step="0.1" />
              </div>
            </div>

            <div>
              <h4 class="mb-3 font-semibold">{{ t('config.remoteIm.behaviorFocusSection') }}</h4>
              <div class="grid gap-3 md:grid-cols-2">
                <label class="form-control md:col-span-2">
                  <span class="label-text text-xs">{{ t('config.remoteIm.focusInstructions') }}</span>
                  <input v-model="draft.focusInstructionsText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="draft.pacing.normalReplyMaxChars" :label="t('config.remoteIm.normalReplyMaxChars')" :min="1" />
                <NumberField v-model="draft.pacing.focusReplyMaxChars" :label="t('config.remoteIm.focusReplyMaxChars')" :min="1" />
                <div class="rounded-box bg-base-200 p-3 text-xs md:col-span-2">
                  <div>{{ t('config.remoteIm.normalReminderPreview', { count: draft.pacing.normalReplyMaxChars }) }}</div>
                  <div>{{ t('config.remoteIm.focusReminderPreview', { count: draft.pacing.focusReplyMaxChars }) }}</div>
                </div>
              </div>
            </div>
          </section>
        </div>

        <footer class="flex shrink-0 flex-wrap justify-end gap-2 border-t border-base-300 px-5 py-4">
          <button type="button" class="btn btn-ghost" :disabled="saving" @click="restoreDefaults">{{ t('config.remoteIm.restoreBehaviorDefaults') }}</button>
          <button type="button" class="btn btn-ghost" :disabled="saving || !savedSnapshot" @click="restoreSaved">{{ t('config.remoteIm.restoreBehaviorSaved') }}</button>
          <button type="button" class="btn btn-primary" :disabled="saving || !dirty || !!validationError" @click="save">
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            <Save v-else class="h-3.5 w-3.5" />{{ t('common.save') }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, ref } from "vue";
import { Save, SlidersHorizontal } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../../services/tauri-api";
import type { RemoteImChannelBehaviorSettings, RemoteImChannelConfig, RemoteImGroupReplyPacing } from "../../../../../types/app";
import {
  cloneChannelBehaviorSettings,
  DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS,
  normalizeGroupReplyPacing,
  parseSpaceSeparatedList,
} from "./helpers";

const props = defineProps<{
  channel: RemoteImChannelConfig | null;
  saveConfigAction: () => Promise<boolean> | boolean;
  setStatusAction: (text: string) => void;
}>();
const { t } = useI18n();

type Draft = {
  blockedMessagePrefixesText: string;
  muteKeywordsText: string;
  unmuteKeywordsText: string;
  patienceSeconds: number;
  muteDurationSeconds: number;
  activationCooldownSeconds: number;
  positiveEnergyPhrasesText: string;
  negativeEnergyPhrasesText: string;
  focusInstructionsText: string;
  pacing: RemoteImGroupReplyPacing;
};

const NumberField = defineComponent({
  props: { modelValue: Number, label: String, min: Number, max: Number, step: Number },
  emits: ["update:modelValue"],
  setup(fieldProps, { emit }) {
    return () => h("label", { class: "form-control" }, [
      h("span", { class: "label-text text-xs" }, fieldProps.label || ""),
      h("input", {
        class: "input input-sm input-bordered",
        type: "number",
        value: fieldProps.modelValue,
        min: fieldProps.min,
        max: fieldProps.max,
        step: fieldProps.step ?? 1,
        onInput: (event: Event) => emit("update:modelValue", Number((event.target as HTMLInputElement).value)),
      }),
    ]);
  },
});

function draftFromSettings(value?: Partial<RemoteImChannelBehaviorSettings> | null): Draft {
  const settings = cloneChannelBehaviorSettings(value);
  const pacing = normalizeGroupReplyPacing(settings.groupReplyPacing);
  return {
    blockedMessagePrefixesText: settings.blockedMessagePrefixes.join(" "),
    muteKeywordsText: settings.muteKeywords.join(" "),
    unmuteKeywordsText: settings.unmuteKeywords.join(" "),
    patienceSeconds: settings.patienceSeconds,
    muteDurationSeconds: settings.muteDurationSeconds,
    activationCooldownSeconds: settings.activationCooldownSeconds,
    positiveEnergyPhrasesText: pacing.positiveEnergyPhrases.join(" "),
    negativeEnergyPhrasesText: pacing.negativeEnergyPhrases.join(" "),
    focusInstructionsText: pacing.focusInstructions.join(" "),
    pacing,
  };
}

function settingsFromDraft(value: Draft): RemoteImChannelBehaviorSettings {
  return {
    blockedMessagePrefixes: parseSpaceSeparatedList(value.blockedMessagePrefixesText),
    muteKeywords: parseSpaceSeparatedList(value.muteKeywordsText),
    unmuteKeywords: parseSpaceSeparatedList(value.unmuteKeywordsText),
    patienceSeconds: Math.max(0, Math.floor(Number(value.patienceSeconds) || 0)),
    muteDurationSeconds: Math.max(0, Math.floor(Number(value.muteDurationSeconds) || 0)),
    activationCooldownSeconds: Math.max(0, Math.floor(Number(value.activationCooldownSeconds) || 0)),
    groupReplyPacing: {
      ...value.pacing,
      positiveEnergyPhrases: parseSpaceSeparatedList(value.positiveEnergyPhrasesText),
      negativeEnergyPhrases: parseSpaceSeparatedList(value.negativeEnergyPhrasesText),
      focusInstructions: parseSpaceSeparatedList(value.focusInstructionsText),
    },
  };
}

const open = ref(false);
const draft = ref<Draft>(draftFromSettings());
const savedSnapshot = ref("");
const editingChannelId = ref("");
const saving = ref(false);
const error = ref("");
const draftSnapshot = computed(() => JSON.stringify(draft.value));
const dirty = computed(() => !!savedSnapshot.value && draftSnapshot.value !== savedSnapshot.value);
const validationError = computed(() => {
  const common = [draft.value.patienceSeconds, draft.value.muteDurationSeconds, draft.value.activationCooldownSeconds];
  if (common.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  const p = draft.value.pacing;
  const group = [p.assistantDebounceSeconds, p.secretaryInspectionSeconds, p.replyCooldownSeconds, p.inspectionJitterRatio, p.maximumEnergy, p.baseReplyEnergyCost, p.energyCostPerCharacter, p.energyRecoveryPerSecond, p.positiveEnergyDelta, p.negativeEnergyDelta, p.normalReplyMaxChars, p.focusReplyMaxChars];
  if (group.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  if (p.assistantDebounceSeconds < 1 || p.secretaryInspectionSeconds < 1) return t("config.remoteIm.behaviorPeriodError");
  if (p.inspectionJitterRatio < 0 || p.inspectionJitterRatio > 1) return t("config.remoteIm.behaviorJitterError");
  if (p.maximumEnergy <= 0 || p.baseReplyEnergyCost < 0 || p.energyCostPerCharacter < 0 || p.energyRecoveryPerSecond < 0 || p.positiveEnergyDelta < 0 || p.negativeEnergyDelta > 0) return t("config.remoteIm.behaviorEnergyError");
  if (p.normalReplyMaxChars < 1 || p.focusReplyMaxChars < p.normalReplyMaxChars) return t("config.remoteIm.behaviorLengthError");
  return "";
});

function openModal() {
  if (!props.channel) return;
  draft.value = draftFromSettings(props.channel.behaviorSettings);
  savedSnapshot.value = JSON.stringify(draft.value);
  editingChannelId.value = props.channel.id;
  error.value = "";
  open.value = true;
}

function closeModal() {
  if (!saving.value) open.value = false;
}

function restoreDefaults() {
  draft.value = draftFromSettings(DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS);
  error.value = "";
}

function restoreSaved() {
  if (!savedSnapshot.value) return;
  try {
    draft.value = JSON.parse(savedSnapshot.value) as Draft;
    error.value = "";
  } catch {
    draft.value = draftFromSettings();
    error.value = "";
  }
}

async function save() {
  if (saving.value || !dirty.value || validationError.value) return;
  const channel = props.channel;
  if (!channel || channel.id !== editingChannelId.value) {
    error.value = t("config.remoteIm.channelBehaviorChannelChanged");
    return;
  }
  saving.value = true;
  error.value = "";
  const submittedSnapshot = draftSnapshot.value;
  const previous = channel.behaviorSettings
    ? cloneChannelBehaviorSettings(channel.behaviorSettings)
    : undefined;
  const next = settingsFromDraft(draft.value);
  channel.behaviorSettings = next;
  try {
    const saved = await Promise.resolve(props.saveConfigAction());
    if (!saved) {
      channel.behaviorSettings = previous;
      error.value = t("config.remoteIm.channelBehaviorSaveFailed");
      return;
    }
    savedSnapshot.value = JSON.stringify(draftFromSettings(next));
    if (draftSnapshot.value === submittedSnapshot) draft.value = draftFromSettings(next);
    props.setStatusAction(t("config.remoteIm.channelBehaviorSaved"));
    try {
      await invokeTauri("remote_im_reconfigure_channel_behavior", { channelId: channel.id });
    } catch (reconfigureError) {
      console.warn("[远程IM] channel behavior reconfigure deferred:", reconfigureError);
      props.setStatusAction(t("config.remoteIm.channelBehaviorSavedReconfigureDeferred"));
    }
  } catch (saveError) {
    channel.behaviorSettings = previous;
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}
</script>
