<template>
  <dialog ref="dialogRef" class="modal !items-stretch overflow-hidden p-0 sm:!items-center sm:p-4" @cancel.prevent="close">
    <div class="modal-box flex h-[100dvh] max-h-none w-screen max-w-none flex-col overflow-hidden rounded-none p-0 sm:h-[calc(100dvh-max(2rem,env(safe-area-inset-top)+env(safe-area-inset-bottom)))] sm:w-[calc(100vw-max(2rem,env(safe-area-inset-left)+env(safe-area-inset-right)))] sm:max-w-5xl sm:rounded-box">
      <div class="flex shrink-0 items-center justify-between gap-3 border-b border-base-300 pb-3 pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))] pt-[max(0.75rem,env(safe-area-inset-top))]">
        <div class="min-w-0">
          <div class="flex items-center gap-2 text-sm font-semibold">
            <FolderOpen class="h-4 w-4 opacity-70" />
            <span>{{ t('config.tools.androidWorkspaceFileManagerTitle') }}</span>
          </div>
          <div class="mt-1 font-mono text-xs opacity-60 break-all">
            {{ t('config.tools.androidWorkspaceFileManagerCurrentPath', { path: fileManagerPathLabel }) }}
          </div>
        </div>
        <button class="btn btn-square btn-ghost btn-sm" type="button" @click="close">
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="grid shrink-0 grid-cols-2 gap-2 border-b border-base-300 bg-base-200/50 py-2 pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))] sm:flex sm:flex-wrap sm:items-center">
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="busy || parentPath === null" @click="navigateParent">
          <ChevronLeft class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceParentDirectory') }}</span>
        </button>
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="busy || importing" @click="chooseImportFile">
          <FileUp class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceImportHere') }}</span>
        </button>
        <button class="btn btn-sm btn-neutral w-full justify-start sm:w-auto" type="button" :disabled="!selectedFile || exporting" @click="exportSelectedFile">
          <Download class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceExport') }}</span>
        </button>
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="!selectedFile || reading" @click="readSelectedFile">
          <Eye class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceReadText') }}</span>
        </button>
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="!selectedFile || saving" @click="editSelectedFile">
          <FilePen class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceEditText') }}</span>
        </button>
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="!selectedFile || moving" @click="requestMoveSelectedFile">
          <Move class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceMove') }}</span>
        </button>
        <button class="btn btn-sm w-full justify-start sm:w-auto" type="button" :disabled="searching" @click="openSearchDialog">
          <Search class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceSearchFiles') }}</span>
        </button>
        <button class="btn btn-sm btn-error btn-outline w-full justify-start sm:w-auto" type="button" :disabled="!selectedFile || deleting" @click="requestDeleteSelectedFile">
          <Trash2 class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceDelete') }}</span>
        </button>
        <button class="btn btn-sm btn-ghost w-full justify-start sm:ml-auto sm:w-auto" type="button" :disabled="loading" @click="loadFiles()">
          <RefreshCw class="h-4 w-4" />
          <span class="min-w-0 truncate">{{ t('config.tools.androidWorkspaceRefreshFiles') }}</span>
        </button>
      </div>

      <div class="shrink-0 border-b border-base-300 py-2 pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))]">
        <div class="breadcrumbs max-w-full overflow-x-auto text-xs">
          <ul>
            <li>
              <button class="link-hover" type="button" :disabled="busy" @click="navigateDirectory('')">
                {{ t('config.tools.androidWorkspaceFileManagerRoot') }}
              </button>
            </li>
            <li v-for="segment in pathSegments" :key="segment.path">
              <button class="link-hover" type="button" :disabled="busy" @click="navigateDirectory(segment.path)">
                {{ segment.name }}
              </button>
            </li>
          </ul>
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto bg-base-100 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
        <div v-if="loading" class="flex h-full items-center justify-center gap-2 px-4 text-sm opacity-60">
          <span class="loading loading-spinner loading-sm"></span>
          <span>{{ t('config.tools.androidWorkspaceFileManagerLoading') }}</span>
        </div>
        <div v-else-if="!entries.length" class="flex h-full items-center justify-center px-4 text-center text-sm opacity-60">
          {{ t('config.tools.androidWorkspaceFileManagerEmpty') }}
        </div>
        <div v-else class="divide-y divide-base-300/60">
          <button
            v-for="entry in entries"
            :key="entry.path"
            class="flex w-full items-center justify-between gap-3 py-3 text-left pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))] hover:bg-base-200"
            :class="entry.path === selectedFilePath ? 'bg-primary/10' : ''"
            type="button"
            @click="openEntry(entry)"
          >
            <span class="flex min-w-0 items-center gap-3">
              <Folder v-if="entry.kind === 'directory'" class="h-5 w-5 shrink-0 text-info" />
              <FileText v-else class="h-5 w-5 shrink-0 opacity-70" />
              <span class="min-w-0">
                <span class="block truncate text-sm">{{ entry.name }}</span>
                <span class="block text-xs opacity-60">
                  {{ entry.kind === 'directory' ? t('config.tools.androidWorkspaceFileKindDirectory') : formatBytes(entry.bytes || 0) }}
                </span>
              </span>
            </span>
            <span v-if="entry.path === selectedFilePath" class="badge badge-sm badge-primary">
              {{ t('config.tools.androidWorkspaceFileSelected') }}
            </span>
          </button>
        </div>
      </div>

      <div v-if="message" class="shrink-0 border-t border-base-300 pb-[max(0.5rem,env(safe-area-inset-bottom))] pt-2 text-xs pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))]" :class="messageError ? 'text-error' : 'opacity-70'">
        {{ message }}
      </div>

      <input ref="importInput" class="hidden" type="file" @change="onImportFileChange" />
    </div>
    <form class="modal-backdrop">
      <button type="button" aria-label="close" @click="close">close</button>
    </form>
  </dialog>

  <dialog ref="deleteDialogRef" class="modal" @cancel.prevent="cancelDelete">
    <div class="modal-box max-w-md p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
      <h3 class="text-sm font-semibold">{{ t('config.tools.androidWorkspaceDeleteTitle') }}</h3>
      <p class="mt-3 text-sm whitespace-pre-wrap">
        {{ t('config.tools.androidWorkspaceDeleteConfirm', { path: deleteTarget?.path || '-' }) }}
      </p>
      <div class="modal-action mt-4">
        <button class="btn btn-sm btn-ghost" type="button" :disabled="deleting" @click="cancelDelete">
          {{ t('common.cancel') }}
        </button>
        <button class="btn btn-sm btn-error" type="button" :disabled="deleting" @click="confirmDelete">
          {{ t('config.tools.androidWorkspaceDelete') }}
        </button>
      </div>
    </div>
    <form class="modal-backdrop">
      <button type="button" aria-label="close" @click="cancelDelete">close</button>
    </form>
  </dialog>

  <dialog ref="textDialogRef" class="modal" @cancel.prevent="textDialogRef?.close()">
    <div class="modal-box flex max-h-[85dvh] w-full max-w-3xl flex-col p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
      <div class="flex items-center justify-between gap-3">
        <h3 class="min-w-0 truncate text-sm font-semibold font-mono">{{ textDialogPath }}</h3>
        <button class="btn btn-square btn-ghost btn-sm shrink-0" type="button" @click="textDialogRef?.close()">
          <X class="h-4 w-4" />
        </button>
      </div>
      <textarea
        v-model="textDialogContent"
        class="textarea textarea-bordered mt-3 min-h-0 flex-1 w-full font-mono text-xs"
        :readonly="textDialogReadOnly"
        :placeholder="t('config.tools.androidWorkspaceTextPlaceholder')"
      ></textarea>
      <div class="modal-action mt-3">
        <button class="btn btn-sm btn-ghost" type="button" @click="textDialogRef?.close()">
          {{ t('common.cancel') }}
        </button>
        <button v-if="!textDialogReadOnly" class="btn btn-sm btn-primary" type="button" :disabled="saving" @click="saveTextFile">
          {{ t('config.tools.androidWorkspaceTextSave') }}
        </button>
      </div>
    </div>
  </dialog>

  <dialog ref="moveDialogRef" class="modal" @cancel.prevent="closeMoveDialog">
    <div class="modal-box max-w-md p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
      <h3 class="text-sm font-semibold">{{ t('config.tools.androidWorkspaceMoveTitle') }}</h3>
      <label class="mt-3 block text-xs opacity-70">{{ t('config.tools.androidWorkspaceMoveSource') }}</label>
      <div class="mt-1 font-mono text-xs break-all">{{ moveSourcePath || '-' }}</div>
      <label class="mt-3 block text-xs opacity-70">{{ t('config.tools.androidWorkspaceMoveTarget') }}</label>
      <input
        v-model="moveTargetPath"
        class="input input-bordered mt-1 w-full font-mono text-sm"
        type="text"
        :placeholder="t('config.tools.androidWorkspaceMoveTargetPlaceholder')"
      />
      <label class="mt-3 flex items-center gap-2 text-sm">
        <input v-model="moveOverwrite" class="checkbox checkbox-sm" type="checkbox" />
        <span>{{ t('config.tools.androidWorkspaceMoveOverwrite') }}</span>
      </label>
      <div class="modal-action mt-4">
        <button class="btn btn-sm btn-ghost" type="button" :disabled="moving" @click="closeMoveDialog">
          {{ t('common.cancel') }}
        </button>
        <button class="btn btn-sm btn-primary" type="button" :disabled="moving" @click="confirmMoveFile">
          {{ t('config.tools.androidWorkspaceMove') }}
        </button>
      </div>
    </div>
  </dialog>

  <dialog ref="searchDialogRef" class="modal" @cancel.prevent="searchDialogRef?.close()">
    <div class="modal-box flex max-h-[85dvh] w-full max-w-3xl flex-col p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
      <div class="flex items-center justify-between gap-3">
        <h3 class="text-sm font-semibold">{{ t('config.tools.androidWorkspaceSearchTitle') }}</h3>
        <button class="btn btn-square btn-ghost btn-sm" type="button" @click="searchDialogRef?.close()">
          <X class="h-4 w-4" />
        </button>
      </div>
      <div class="tabs tabs-boxed mt-3 w-fit">
        <button class="tab tab-sm" :class="searchMode === 'glob' ? 'tab-active' : ''" type="button" @click="searchMode = 'glob'">
          {{ t('config.tools.androidWorkspaceSearchGlob') }}
        </button>
        <button class="tab tab-sm" :class="searchMode === 'grep' ? 'tab-active' : ''" type="button" @click="searchMode = 'grep'">
          {{ t('config.tools.androidWorkspaceSearchGrep') }}
        </button>
      </div>
      <div class="mt-3 grid gap-2">
        <label v-if="searchMode === 'glob'" class="block text-xs opacity-70">
          {{ t('config.tools.androidWorkspaceSearchPattern') }}
          <input
            v-model="searchPattern"
            class="input input-bordered mt-1 w-full font-mono text-sm"
            type="text"
            :placeholder="t('config.tools.androidWorkspaceSearchPatternPlaceholder')"
          />
        </label>
        <label v-else class="block text-xs opacity-70">
          {{ t('config.tools.androidWorkspaceSearchQuery') }}
          <input
            v-model="searchQuery"
            class="input input-bordered mt-1 w-full font-mono text-sm"
            type="text"
            :placeholder="t('config.tools.androidWorkspaceSearchQueryPlaceholder')"
          />
        </label>
        <label class="block text-xs opacity-70">
          {{ t('config.tools.androidWorkspaceSearchPath') }}
          <input
            v-model="searchPath"
            class="input input-bordered mt-1 w-full font-mono text-sm"
            type="text"
            :placeholder="t('config.tools.androidWorkspaceSearchPathPlaceholder')"
          />
        </label>
        <label v-if="searchMode === 'grep'" class="block text-xs opacity-70">
          {{ t('config.tools.androidWorkspaceSearchIncludeGlob') }}
          <input
            v-model="searchIncludeGlob"
            class="input input-bordered mt-1 w-full font-mono text-sm"
            type="text"
            :placeholder="t('config.tools.androidWorkspaceSearchIncludeGlobPlaceholder')"
          />
        </label>
        <div v-if="searchMode === 'grep'" class="flex flex-wrap gap-4 text-sm">
          <label class="flex items-center gap-2">
            <input v-model="searchRegex" class="checkbox checkbox-sm" type="checkbox" />
            <span>{{ t('config.tools.androidWorkspaceSearchRegex') }}</span>
          </label>
          <label class="flex items-center gap-2">
            <input v-model="searchIgnoreCase" class="checkbox checkbox-sm" type="checkbox" />
            <span>{{ t('config.tools.androidWorkspaceSearchIgnoreCase') }}</span>
          </label>
        </div>
      </div>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button class="btn btn-sm btn-primary" type="button" :disabled="searching" @click="runWorkspaceSearch">
          <span v-if="searching" class="loading loading-spinner loading-sm"></span>
          {{ t('config.tools.androidWorkspaceSearchRun') }}
        </button>
      </div>
      <div class="mt-3 min-h-0 flex-1 overflow-y-auto border-t border-base-300 pt-2">
        <div v-if="searchMode === 'glob'">
          <div v-if="searchDone && !searchGlobResults.length" class="py-4 text-center text-xs opacity-60">
            {{ t('config.tools.androidWorkspaceSearchEmpty') }}
          </div>
          <button
            v-for="entry in searchGlobResults"
            :key="`glob-${entry.path}`"
            class="flex w-full items-center gap-2 py-2 text-left font-mono text-xs hover:bg-base-200"
            type="button"
            @click="openSearchResult(entry.path, entry.kind === 'directory')"
          >
            <Folder v-if="entry.kind === 'directory'" class="h-4 w-4 shrink-0 text-info" />
            <FileText v-else class="h-4 w-4 shrink-0 opacity-70" />
            <span class="min-w-0 truncate">{{ entry.path }}</span>
          </button>
        </div>
        <div v-else>
          <div v-if="searchDone && !searchGrepResults.length" class="py-4 text-center text-xs opacity-60">
            {{ t('config.tools.androidWorkspaceSearchEmpty') }}
          </div>
          <button
            v-for="(match, index) in searchGrepResults"
            :key="`grep-${match.path}-${match.line}-${index}`"
            class="block w-full py-2 text-left hover:bg-base-200"
            type="button"
            @click="openSearchResult(match.path, false)"
          >
            <span class="block truncate font-mono text-xs">{{ match.path }}:{{ match.line }}</span>
            <span class="block truncate text-xs opacity-70">{{ match.text }}</span>
          </button>
        </div>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { ChevronLeft, Download, Eye, FilePen, FileText, FileUp, Folder, FolderOpen, Move, RefreshCw, Search, Trash2, X } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { invokeTauri, isAndroidRuntime } from "../../../../services/tauri-api";
import { toErrorMessage } from "../../../../utils/error";

type AndroidWorkspaceState = "not_downloaded" | "downloading" | "ready";

type AndroidWorkspaceStatus = {
  state: AndroidWorkspaceState;
  rootPath: string;
  llmWorkspaceRoot?: string | null;
  runtimeRoot?: string | null;
  initializedAt?: string | null;
  updatedAt?: string | null;
  lastError?: string | null;
  version: number;
  runtimeVersion?: string | null;
  downloadBytes?: number | null;
  downloadTotalBytes?: number | null;
  downloadStage?: string | null;
};

type AndroidWorkspaceImportResult = AndroidWorkspaceStatus & {
  importedPath?: string;
  fileName?: string;
  bytes?: number;
};

type AndroidWorkspaceExportResult = {
  path: string;
  fileName: string;
  mime: string;
  dataBase64: string;
  bytes: number;
};

type AndroidWorkspaceFileEntry = {
  name: string;
  path: string;
  kind: "file" | "directory";
  bytes?: number | null;
};

type AndroidWorkspaceFileListResult = {
  currentPath: string;
  parentPath?: string | null;
  entries: AndroidWorkspaceFileEntry[];
};

type AndroidWorkspaceDeleteResult = {
  deletedPath: string;
};

type AndroidWorkspaceTextResult = {
  path: string;
  text: string;
  bytes: number;
};

type AndroidWorkspaceWriteResult = {
  entry: AndroidWorkspaceFileEntry;
};

type AndroidWorkspaceMoveResult = {
  sourcePath: string;
  entry: AndroidWorkspaceFileEntry;
};

type AndroidWorkspaceGlobResult = {
  entries: AndroidWorkspaceFileEntry[];
};

type AndroidWorkspaceSearchMatch = {
  path: string;
  line: number;
  text: string;
};

type AndroidWorkspaceGrepResult = {
  matches: AndroidWorkspaceSearchMatch[];
};

type AndroidWorkspaceSearchMode = "glob" | "grep";

const emit = defineEmits<{
  (e: "statusChanged", status: AndroidWorkspaceStatus): void;
}>();

const { t } = useI18n();
const dialogRef = ref<HTMLDialogElement | null>(null);
const deleteDialogRef = ref<HTMLDialogElement | null>(null);
const textDialogRef = ref<HTMLDialogElement | null>(null);
const moveDialogRef = ref<HTMLDialogElement | null>(null);
const searchDialogRef = ref<HTMLDialogElement | null>(null);
const importInput = ref<HTMLInputElement | null>(null);
const loading = ref(false);
const importing = ref(false);
const exporting = ref(false);
const deleting = ref(false);
const reading = ref(false);
const saving = ref(false);
const moving = ref(false);
const searching = ref(false);
const message = ref("");
const messageError = ref(false);
const currentPath = ref("");
const parentPath = ref<string | null>(null);
const entries = ref<AndroidWorkspaceFileEntry[]>([]);
const selectedFilePath = ref("");
const deleteTarget = ref<AndroidWorkspaceFileEntry | null>(null);
const textDialogPath = ref("");
const textDialogContent = ref("");
const textDialogReadOnly = ref(true);
const textDialogOverwrite = ref(false);
const moveSourcePath = ref("");
const moveTargetPath = ref("");
const moveOverwrite = ref(false);
const searchMode = ref<AndroidWorkspaceSearchMode>("glob");
const searchPattern = ref("");
const searchQuery = ref("");
const searchPath = ref("");
const searchRegex = ref(false);
const searchIgnoreCase = ref(true);
const searchIncludeGlob = ref("");
const searchGlobResults = ref<AndroidWorkspaceFileEntry[]>([]);
const searchGrepResults = ref<AndroidWorkspaceSearchMatch[]>([]);
const searchDone = ref(false);
let fileManagerHistoryActive = false;

const FILE_MANAGER_HISTORY_STATE_KEY = "__paiAndroidWorkspaceFileManager";

const busy = computed(() => loading.value || importing.value || exporting.value || deleting.value || reading.value || saving.value || moving.value || searching.value);
const fileManagerPathLabel = computed(() => currentPath.value || "/");
const pathSegments = computed(() => {
  const parts = currentPath.value.split("/").filter(Boolean);
  return parts.map((name, index) => ({
    name,
    path: parts.slice(0, index + 1).join("/"),
  }));
});
const selectedFile = computed(() => {
  const selectedPath = selectedFilePath.value;
  if (!selectedPath) return null;
  return entries.value.find((entry) => entry.kind === "file" && entry.path === selectedPath) || null;
});

function setMessage(text: string, isError = false) {
  message.value = text;
  messageError.value = isError;
}

function clearFileManager() {
  entries.value = [];
  selectedFilePath.value = "";
  parentPath.value = null;
}

function normalizeRelativePath(path: string): string {
  return String(path || "")
    .replace(/\\/g, "/")
    .split("/")
    .filter((part) => part && part !== "." && part !== "..")
    .join("/");
}

function joinWorkspacePath(dir: string, name: string): string {
  const normalizedDir = normalizeRelativePath(dir);
  const safeName = String(name || "imported-file").replace(/[\\/]+/g, "_") || "imported-file";
  return normalizedDir ? `${normalizedDir}/${safeName}` : safeName;
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  const digits = unitIndex === 0 ? 0 : 1;
  return `${size.toFixed(digits)} ${units[unitIndex]}`;
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function base64ToBlob(base64: string, mime: string): Blob {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Blob([bytes], { type: mime || "application/octet-stream" });
}

function pushFileManagerHistoryState() {
  if (fileManagerHistoryActive) return;
  window.history.pushState({ [FILE_MANAGER_HISTORY_STATE_KEY]: true }, "", window.location.href);
  fileManagerHistoryActive = true;
}

function clearFileManagerHistoryState() {
  if (!fileManagerHistoryActive) return;
  fileManagerHistoryActive = false;
  window.history.back();
}

function isFileManagerOpen() {
  return Boolean(dialogRef.value?.open);
}

function closeDialogs() {
  if (deleteDialogRef.value?.open) {
    deleteDialogRef.value.close();
  }
  deleteTarget.value = null;
  if (moveDialogRef.value?.open) {
    moveDialogRef.value.close();
  }
  if (searchDialogRef.value?.open) {
    searchDialogRef.value.close();
  }
  if (textDialogRef.value?.open) {
    textDialogRef.value.close();
  }
  if (dialogRef.value?.open) {
    dialogRef.value.close();
  }
}

function closeDialogFromNavigation() {
  fileManagerHistoryActive = false;
  closeDialogs();
}

async function handleFileManagerBackNavigation() {
  if (!isFileManagerOpen()) return;
  if (busy.value) {
    pushFileManagerHistoryState();
    return;
  }
  if (deleteDialogRef.value?.open) {
    cancelDelete();
    pushFileManagerHistoryState();
    return;
  }
  if (moveDialogRef.value?.open) {
    closeMoveDialog();
    pushFileManagerHistoryState();
    return;
  }
  if (textDialogRef.value?.open) {
    textDialogRef.value.close();
    pushFileManagerHistoryState();
    return;
  }
  if (searchDialogRef.value?.open) {
    searchDialogRef.value.close();
    pushFileManagerHistoryState();
    return;
  }
  if (parentPath.value !== null) {
    await navigateParent();
    pushFileManagerHistoryState();
    return;
  }
  closeDialogFromNavigation();
}

function handlePopState() {
  if (!isFileManagerOpen()) {
    fileManagerHistoryActive = false;
    return;
  }
  fileManagerHistoryActive = false;
  void handleFileManagerBackNavigation();
}

async function loadFiles(path = currentPath.value) {
  loading.value = true;
  try {
    const result = await invokeTauri<AndroidWorkspaceFileListResult>("list_android_workspace_files", {
      path: normalizeRelativePath(path),
    });
    currentPath.value = result.currentPath || "";
    parentPath.value = result.parentPath ?? null;
    entries.value = Array.isArray(result.entries) ? result.entries : [];
    selectedFilePath.value = "";
    setMessage("");
  } catch (error) {
    clearFileManager();
    setMessage(t("config.tools.androidWorkspaceFileManagerLoadFailed", { err: toErrorMessage(error) }), true);
  } finally {
    loading.value = false;
  }
}

async function navigateDirectory(path: string) {
  if (busy.value) return;
  await loadFiles(path);
}

async function navigateParent() {
  if (parentPath.value === null) return;
  await navigateDirectory(parentPath.value);
}

function openEntry(entry: AndroidWorkspaceFileEntry) {
  if (entry.kind === "directory") {
    void navigateDirectory(entry.path);
    return;
  }
  selectedFilePath.value = entry.path;
}

function chooseImportFile() {
  if (busy.value) return;
  if (isAndroidRuntime()) {
    void pickAndImportAndroidFile();
    return;
  }
  importInput.value?.click();
}

async function onImportFileChange(event: Event) {
  const input = event.target as HTMLInputElement | null;
  const file = input?.files?.[0];
  if (input) input.value = "";
  if (!file) return;
  importing.value = true;
  setMessage("");
  const targetPath = joinWorkspacePath(currentPath.value, file.name);
  try {
    const dataBase64 = arrayBufferToBase64(await file.arrayBuffer());
    const result = await invokeTauri<AndroidWorkspaceImportResult>("import_file_to_android_workspace", {
      fileName: file.name,
      mime: file.type || "application/octet-stream",
      dataBase64,
      targetPath,
    });
    emit("statusChanged", result);
    setMessage(t("config.tools.androidWorkspaceImportDone", { path: result.importedPath || targetPath }));
    await loadFiles(currentPath.value);
    selectedFilePath.value = result.importedPath || targetPath;
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceImportFailed", { err: toErrorMessage(error) }), true);
  } finally {
    importing.value = false;
  }
}

/** Android 上通过系统文件选择器拿 content URI，直接流式导入沙盒工作区（绕开 base64）。 */
async function pickAndImportAndroidFile() {
  if (busy.value || importing.value) return;
  importing.value = true;
  setMessage("");
  try {
    const dialog = await import("@tauri-apps/plugin-dialog");
    const selected = await dialog.open({ multiple: false, directory: false });
    const uri = String(selected || "").trim();
    if (!uri.startsWith("content://")) return;
    const result = await invokeTauri<AndroidWorkspaceImportResult>("import_android_workspace_file_from_uri", {
      file_name: "",
      uri,
      target_path: null,
    });
    emit("statusChanged", result);
    const fallbackPath = result.fileName ? joinWorkspacePath(currentPath.value, result.fileName) : "";
    setMessage(t("config.tools.androidWorkspaceImportDone", { path: result.importedPath || fallbackPath }));
    await loadFiles(currentPath.value);
    selectedFilePath.value = result.importedPath || fallbackPath;
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceImportFailed", { err: toErrorMessage(error) }), true);
  } finally {
    importing.value = false;
  }
}

async function exportSelectedFile() {
  const selected = selectedFile.value;
  if (!selected || exporting.value) {
    if (!selected) setMessage(t("config.tools.androidWorkspaceFileSelectRequired"), true);
    return;
  }
  exporting.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceExportResult>("export_file_from_android_workspace", { path: selected.path });
    const blob = base64ToBlob(result.dataBase64, result.mime);
    const file = new File([blob], result.fileName, { type: result.mime || "application/octet-stream" });
    const nav = navigator as Navigator & { canShare?: (data: { files?: File[] }) => boolean; share?: (data: { files?: File[]; title?: string }) => Promise<void> };
    if (nav.share && (!nav.canShare || nav.canShare({ files: [file] }))) {
      await nav.share({ files: [file], title: result.fileName });
    } else {
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = result.fileName;
      anchor.click();
      URL.revokeObjectURL(url);
    }
    setMessage(t("config.tools.androidWorkspaceExportDone", { path: result.path }));
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceExportFailed", { err: toErrorMessage(error) }), true);
  } finally {
    exporting.value = false;
  }
}

function requestDeleteSelectedFile() {
  const selected = selectedFile.value;
  if (!selected || deleting.value) return;
  deleteTarget.value = selected;
  deleteDialogRef.value?.showModal();
}

async function readSelectedFile() {
  const selected = selectedFile.value;
  if (!selected || reading.value) {
    if (!selected) setMessage(t("config.tools.androidWorkspaceFileSelectRequired"), true);
    return;
  }
  reading.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceTextResult>("read_android_workspace_text", { path: selected.path });
    textDialogPath.value = result.path;
    textDialogContent.value = result.text;
    textDialogReadOnly.value = true;
    textDialogOverwrite.value = false;
    textDialogRef.value?.showModal();
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceTextReadFailed", { err: toErrorMessage(error) }), true);
  } finally {
    reading.value = false;
  }
}

async function editSelectedFile() {
  const selected = selectedFile.value;
  if (!selected || saving.value) {
    if (!selected) setMessage(t("config.tools.androidWorkspaceFileSelectRequired"), true);
    return;
  }
  saving.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceTextResult>("read_android_workspace_text", { path: selected.path });
    textDialogPath.value = result.path;
    textDialogContent.value = result.text;
    textDialogReadOnly.value = false;
    textDialogOverwrite.value = true;
    textDialogRef.value?.showModal();
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceTextReadFailed", { err: toErrorMessage(error) }), true);
  } finally {
    saving.value = false;
  }
}

async function saveTextFile() {
  if (saving.value) return;
  const targetPath = normalizeRelativePath(textDialogPath.value);
  if (!targetPath) {
    setMessage(t("config.tools.androidWorkspaceMoveTargetRequired"), true);
    return;
  }
  saving.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceWriteResult>("write_android_workspace_text", {
      path: targetPath,
      text: textDialogContent.value,
      overwrite: textDialogOverwrite.value,
    });
    textDialogRef.value?.close();
    setMessage(t("config.tools.androidWorkspaceTextSaved", { path: result.entry.path }));
    await loadFiles(currentPath.value);
    selectedFilePath.value = result.entry.path;
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceTextSaveFailed", { err: toErrorMessage(error) }), true);
  } finally {
    saving.value = false;
  }
}

function requestMoveSelectedFile() {
  const selected = selectedFile.value;
  if (!selected || moving.value) {
    if (!selected) setMessage(t("config.tools.androidWorkspaceFileSelectRequired"), true);
    return;
  }
  moveSourcePath.value = selected.path;
  moveTargetPath.value = selected.path;
  moveOverwrite.value = false;
  moveDialogRef.value?.showModal();
}

async function confirmMoveFile() {
  if (moving.value) return;
  const source = moveSourcePath.value;
  const target = normalizeRelativePath(moveTargetPath.value);
  if (!source || !target) {
    setMessage(t("config.tools.androidWorkspaceMoveTargetRequired"), true);
    return;
  }
  moving.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceMoveResult>("move_android_workspace_file", {
      source,
      target,
      overwrite: moveOverwrite.value,
    });
    moveDialogRef.value?.close();
    setMessage(t("config.tools.androidWorkspaceMoveDone", { path: result.entry.path }));
    await loadFiles(currentPath.value);
    selectedFilePath.value = result.entry.path;
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceMoveFailed", { err: toErrorMessage(error) }), true);
  } finally {
    moving.value = false;
  }
}

function closeMoveDialog() {
  if (moveDialogRef.value?.open) {
    moveDialogRef.value.close();
  }
}

async function runWorkspaceSearch() {
  if (searching.value) return;
  if (searchMode.value === "glob" && !searchPattern.value.trim()) {
    setMessage(t("config.tools.androidWorkspaceSearchPatternRequired"), true);
    return;
  }
  if (searchMode.value === "grep" && !searchQuery.value.trim()) {
    setMessage(t("config.tools.androidWorkspaceSearchQueryRequired"), true);
    return;
  }
  searching.value = true;
  searchDone.value = false;
  searchGlobResults.value = [];
  searchGrepResults.value = [];
  setMessage("");
  try {
    const searchPathValue = normalizeRelativePath(searchPath.value);
    if (searchMode.value === "glob") {
      const result = await invokeTauri<AndroidWorkspaceGlobResult>("glob_android_workspace_files", {
        pattern: searchPattern.value.trim(),
        path: searchPathValue,
      });
      searchGlobResults.value = Array.isArray(result.entries) ? result.entries : [];
    } else {
      const result = await invokeTauri<AndroidWorkspaceGrepResult>("grep_android_workspace_files", {
        query: searchQuery.value,
        path: searchPathValue,
        regex: searchRegex.value,
        ignoreCase: searchIgnoreCase.value,
        includeGlob: searchIncludeGlob.value.trim() || null,
      });
      searchGrepResults.value = Array.isArray(result.matches) ? result.matches : [];
    }
    searchDone.value = true;
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceSearchFailed", { err: toErrorMessage(error) }), true);
  } finally {
    searching.value = false;
  }
}

async function openSearchResult(path: string, isDirectory: boolean) {
  searchDialogRef.value?.close();
  if (isDirectory) {
    await navigateDirectory(path);
  } else {
    selectedFilePath.value = path;
    setMessage("");
  }
}

function openSearchDialog() {
  searchDialogRef.value?.showModal();
}

function cancelDelete() {
  if (deleteDialogRef.value?.open) {
    deleteDialogRef.value.close();
  }
  deleteTarget.value = null;
}

async function confirmDelete() {
  const target = deleteTarget.value;
  if (!target || deleting.value) return;
  deleting.value = true;
  setMessage("");
  try {
    const result = await invokeTauri<AndroidWorkspaceDeleteResult>("delete_file_from_android_workspace", { path: target.path });
    cancelDelete();
    setMessage(t("config.tools.androidWorkspaceDeleteDone", { path: result.deletedPath }));
    await loadFiles(currentPath.value);
  } catch (error) {
    setMessage(t("config.tools.androidWorkspaceDeleteFailed", { err: toErrorMessage(error) }), true);
  } finally {
    deleting.value = false;
  }
}

function open(path = currentPath.value) {
  setMessage("");
  if (!dialogRef.value?.open) {
    dialogRef.value?.showModal();
    pushFileManagerHistoryState();
  }
  void loadFiles(path);
}

function close() {
  if (!dialogRef.value?.open) return;
  closeDialogs();
  clearFileManagerHistoryState();
}

onMounted(() => {
  window.addEventListener("popstate", handlePopState);
});

onBeforeUnmount(() => {
  window.removeEventListener("popstate", handlePopState);
  closeDialogs();
  fileManagerHistoryActive = false;
});

defineExpose({ open });
</script>
