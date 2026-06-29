<script setup lang="ts">
import { useI18n } from "vue-i18n";

const props = defineProps<{
  visible: boolean;
  latestVersion?: string;
  runtimeKind?: "installer" | "portable";
  accessModeLabel?: string;
  releaseUrl?: string;
  updateReadyToRestart: boolean;
  updateInProgress: boolean;
  updateCancelPending: boolean;
  updateCanCancel: boolean;
  progressPercent?: number | null;
  progressText?: string;
}>();

const emit = defineEmits<{
  updateNow: [];
  skipVersion: [];
  cancelUpdate: [];
  openRelease: [];
}>();

const { t } = useI18n();
</script>

<template>
  <div v-if="props.visible" class="border-b border-warning/20 bg-gradient-to-r from-warning/12 via-base-100 to-base-100 px-4 py-3">
    <div class="mx-auto flex max-w-6xl flex-col gap-3 rounded-box border border-warning/20 bg-base-100/90 px-4 py-3 shadow-sm lg:flex-row lg:items-center lg:justify-between">
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-warning badge-outline">{{ t("about.updateAvailableBadge") }}</span>
          <span class="font-medium">{{ t("about.updateReminderTitle", { version: props.latestVersion || "-" }) }}</span>
          <span class="badge badge-ghost badge-sm">{{ props.accessModeLabel || t("about.updateMethodAuto") }}</span>
          <span v-if="props.runtimeKind === 'portable'" class="badge badge-ghost badge-sm">{{ t("about.runtimePortable") }}</span>
          <span v-else-if="props.runtimeKind === 'installer'" class="badge badge-ghost badge-sm">{{ t("about.runtimeInstaller") }}</span>
        </div>
        <div class="mt-1 text-sm opacity-75">
          {{
            props.updateReadyToRestart
              ? t("about.updateReadyAction", { version: props.latestVersion || "-" })
              : props.updateInProgress
                ? (props.progressText || t("about.updating"))
                : t("about.updateReminderBody")
          }}
        </div>
        <progress
          v-if="props.updateInProgress && typeof props.progressPercent === 'number'"
          class="progress progress-primary mt-3 w-full max-w-xl"
          :value="Math.max(0, Math.min(100, props.progressPercent || 0))"
          max="100"
        />
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <button
          v-if="!props.updateReadyToRestart && !props.updateInProgress"
          class="btn btn-sm btn-ghost"
          @click="emit('skipVersion')"
        >
          {{ t("about.skipVersion") }}
        </button>
        <button
          v-if="props.updateInProgress && props.updateCanCancel"
          class="btn btn-sm btn-ghost"
          :disabled="props.updateCancelPending"
          @click="emit('cancelUpdate')"
        >
          {{ props.updateCancelPending ? t("about.cancellingUpdate") : t("about.cancelUpdate") }}
        </button>
        <button
          v-if="props.releaseUrl"
          class="btn btn-sm btn-ghost"
          @click="emit('openRelease')"
        >
          {{ t("dialogs.update.openReleases") }}
        </button>
        <button
          class="btn btn-sm btn-primary"
          @click="emit('updateNow')"
        >
          {{ props.updateReadyToRestart ? t("about.updateAndRestart") : t("about.updateNow") }}
        </button>
      </div>
    </div>
  </div>
</template>
