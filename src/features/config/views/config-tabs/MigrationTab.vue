<template>
  <div class="space-y-5">
    <div class="text-xl font-semibold">{{ t("config.migration.pageTitle") }}</div>

    <div class="grid grid-cols-1 gap-5 xl:grid-cols-2">
      <div class="card bg-base-100 border border-base-300 shadow-sm">
        <div class="card-body flex flex-col gap-5">
          <div class="flex items-start gap-4">
            <div class="flex items-center justify-center rounded-box bg-info/15 p-3 text-info">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 16V4m0 0-4 4m4-4 4 4M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3" />
              </svg>
            </div>
            <div>
              <div class="card-title text-base">{{ t("config.migration.exportTitle") }}</div>
              <div class="mt-1 text-sm opacity-70">{{ t("config.migration.exportHint") }}</div>
            </div>
          </div>

          <div class="rounded-box border border-info/25 bg-info/10 px-4 py-3 text-sm text-base-content">
            <div class="flex items-center gap-3">
              <div class="flex items-center justify-center rounded-full bg-info/20 px-2 py-1 text-sm font-bold text-info">i</div>
              <span>{{ t("config.migration.exportNotice") }}</span>
            </div>
          </div>

          <label class="form-control w-full gap-2">
            <span class="text-sm font-medium">{{ t("config.migration.password") }}</span>
            <label class="input input-bordered flex w-full items-center gap-3">
              <input
                v-model.trim="exportPassword"
                :type="showExportPassword ? 'text' : 'password'"
                class="min-w-0 grow"
                :placeholder="t('config.migration.passwordPlaceholder')"
              />
              <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="showExportPassword = !showExportPassword">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 opacity-60" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.27 2.943 9.542 7-1.272 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7Z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
              </button>
              </label>
            </label>

          <button
            class="btn btn-primary w-full"
            :disabled="busy || !canExport"
            @click="handleExport"
          >
            {{ t("config.migration.exportAction") }}
          </button>

          <div v-if="exportMessage" class="alert" :class="exportMessageIsError ? 'alert-error' : 'alert-success'">
            <span>{{ exportMessage }}</span>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 border border-base-300 shadow-sm">
        <div class="card-body flex flex-col gap-5">
          <div class="flex items-start gap-4">
            <div class="flex items-center justify-center rounded-box bg-success/15 p-3 text-success">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 8v12m0 0 4-4m-4 4-4-4M4 9V6a2 2 0 0 1 2-2h4m10 5V6a2 2 0 0 0-2-2h-4" />
              </svg>
            </div>
            <div>
              <div class="card-title text-base">{{ t("config.migration.importTitle") }}</div>
              <div class="mt-1 text-sm opacity-70">{{ t("config.migration.importHint") }}</div>
            </div>
          </div>

          <div class="rounded-box border border-info/25 bg-info/10 px-4 py-3 text-sm text-base-content">
            <div class="flex items-center gap-3">
              <div class="flex items-center justify-center rounded-full bg-info/20 px-2 py-1 text-sm font-bold text-info">i</div>
              <span>{{ t("config.migration.importNotice") }}</span>
            </div>
          </div>

          <button
            type="button"
            class="flex w-full flex-col items-center justify-center rounded-box border-2 border-dashed border-base-300 bg-base-100 px-6 py-8 text-center transition hover:border-primary/50 hover:bg-primary/5"
            :disabled="busy"
            @click="handleSelectImportPackage"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="mb-4 h-12 w-12 text-base-content/35" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7">
              <path stroke-linecap="round" stroke-linejoin="round" d="M14 3v4a1 1 0 0 0 1 1h4M12 13v6m-3-3h6M6 3h7l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Z" />
            </svg>
            <div class="text-base font-semibold">{{ t("config.migration.importUploadTitle") }}</div>
            <div class="mt-2 whitespace-pre-line text-sm opacity-70">{{ t("config.migration.importUploadHint") }}</div>
          </button>

          <label v-if="needImportPassword" class="form-control w-full gap-2">
            <span class="text-sm font-medium">{{ t("config.migration.decryptPassword") }}</span>
            <label class="input input-bordered flex w-full items-center gap-3">
              <input
                v-model.trim="importPassword"
                :type="showImportPassword ? 'text' : 'password'"
                class="min-w-0 grow"
                :placeholder="t('config.migration.decryptPasswordPlaceholder')"
              />
              <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="showImportPassword = !showImportPassword">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 opacity-60" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.27 2.943 9.542 7-1.272 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7Z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
              </button>
            </label>
          </label>

          <button
            v-if="needImportPassword"
            class="btn btn-primary w-full"
            :disabled="busy || importPassword.length === 0"
            @click="handlePreviewImport"
          >
            {{ t("config.migration.previewWithPasswordAction") }}
          </button>

          <div v-if="previewResult" class="rounded-box border border-base-300 bg-base-100 p-5">
            <div class="mb-4 text-base font-semibold">{{ t("config.migration.previewTitle") }}</div>
            <div class="grid grid-cols-1 gap-3 text-sm md:grid-cols-2">
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.packageVersion", { version: previewResult.packageVersion }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.memoryAdded", { count: previewResult.memoryAddedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.memoryMerged", { count: previewResult.memoryMergedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.providerAdded", { count: previewResult.providerAddedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.providerUpdated", { count: previewResult.providerUpdatedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.apiConfigAdded", { count: previewResult.apiConfigAddedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.apiConfigUpdated", { count: previewResult.apiConfigUpdatedCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.oauthFileCount", { count: previewResult.oauthFileCount }) }}</div>
              <div class="rounded-box bg-base-200/60 p-4">{{ t("config.migration.avatarFileCount", { count: previewResult.avatarFileCount }) }}</div>
            </div>

            <div class="alert alert-warning mt-4">
              <span>{{ t("config.migration.importWarning") }}</span>
            </div>

            <div class="mt-4 flex justify-end">
              <button class="btn btn-primary" :disabled="busy" @click="handleApplyImport">
                {{ t("config.migration.applyAction") }}
              </button>
            </div>
          </div>

          <div v-if="importMessage" class="alert" :class="importMessageIsError ? 'alert-error' : 'alert-success'">
            <span>{{ importMessage }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import { toErrorMessage } from "../../../../utils/error";

type MigrationPreviewResult = {
  previewId: string;
  packageVersion: string;
  memoryAddedCount: number;
  memoryMergedCount: number;
  providerAddedCount: number;
  providerUpdatedCount: number;
  apiConfigAddedCount: number;
  apiConfigUpdatedCount: number;
  oauthFileCount: number;
  avatarFileCount: number;
};

const { t } = useI18n();

const busy = ref(false);
const exportPassword = ref("");
const importPassword = ref("");
const previewResult = ref<MigrationPreviewResult | null>(null);
const exportMessage = ref("");
const exportMessageIsError = ref(false);
const importMessage = ref("");
const importMessageIsError = ref(false);
const showExportPassword = ref(false);
const showImportPassword = ref(false);
const needImportPassword = ref(false);
const selectedImportPackagePath = ref("");
const PASSWORD_REQUIRED_CODE = "MIGRATION_PASSWORD_REQUIRED";

const canExport = computed(() => exportPassword.value.length >= 6);

function setExportMessage(text: string, isError = false) {
  exportMessage.value = text;
  exportMessageIsError.value = isError;
}

function setImportMessage(text: string, isError = false) {
  importMessage.value = text;
  importMessageIsError.value = isError;
}

async function handleExport() {
  busy.value = true;
  previewResult.value = null;
  exportMessage.value = "";
  try {
    const result = await invokeTauri<{ path: string }>("export_config_migration_package", {
      input: { password: exportPassword.value },
    });
    setExportMessage(t("config.migration.exportSuccess", { path: result.path }));
  } catch (error) {
    setExportMessage(toErrorMessage(error), true);
  } finally {
    busy.value = false;
  }
}

async function handleSelectImportPackage() {
  const picked = await open({
    multiple: false,
    filters: [{ name: "P-AI Migration", extensions: ["zip"] }],
  });
  if (!picked || Array.isArray(picked)) {
    setImportMessage(t("config.migration.importCancelled"));
    return;
  }
  selectedImportPackagePath.value = String(picked);
  importPassword.value = "";
  needImportPassword.value = false;
  previewResult.value = null;
  await handlePreviewImport();
}

async function handlePreviewImport() {
  if (!selectedImportPackagePath.value) {
    setImportMessage(t("config.migration.importCancelled"));
    return;
  }
  busy.value = true;
  importMessage.value = "";
  try {
    previewResult.value = await invokeTauri<MigrationPreviewResult>("preview_import_config_migration_package", {
      input: {
        password: importPassword.value,
        packagePath: selectedImportPackagePath.value,
      },
    });
    needImportPassword.value = false;
    setImportMessage(t("config.migration.previewSuccess"));
  } catch (error) {
    previewResult.value = null;
    const raw = error as { code?: string; type?: string } | undefined;
    const text = toErrorMessage(error);
    if (raw?.code === PASSWORD_REQUIRED_CODE || raw?.type === PASSWORD_REQUIRED_CODE) {
      needImportPassword.value = true;
    }
    setImportMessage(text, true);
  } finally {
    busy.value = false;
  }
}

async function handleApplyImport() {
  if (!previewResult.value) return;
  busy.value = true;
  try {
    await invokeTauri("apply_import_config_migration_package", {
      input: { previewId: previewResult.value.previewId },
    });
    setImportMessage(t("config.migration.applySuccess"));
  } catch (error) {
    setImportMessage(toErrorMessage(error), true);
  } finally {
    busy.value = false;
  }
}
</script>
