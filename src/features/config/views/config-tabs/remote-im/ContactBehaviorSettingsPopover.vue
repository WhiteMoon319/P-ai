<template>
  <div class="dropdown dropdown-end">
    <button
      type="button"
      tabindex="0"
      class="btn btn-square btn-ghost"
      :title="t('config.remoteIm.behaviorSettings')"
    >
      <SlidersHorizontal class="h-3.5 w-3.5" />
    </button>
    <div tabindex="0" class="dropdown-content z-40 mt-2 w-[min(44rem,calc(100vw-2rem))] rounded-box border border-base-300 bg-base-100 p-3 shadow-xl">
      <div class="mb-3 flex items-center justify-between gap-3">
        <div>
          <div class="font-semibold">{{ t('config.remoteIm.behaviorSettings') }}</div>
          <div class="text-xs opacity-60">{{ t('config.remoteIm.behaviorSettingsHint') }}</div>
        </div>
        <label class="label cursor-pointer gap-2 py-0 text-xs">
          <span>{{ t('config.remoteIm.includePrivateContacts') }}</span>
          <input v-model="showPrivate" type="checkbox" class="toggle toggle-xs" />
        </label>
      </div>
      <input v-model="search" class="input input-sm input-bordered mb-3 w-full" :placeholder="t('config.remoteIm.behaviorSearch')" />
      <div class="max-h-[70vh] space-y-2 overflow-y-auto pr-1">
        <div v-if="filteredContacts.length === 0" class="py-6 text-center text-sm opacity-60">
          {{ t('config.remoteIm.contactsEmpty') }}
        </div>
        <div v-for="contact in filteredContacts" :key="contact.id" class="collapse collapse-arrow border border-base-300 bg-base-100">
          <input type="checkbox" />
          <div class="collapse-title flex items-center gap-2 py-3 pr-10">
            <div class="avatar placeholder shrink-0">
              <div class="h-8 w-8 overflow-hidden rounded-full bg-base-200 text-xs">
                <img v-if="contact.avatarUrl" :src="contact.avatarUrl" class="h-full w-full object-cover" />
                <span v-else>{{ displayName(contact).slice(0, 1) }}</span>
              </div>
            </div>
            <span class="min-w-0 flex-1 truncate font-medium">{{ displayName(contact) }}</span>
            <span class="badge badge-sm" :class="isGroup(contact) ? 'badge-secondary' : 'badge-primary'">
              {{ isGroup(contact) ? t('config.remoteIm.groupContact') : t('config.remoteIm.privateContact') }}
            </span>
            <span v-if="isDirty(contact.id)" class="badge badge-warning badge-sm">{{ t('config.remoteIm.behaviorUnsaved') }}</span>
          </div>
          <div class="collapse-content space-y-4 text-sm">
            <div v-if="errors[contact.id]" class="alert alert-error py-2 text-xs">{{ errors[contact.id] }}</div>

            <section>
              <h4 class="mb-2 font-semibold">{{ t('config.remoteIm.behaviorPresenceSection') }}</h4>
              <div class="grid gap-2 sm:grid-cols-2">
                <label class="form-control sm:col-span-2">
                  <span class="label-text text-xs">{{ t('config.remoteIm.blockedMessagePrefixes') }}</span>
                  <input v-model="drafts[contact.id].blockedMessagePrefixesText" class="input input-sm input-bordered" />
                </label>
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.muteKeywords') }}</span>
                  <input v-model="drafts[contact.id].muteKeywordsText" class="input input-sm input-bordered" />
                </label>
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.unmuteKeywords') }}</span>
                  <input v-model="drafts[contact.id].unmuteKeywordsText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="drafts[contact.id].patienceSeconds" :label="t('config.remoteIm.patienceSeconds')" :min="0" />
                <NumberField v-model="drafts[contact.id].muteDurationSeconds" :label="t('config.remoteIm.muteDurationSeconds')" :min="0" />
                <NumberField v-model="drafts[contact.id].activationCooldownSeconds" :label="t('config.remoteIm.activationCooldownSeconds')" :min="0" />
              </div>
            </section>

            <section :class="!isGroup(contact) ? 'opacity-50' : ''">
              <h4 class="mb-1 font-semibold">{{ t('config.remoteIm.behaviorInspectionSection') }}</h4>
              <p v-if="!isGroup(contact)" class="mb-2 text-xs text-warning">{{ t('config.remoteIm.groupOnlySetting') }}</p>
              <fieldset :disabled="!isGroup(contact)" class="grid gap-2 sm:grid-cols-2">
                <NumberField v-model="drafts[contact.id].pacing.assistantDebounceSeconds" :label="t('config.remoteIm.assistantDebounceSeconds')" :min="1" />
                <NumberField v-model="drafts[contact.id].pacing.secretaryInspectionSeconds" :label="t('config.remoteIm.secretaryInspectionSeconds')" :min="1" />
                <NumberField v-model="drafts[contact.id].pacing.replyCooldownSeconds" :label="t('config.remoteIm.replyCooldownSeconds')" :min="0" />
                <NumberField v-model="drafts[contact.id].pacing.inspectionJitterRatio" :label="t('config.remoteIm.inspectionJitterRatio')" :min="0" :max="1" :step="0.05" />
              </fieldset>
            </section>

            <section :class="!isGroup(contact) ? 'opacity-50' : ''">
              <h4 class="mb-2 font-semibold">{{ t('config.remoteIm.behaviorEnergySection') }}</h4>
              <fieldset :disabled="!isGroup(contact)" class="grid gap-2 sm:grid-cols-2">
                <NumberField v-model="drafts[contact.id].pacing.maximumEnergy" :label="t('config.remoteIm.maximumEnergy')" :min="0.01" :step="1" />
                <NumberField v-model="drafts[contact.id].pacing.baseReplyEnergyCost" :label="t('config.remoteIm.baseReplyEnergyCost')" :min="0" :step="0.1" />
                <NumberField v-model="drafts[contact.id].pacing.energyCostPerCharacter" :label="t('config.remoteIm.energyCostPerCharacter')" :min="0" :step="0.01" />
                <NumberField v-model="drafts[contact.id].pacing.energyRecoveryPerSecond" :label="t('config.remoteIm.energyRecoveryPerSecond')" :min="0" :step="0.01" />
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.positiveEnergyPhrases') }}</span>
                  <input v-model="drafts[contact.id].positiveEnergyPhrasesText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="drafts[contact.id].pacing.positiveEnergyDelta" :label="t('config.remoteIm.positiveEnergyDelta')" :min="0" :step="0.1" />
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('config.remoteIm.negativeEnergyPhrases') }}</span>
                  <input v-model="drafts[contact.id].negativeEnergyPhrasesText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="drafts[contact.id].pacing.negativeEnergyDelta" :label="t('config.remoteIm.negativeEnergyDelta')" :max="0" :step="0.1" />
              </fieldset>
            </section>

            <section :class="!isGroup(contact) ? 'opacity-50' : ''">
              <h4 class="mb-2 font-semibold">{{ t('config.remoteIm.behaviorFocusSection') }}</h4>
              <fieldset :disabled="!isGroup(contact)" class="grid gap-2 sm:grid-cols-2">
                <label class="form-control sm:col-span-2">
                  <span class="label-text text-xs">{{ t('config.remoteIm.focusInstructions') }}</span>
                  <input v-model="drafts[contact.id].focusInstructionsText" class="input input-sm input-bordered" />
                </label>
                <NumberField v-model="drafts[contact.id].pacing.normalReplyMaxChars" :label="t('config.remoteIm.normalReplyMaxChars')" :min="1" />
                <NumberField v-model="drafts[contact.id].pacing.focusReplyMaxChars" :label="t('config.remoteIm.focusReplyMaxChars')" :min="1" />
                <div class="sm:col-span-2 rounded-box bg-base-200 p-2 text-xs">
                  <div>{{ t('config.remoteIm.normalReminderPreview', { count: drafts[contact.id].pacing.normalReplyMaxChars }) }}</div>
                  <div>{{ t('config.remoteIm.focusReminderPreview', { count: drafts[contact.id].pacing.focusReplyMaxChars }) }}</div>
                </div>
              </fieldset>
            </section>

            <div class="flex flex-wrap justify-end gap-2">
              <button class="btn btn-sm btn-ghost" type="button" @click="copyBehavior(contact.id)"><Copy class="h-3.5 w-3.5" />{{ t('common.copy') }}</button>
              <button class="btn btn-sm btn-ghost" type="button" :disabled="!clipboard" @click="pasteBehavior(contact)"><ClipboardPaste class="h-3.5 w-3.5" />{{ t('common.paste') }}</button>
              <button class="btn btn-sm btn-ghost" type="button" :disabled="!isDirty(contact.id) || !!saving[contact.id]" @click="resetDraft(contact)"><RotateCcw class="h-3.5 w-3.5" />{{ t('common.reset') }}</button>
              <button class="btn btn-sm btn-primary" type="button" :disabled="!isDirty(contact.id) || !!saving[contact.id] || !!validationError(contact)" @click="saveDraft(contact)">
                <span v-if="saving[contact.id]" class="loading loading-spinner loading-xs"></span>
                <Save v-else class="h-3.5 w-3.5" />{{ t('common.save') }}
              </button>
            </div>
            <p v-if="validationError(contact)" class="text-right text-xs text-error">{{ validationError(contact) }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, ref, watch } from "vue";
import { ClipboardPaste, Copy, RotateCcw, Save, SlidersHorizontal } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../../services/tauri-api";
import type { RemoteImContact, RemoteImGroupReplyPacing } from "../../../../../types/app";
import { normalizeGroupReplyPacing, parseSpaceSeparatedList, resolveBehaviorDraftSave } from "./helpers";

const props = defineProps<{ contacts: RemoteImContact[] }>();
const emit = defineEmits<{ updated: [contact: RemoteImContact]; status: [message: string] }>();
const { t } = useI18n();

type Draft = {
  muteKeywordsText: string;
  unmuteKeywordsText: string;
  blockedMessagePrefixesText: string;
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
  setup(fieldProps, { emit: fieldEmit }) {
    return () => h("label", { class: "form-control" }, [
      h("span", { class: "label-text text-xs" }, fieldProps.label || ""),
      h("input", {
        class: "input input-sm input-bordered",
        type: "number",
        value: fieldProps.modelValue,
        min: fieldProps.min,
        max: fieldProps.max,
        step: fieldProps.step ?? 1,
        onInput: (event: Event) => fieldEmit("update:modelValue", Number((event.target as HTMLInputElement).value)),
      }),
    ]);
  },
});

const search = ref("");
const showPrivate = ref(false);
const drafts = ref<Record<string, Draft>>({});
const snapshots = ref<Record<string, string>>({});
const saving = ref<Record<string, boolean>>({});
const errors = ref<Record<string, string>>({});
const clipboard = ref<Draft | null>(null);

function isGroup(contact: RemoteImContact) {
  return String(contact.remoteContactType || "").toLowerCase() === "group";
}
function displayName(contact: RemoteImContact) {
  return String(contact.remarkName || contact.remoteContactName || contact.remoteContactId || contact.id);
}
function draftFromContact(contact: RemoteImContact): Draft {
  const pacing = normalizeGroupReplyPacing(contact.groupReplyPacing);
  return {
    muteKeywordsText: (contact.muteKeywords || []).join(" "),
    unmuteKeywordsText: (contact.unmuteKeywords || []).join(" "),
    blockedMessagePrefixesText: (contact.blockedMessagePrefixes || []).join(" "),
    patienceSeconds: Number(contact.patienceSeconds || 0),
    muteDurationSeconds: Number(contact.muteDurationSeconds || 0),
    activationCooldownSeconds: Number(contact.activationCooldownSeconds || 0),
    positiveEnergyPhrasesText: pacing.positiveEnergyPhrases.join(" "),
    negativeEnergyPhrasesText: pacing.negativeEnergyPhrases.join(" "),
    focusInstructionsText: pacing.focusInstructions.join(" "),
    pacing,
  };
}
function snapshot(draft: Draft) { return JSON.stringify(draft); }
function resetDraft(contact: RemoteImContact) {
  const draft = draftFromContact(contact);
  drafts.value[contact.id] = draft;
  snapshots.value[contact.id] = snapshot(draft);
  errors.value[contact.id] = "";
}
function isDirty(contactId: string) {
  const draft = drafts.value[contactId];
  return !!draft && snapshot(draft) !== snapshots.value[contactId];
}
function validationError(contact: RemoteImContact) {
  const draft = drafts.value[contact.id];
  if (!draft) return "";
  const p = draft.pacing;
  const commonNumbers = [draft.patienceSeconds, draft.muteDurationSeconds, draft.activationCooldownSeconds];
  if (commonNumbers.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  if (!isGroup(contact)) return "";
  const groupNumbers = [p.assistantDebounceSeconds, p.secretaryInspectionSeconds, p.replyCooldownSeconds, p.inspectionJitterRatio, p.maximumEnergy, p.baseReplyEnergyCost, p.energyCostPerCharacter, p.energyRecoveryPerSecond, p.positiveEnergyDelta, p.negativeEnergyDelta, p.normalReplyMaxChars, p.focusReplyMaxChars];
  if (groupNumbers.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  if (p.assistantDebounceSeconds < 1 || p.secretaryInspectionSeconds < 1) return t("config.remoteIm.behaviorPeriodError");
  if (p.inspectionJitterRatio < 0 || p.inspectionJitterRatio > 1) return t("config.remoteIm.behaviorJitterError");
  if (p.maximumEnergy <= 0 || p.baseReplyEnergyCost < 0 || p.energyCostPerCharacter < 0 || p.energyRecoveryPerSecond < 0 || p.positiveEnergyDelta < 0 || p.negativeEnergyDelta > 0) return t("config.remoteIm.behaviorEnergyError");
  if (p.normalReplyMaxChars < 1 || p.focusReplyMaxChars < p.normalReplyMaxChars) return t("config.remoteIm.behaviorLengthError");
  return "";
}
function copyBehavior(contactId: string) {
  const draft = drafts.value[contactId];
  if (draft) clipboard.value = structuredClone(draft);
}
function pasteBehavior(contact: RemoteImContact) {
  if (!clipboard.value) return;
  const next = structuredClone(clipboard.value);
  if (!isGroup(contact)) {
    const current = drafts.value[contact.id] || draftFromContact(contact);
    next.positiveEnergyPhrasesText = current.positiveEnergyPhrasesText;
    next.negativeEnergyPhrasesText = current.negativeEnergyPhrasesText;
    next.focusInstructionsText = current.focusInstructionsText;
    next.pacing = structuredClone(current.pacing);
  }
  drafts.value[contact.id] = next;
}
async function saveDraft(contact: RemoteImContact) {
  const draft = drafts.value[contact.id];
  const error = validationError(contact);
  if (!draft || error) return;
  saving.value[contact.id] = true;
  errors.value[contact.id] = "";
  const submittedSnapshot = snapshot(draft);
  const pacing = {
    ...draft.pacing,
    positiveEnergyPhrases: parseSpaceSeparatedList(draft.positiveEnergyPhrasesText),
    negativeEnergyPhrases: parseSpaceSeparatedList(draft.negativeEnergyPhrasesText),
    focusInstructions: parseSpaceSeparatedList(draft.focusInstructionsText),
  };
  try {
    const updated = await invokeTauri<RemoteImContact>("remote_im_update_contact_behavior", {
      input: {
        contactId: contact.id,
        muteKeywords: parseSpaceSeparatedList(draft.muteKeywordsText),
        unmuteKeywords: parseSpaceSeparatedList(draft.unmuteKeywordsText),
        patienceSeconds: draft.patienceSeconds,
        muteDurationSeconds: draft.muteDurationSeconds,
        activationCooldownSeconds: draft.activationCooldownSeconds,
        blockedMessagePrefixes: parseSpaceSeparatedList(draft.blockedMessagePrefixesText),
        groupReplyPacing: pacing,
      },
    });
    emit("updated", updated);
    const serverDraft = draftFromContact(updated);
    const resolved = resolveBehaviorDraftSave(
      drafts.value[contact.id],
      snapshots.value[contact.id],
      submittedSnapshot,
      serverDraft,
    );
    drafts.value[contact.id] = resolved.draft;
    snapshots.value[contact.id] = resolved.savedSnapshot;
    errors.value[contact.id] = resolved.error;
    emit("status", t("config.remoteIm.behaviorSaved"));
  } catch (saveError) {
    const resolved = resolveBehaviorDraftSave(
      drafts.value[contact.id],
      snapshots.value[contact.id],
      submittedSnapshot,
      null,
      saveError,
    );
    drafts.value[contact.id] = resolved.draft;
    snapshots.value[contact.id] = resolved.savedSnapshot;
    errors.value[contact.id] = resolved.error;
  } finally {
    saving.value[contact.id] = false;
  }
}

const filteredContacts = computed(() => {
  const needle = search.value.trim().toLowerCase();
  return props.contacts.filter((contact) => {
    if (!showPrivate.value && !isGroup(contact)) return false;
    return !needle || displayName(contact).toLowerCase().includes(needle) || contact.remoteContactId.toLowerCase().includes(needle);
  });
});

watch(() => props.contacts, (contacts) => {
  for (const contact of contacts) {
    if (!drafts.value[contact.id] || !isDirty(contact.id)) resetDraft(contact);
  }
}, { immediate: true, deep: true });
</script>
