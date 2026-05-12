<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { RuntimeLogEntry } from "../../../types/app";
import RuntimeLogsDialog from "./RuntimeLogsDialog.vue";

type UpdateDialogKind = "error" | "info" | "warning";
type UpdateDialogPrimaryAction = "force" | "download" | "restart" | null | undefined;
type ConfigSaveErrorDialogKind = "warning" | "error";
type ArchiveImportPreview = {
  fileName: string;
  total: number;
  imported: number;
  replaced: number;
} | null;
type ForceArchivePreviewResult = {
  canArchive: boolean;
  canDropConversation: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  archiveDisabledReason?: string | null;
} | null;
type ForceCompactionPreviewResult = {
  canCompact: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
} | null;

const props = defineProps<{
  updateDialogOpen: boolean;
  updateDialogTitle: string;
  updateDialogBody: string;
  updateDialogKind: UpdateDialogKind;
  updateDialogReleaseUrl?: string;
  updateDialogPrimaryAction?: UpdateDialogPrimaryAction;
  updateProgressPercent?: number | null;
  runtimeLogsDialogOpen: boolean;
  runtimeLogs: RuntimeLogEntry[];
  runtimeLogsLoading: boolean;
  runtimeLogsError: string;
  rewindConfirmDialogOpen: boolean;
  rewindConfirmCanUndoPatch: boolean;
  configSaveErrorDialogOpen: boolean;
  configSaveErrorDialogTitle: string;
  configSaveErrorDialogBody: string;
  configSaveErrorDialogKind: ConfigSaveErrorDialogKind;
  archiveImportPreviewDialogOpen: boolean;
  archiveImportPreview: ArchiveImportPreview;
  archiveImportRunning: boolean;
  skillPlaceholderDialogOpen: boolean;
  forceArchiveActionDialogOpen: boolean;
  forceArchivePreviewLoading: boolean;
  forceArchivePreview: ForceArchivePreviewResult;
  forceCompactionPreview: ForceCompactionPreviewResult;
  forcingArchive: boolean;
}>();

const emit = defineEmits<{
  closeUpdateDialog: [];
  confirmUpdateDialogPrimary: [];
  openUpdateRelease: [];
  closeRuntimeLogsDialog: [];
  refreshRuntimeLogs: [];
  clearRuntimeLogs: [];
  confirmRewindWithPatch: [];
  confirmRewindMessageOnly: [];
  cancelRewindConfirm: [];
  closeConfigSaveErrorDialog: [];
  closeArchiveImportPreviewDialog: [];
  confirmArchiveImport: [];
  closeSkillPlaceholderDialog: [];
  confirmDeleteConversationFromArchiveDialog: [];
  confirmForceCompactionAction: [];
  confirmForceArchiveAction: [];
  closeForceArchiveActionDialog: [];
}>();

const { t } = useI18n();

function handleConfirmForceArchiveAction() {
  emit("confirmForceArchiveAction");
}

function handleCloseForceArchiveActionDialog() {
  emit("closeForceArchiveActionDialog");
}

function handleConfirmForceCompactionAction() {
  emit("confirmForceCompactionAction");
}

function handleConfirmDeleteConversationFromArchiveDialog() {
  emit("confirmDeleteConversationFromArchiveDialog");
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': updateDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">
        {{ updateDialogTitle }}
      </h3>
      <pre
        class="mt-2 whitespace-pre-wrap text-sm"
        :class="updateDialogKind === 'error' ? 'text-error' : 'text-base-content'"
      >{{ updateDialogBody }}</pre>
      <progress
        v-if="typeof updateProgressPercent === 'number'"
        class="progress progress-primary mt-4 w-full"
        :value="Math.max(0, Math.min(100, updateProgressPercent))"
        max="100"
      />
      <div class="modal-action">
        <button
          v-if="updateDialogPrimaryAction"
          class="btn btn-sm btn-primary"
          @click="emit('confirmUpdateDialogPrimary')"
        >
          {{
            updateDialogPrimaryAction === 'force'
              ? t("dialogs.update.forceDownload")
              : updateDialogPrimaryAction === 'restart'
                ? t("dialogs.update.restart")
                : t("dialogs.update.download")
          }}
        </button>
        <button
          v-if="updateDialogReleaseUrl"
          class="btn btn-sm"
          @click="emit('openUpdateRelease')"
        >
          {{ t("dialogs.update.openReleases") }}
        </button>
        <button class="btn btn-sm" @click="emit('closeUpdateDialog')">
          {{ updateDialogPrimaryAction ? t("common.cancel") : t("common.confirm") }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeUpdateDialog')">close</button>
    </form>
  </dialog>

  <RuntimeLogsDialog
    :open="runtimeLogsDialogOpen"
    :logs="runtimeLogs"
    :loading="runtimeLogsLoading"
    :error-text="runtimeLogsError"
    @close="emit('closeRuntimeLogsDialog')"
    @refresh="emit('refreshRuntimeLogs')"
    @clear="emit('clearRuntimeLogs')"
  />

  <dialog class="modal" :class="{ 'modal-open': rewindConfirmDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">{{ t("dialogs.rewind.title") }}</h3>
      <div class="mt-2 text-sm opacity-80">{{ t("dialogs.rewind.hint") }}</div>
      <div class="mt-4 flex flex-col items-center gap-2">
        <button
          v-if="rewindConfirmCanUndoPatch"
          class="btn btn-sm btn-error w-full"
          @click="emit('confirmRewindWithPatch')"
        >
          {{ t("dialogs.rewind.withPatch") }}
        </button>
        <button class="btn btn-sm w-full" @click="emit('confirmRewindMessageOnly')">
          {{ t("dialogs.rewind.messageOnly") }}
        </button>
        <button class="btn btn-sm btn-primary w-full" @click="emit('cancelRewindConfirm')">{{ t("common.cancel") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('cancelRewindConfirm')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': configSaveErrorDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">
        {{ configSaveErrorDialogTitle }}
      </h3>
      <pre
        class="mt-2 whitespace-pre-wrap text-sm"
        :class="configSaveErrorDialogKind === 'warning' ? 'text-warning' : 'text-error'"
      >{{ configSaveErrorDialogBody }}</pre>
      <div class="modal-action">
        <button class="btn btn-sm btn-primary" @click="emit('closeConfigSaveErrorDialog')">{{ t("common.close") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeConfigSaveErrorDialog')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': archiveImportPreviewDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">
        {{ t("archives.importPreviewTitle") }}
      </h3>
      <div v-if="archiveImportPreview" class="mt-3 space-y-1 text-sm">
        <div>{{ t("archives.importPreviewFile", { name: archiveImportPreview.fileName }) }}</div>
        <div>{{ t("archives.importPreviewTotal", { count: archiveImportPreview.total }) }}</div>
        <div>{{ t("archives.importPreviewAdd", { count: archiveImportPreview.imported }) }}</div>
        <div>{{ t("archives.importPreviewReplace", { count: archiveImportPreview.replaced }) }}</div>
        <div class="text-sm opacity-70 mt-2">{{ t("archives.importPreviewHint") }}</div>
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="archiveImportRunning" @click="emit('closeArchiveImportPreviewDialog')">
          {{ t("common.cancel") }}
        </button>
        <button class="btn btn-sm btn-primary" :disabled="archiveImportRunning" @click="emit('confirmArchiveImport')">
          {{ archiveImportRunning ? t("common.loading") : t("archives.importConfirm") }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeArchiveImportPreviewDialog')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': skillPlaceholderDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">{{ t("dialogs.skill.title") }}</h3>
      <div class="mt-2 text-sm opacity-80">{{ t("dialogs.skill.placeholder") }}</div>
      <div class="modal-action">
        <button class="btn btn-sm btn-primary" @click="emit('closeSkillPlaceholderDialog')">{{ t("common.close") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeSkillPlaceholderDialog')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': forceArchiveActionDialogOpen }">
    <div class="modal-box w-[80vw] max-w-[80vw]">
      <h3 class="font-semibold text-base">{{ t("dialogs.forceArchive.title") }}</h3>
      <div v-if="forceArchivePreviewLoading" class="mt-3 text-sm opacity-70">{{ t("dialogs.forceArchive.loading") }}</div>
      <template v-else>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">{{ t("dialogs.forceArchive.compactTitle") }}</div>
          <div class="mt-1 opacity-80">{{ t("dialogs.forceArchive.compactSummary") }}</div>
          <div class="mt-2 text-xs opacity-70">{{ t("dialogs.forceArchive.compactHint") }}</div>
          <div
            v-if="forceCompactionPreview?.compactionDisabledReason"
            class="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning-content"
          >
            {{ forceCompactionPreview.compactionDisabledReason }}
          </div>
        </div>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">{{ t("dialogs.forceArchive.dropTitle") }}</div>
          <div class="mt-1 opacity-80">{{ t("dialogs.forceArchive.dropSummary") }}</div>
          <div class="mt-2 text-xs opacity-70">{{ t("dialogs.forceArchive.dropHint") }}</div>
        </div>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">{{ t("dialogs.forceArchive.archiveTitle") }}</div>
          <div class="mt-1 opacity-80">{{ t("dialogs.forceArchive.archiveSummary") }}</div>
          <div class="mt-2 text-xs opacity-70">{{ t("dialogs.forceArchive.archiveHint") }}</div>
          <div
            v-if="forceArchivePreview?.archiveDisabledReason"
            class="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning-content"
          >
            {{ forceArchivePreview.archiveDisabledReason }}
          </div>
        </div>
      </template>
      <div class="mt-4 flex items-end justify-between gap-4">
        <div class="text-xs opacity-60">
          <div>{{ t("dialogs.forceArchive.messageCount", { count: forceArchivePreview?.messageCount ?? 0 }) }}</div>
          <div>{{ t("dialogs.forceArchive.contextUsage", { percent: forceCompactionPreview?.contextUsagePercent ?? 0 }) }}</div>
        </div>
        <div class="modal-action mt-0">
        <button
          class="btn btn-sm btn-error"
          :disabled="forceArchivePreviewLoading || !forceArchivePreview?.canDropConversation || forcingArchive"
          @click="handleConfirmDeleteConversationFromArchiveDialog"
        >
          {{ t("dialogs.forceArchive.dropTitle") }}
        </button>
        <button
          class="btn btn-sm btn-primary"
          :disabled="forceArchivePreviewLoading || !forceCompactionPreview?.canCompact || forcingArchive"
          @click="handleConfirmForceCompactionAction"
        >
          {{ t("dialogs.forceArchive.compactTitle") }}
        </button>
        <button
          class="btn btn-sm btn-secondary"
          :disabled="forceArchivePreviewLoading || !forceArchivePreview?.canArchive || forcingArchive"
          @click="handleConfirmForceArchiveAction"
        >
          {{ t("dialogs.forceArchive.archiveTitle") }}
        </button>
        <button class="btn btn-sm" :disabled="forceArchivePreviewLoading || forcingArchive" @click="handleCloseForceArchiveActionDialog">
          {{ t("common.cancel") }}
        </button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="handleCloseForceArchiveActionDialog">close</button>
    </form>
  </dialog>
</template>
