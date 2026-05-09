<template>
  <div class="relative flex h-screen min-h-0 flex-col bg-base-100 text-base-content">
    <header class="flex h-10 shrink-0 items-end gap-2 bg-base-200 px-2" data-tauri-drag-region>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" title="打开文件" @click.stop="pickFile">
        <FilePlus class="h-4 w-4" />
      </button>
      <div class="flex min-w-0 flex-1 items-end gap-1 overflow-hidden" data-tauri-drag-region>
        <div
          v-for="tab in tabs"
          :key="tab.path"
          class="group flex h-9 min-w-0 max-w-64 flex-1 basis-0 items-center gap-2 overflow-hidden rounded-t-box border border-b-0 px-2 text-sm"
          :class="tab.path === activePath ? 'border-base-300 bg-base-100 text-base-content' : 'border-transparent bg-base-200 text-base-content/65 hover:bg-base-100/70 hover:text-base-content'"
          :title="tab.path"
          role="button"
          tabindex="0"
          :aria-selected="tab.path === activePath"
          @click="setActiveTab(tab.path)"
          @keydown.enter.prevent="setActiveTab(tab.path)"
          @keydown.space.prevent="setActiveTab(tab.path)"
        >
          <FileText class="h-4 w-4 shrink-0 opacity-70" />
          <span class="min-w-0 flex-1 truncate font-medium">{{ tab.title }}</span>
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 p-0 opacity-60 hover:opacity-100"
            title="关闭"
            @click.stop="closeTab(tab.path)"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" title="最小化" @click.stop="minimizeWindow">
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" :title="maximized ? '还原窗口' : '最大化'" @click.stop="toggleMaximizeWindow">
        <Square class="h-3.5 w-3.5" />
      </button>
      <button class="btn btn-sm btn-ghost shrink-0 hover:bg-error" type="button" title="关闭" @click.stop="closeWindow">
        <X class="h-3.5 w-3.5" />
      </button>
    </header>

    <div
      v-if="activeTab"
      class="relative flex h-9 shrink-0 items-center gap-2 border-x border-b border-base-300 bg-base-100 px-3 text-sm text-base-content/60"
    >
      <div class="relative min-w-0 flex-1">
        <div
          ref="addressScroller"
          class="file-reader-address-scroll flex min-w-0 items-center gap-1 overflow-x-auto overflow-y-hidden"
          @scroll="updateAddressScrollState"
          @wheel="handleAddressWheel"
        >
          <template v-for="segment in activePathSegments" :key="segment.key">
            <span v-if="segment.index > 0" class="shrink-0 text-base-content/35">›</span>
            <button
              type="button"
              class="inline-flex shrink-0 items-center rounded px-1.5 py-1 hover:bg-base-200 hover:text-base-content"
              :title="`浏览目录：${segment.path}`"
              @click="openDirectoryTree(segment.path)"
            >
              {{ segment.label }}
            </button>
          </template>
          <span v-if="activePathSegments.length > 0" class="shrink-0 text-base-content/35">›</span>
          <span
            class="inline-flex shrink-0 items-center rounded px-1.5 py-1 font-medium text-base-content/80"
            :title="activeTab.path"
          >
            {{ activeTab.title }}
          </span>
        </div>
        <div
          v-if="addressScrollState.scrollable"
          class="pointer-events-none absolute inset-x-0 bottom-0 h-px"
        >
          <div
            class="file-reader-address-scrollbar-thumb h-px rounded-full bg-primary/55"
            :style="addressScrollbarThumbStyle"
          ></div>
        </div>
      </div>
      <button class="btn btn-ghost btn-xs ml-1 shrink-0" type="button" :disabled="!activeTab" title="刷新" @click.stop="refreshActiveTab">
        <RefreshCw class="h-4 w-4" />
      </button>
      <button
        class="btn btn-ghost btn-xs h-6 min-h-6 w-6 shrink-0 px-0"
        type="button"
        :disabled="!activeTab"
        :title="activeTab?.rawMode ? '切换到渲染视图' : '切换到原文视图'"
        :aria-label="activeTab?.rawMode ? '当前为原文视图' : '当前为渲染视图'"
        @click.stop="toggleActiveRawMode"
      >
        <Code2 v-if="activeTab?.rawMode" class="h-4 w-4" />
        <Eye v-else class="h-4 w-4" />
      </button>
    </div>

    <div class="flex min-h-0 flex-1">
      <aside
        v-if="directoryTreeRoot"
        class="flex w-72 shrink-0 flex-col border-r border-base-300 bg-base-200/35"
      >
        <div class="flex h-9 shrink-0 items-center gap-2 border-b border-base-300 px-3 text-sm">
          <FolderOpen class="h-4 w-4 shrink-0 text-primary" />
          <div class="min-w-0 flex-1 truncate font-medium" :title="directoryTreeRoot.path">{{ directoryTreeRoot.name }}</div>
          <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" title="关闭目录树" @click="closeDirectoryTree">
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
        <div class="min-h-0 flex-1 overflow-auto py-1 text-sm">
          <div v-if="directoryTreeRoot.loading" class="flex items-center gap-2 px-3 py-2 text-xs opacity-65">
            <span class="loading loading-spinner loading-xs"></span>
            正在读取目录
          </div>
          <div v-else-if="directoryTreeRoot.error" class="px-3 py-2 text-xs text-error">
            {{ directoryTreeRoot.error }}
          </div>
          <div v-else-if="visibleTreeRows.length === 0" class="px-3 py-2 text-xs opacity-60">
            空目录
          </div>
          <template v-else>
            <div
              v-for="row in visibleTreeRows"
              :key="row.key"
              class="flex h-7 items-center gap-1 px-2"
              :class="row.kind === 'entry' && !row.entry.isDirectory && normalizePath(row.entry.path) === activePath ? 'bg-primary/10 text-primary' : 'hover:bg-base-300/55'"
              :style="{ paddingLeft: `${8 + row.depth * 14}px` }"
            >
              <template v-if="row.kind === 'entry'">
                <button
                  v-if="row.entry.isDirectory"
                  class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0"
                  type="button"
                  :title="isTreeDirectoryExpanded(row.entry.path) ? '收起目录' : '展开目录'"
                  @click.stop="toggleTreeDirectory(row.entry)"
                >
                  <ChevronDown v-if="isTreeDirectoryExpanded(row.entry.path)" class="h-3.5 w-3.5" />
                  <ChevronRight v-else class="h-3.5 w-3.5" />
                </button>
                <span v-else class="h-5 w-5 shrink-0"></span>
                <button
                  type="button"
                  class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-0.5 text-left"
                  :title="row.entry.path"
                  @click="row.entry.isDirectory ? toggleTreeDirectory(row.entry) : openOrActivatePath(row.entry.path)"
                >
                  <Folder v-if="row.entry.isDirectory" class="h-4 w-4 shrink-0 text-warning" />
                  <FileText v-else class="h-4 w-4 shrink-0 opacity-70" />
                  <span class="min-w-0 truncate">{{ row.entry.name }}</span>
                </button>
              </template>
              <template v-else>
                <span class="h-5 w-5 shrink-0"></span>
                <span class="truncate px-1 text-xs opacity-60">{{ row.text }}</span>
              </template>
            </div>
          </template>
        </div>
      </aside>

      <main class="min-h-0 flex-1 overflow-auto bg-base-100" :class="activeTab?.kind === 'markdown' && !activeTab?.rawMode ? '' : 'file-reader-code-main'">
        <div v-if="!activeTab" class="flex h-full items-center justify-center text-sm text-base-content/55">
          还没有打开文件。
        </div>
        <div v-else-if="activeTab.loading" class="flex h-full items-center justify-center gap-3 text-sm text-base-content/65">
          <span class="loading loading-spinner loading-sm"></span>
          正在读取文件
        </div>
        <div v-else-if="activeTab.error" class="m-4 rounded-box border border-error/30 bg-error/10 p-4 text-sm text-error">
          {{ activeTab.error }}
        </div>
        <div v-else-if="activeTab.rawMode" class="file-reader-code-view">
          <pre class="file-reader-raw-pre">{{ activeTab.content }}</pre>
        </div>
        <div v-else-if="activeTab.kind === 'markdown'" class="file-reader-content mx-auto w-full max-w-300 px-4 py-4">
          <MarkdownRender
            class="ecall-markdown-content max-w-none"
            custom-id="chat-markstream"
            :nodes="activeMarkdownNodes"
            :is-dark="markdownIsDark"
            :final="true"
            :max-live-nodes="0"
            :batch-rendering="false"
            :initial-render-batch-size="0"
            :render-batch-size="0"
            :render-batch-delay="0"
            :render-batch-budget-ms="0"
            :code-block-props="markdownCodeBlockProps"
            :mermaid-props="markdownMermaidProps"
            :typewriter="false"
          />
        </div>
        <div v-else class="file-reader-code-view" v-html="activeHighlightedCodeHtml"></div>
      </main>
    </div>

    <div
      v-if="fileDragActive"
      class="pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-base-100/70 backdrop-blur-[1px]"
    >
      <div class="rounded-box border border-primary/30 bg-base-100 px-5 py-3 text-sm font-medium text-primary shadow-lg">
        松开打开文件
      </div>
    </div>

    <div v-if="actionErrorMessage" class="toast toast-end toast-bottom z-50">
      <div class="alert alert-error max-w-xl text-sm shadow-lg">
        <span>{{ actionErrorMessage }}</span>
      </div>
    </div>
    <Win10ResizeHandles :enabled="!maximized" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronDown, ChevronRight, Code2, Eye, FilePlus, FileText, Folder, FolderOpen, Minus, RefreshCw, Square, X } from "lucide-vue-next";
import MarkdownRender, { enableKatex, enableMermaid, getMarkdown, parseMarkdownToStructure } from "markstream-vue";
import { bundledLanguagesInfo, codeToHtml } from "shiki";
import "markstream-vue/index.css";
import { invokeTauri } from "../../services/tauri-api";
import { registerChatMarkstreamComponents } from "../../features/chat/markdown/register-chat-markstream";
import { useAppTheme } from "../../features/shell/composables/use-app-theme";
import Win10ResizeHandles from "../../features/shell/components/Win10ResizeHandles.vue";

enableMermaid();
enableKatex();
registerChatMarkstreamComponents();

type FileReaderFilePayload = {
  path: string;
  name: string;
  extension: string;
  kind: "markdown" | "code" | string;
  content: string;
};

type FileReaderDirectoryEntry = {
  path: string;
  name: string;
  isDirectory: boolean;
};

type FileReaderDirectoryPayload = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
};

const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

type FileTab = {
  path: string;
  title: string;
  extension: string;
  kind: string;
  content: string;
  rawMode: boolean;
  loaded: boolean;
  loading: boolean;
  error: string;
};

type DirectoryNode = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
  loaded: boolean;
  loading: boolean;
  error: string;
  expanded: boolean;
};

type TreeRow =
  | {
      kind: "entry";
      key: string;
      depth: number;
      entry: FileReaderDirectoryEntry;
    }
  | {
      kind: "status";
      key: string;
      depth: number;
      text: string;
    };

type FileReaderSessionState = {
  tabs?: string[];
  activePath?: string;
  directoryRootPath?: string;
};

const FILE_READER_SESSION_STORAGE_KEY = "easy-call:file-reader-session:v1";

const CODE_LANGUAGE_BY_EXTENSION: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  c: "c",
  cc: "cpp",
  cpp: "cpp",
  cxx: "cpp",
  h: "c",
  hpp: "cpp",
  cs: "csharp",
  java: "java",
  kt: "kotlin",
  kts: "kotlin",
  go: "go",
  js: "javascript",
  jsx: "jsx",
  vue: "vue",
  rs: "rust",
  py: "python",
  rb: "ruby",
  php: "php",
  swift: "swift",
  scala: "scala",
  dart: "dart",
  lua: "lua",
  r: "r",
  m: "objective-c",
  mm: "objective-cpp",
  pl: "perl",
  pm: "perl",
  json: "json",
  jsonc: "jsonc",
  json5: "json5",
  toml: "toml",
  yaml: "yaml",
  yml: "yaml",
  css: "css",
  scss: "scss",
  sass: "sass",
  less: "less",
  html: "html",
  htm: "html",
  xml: "xml",
  svg: "xml",
  sql: "sql",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  fish: "fish",
  ps1: "powershell",
  bat: "bat",
  cmd: "bat",
  dockerfile: "dockerfile",
  ini: "ini",
  env: "dotenv",
  gitignore: "gitignore",
  gitattributes: "gitignore",
  editorconfig: "ini",
  lock: "text",
  csv: "csv",
  tsv: "tsv",
  txt: "text",
  log: "log",
  md: "markdown",
  markdown: "markdown",
  mdx: "mdx",
};

const markdownCodeBlockProps = {
  showHeader: true,
  showCopyButton: true,
  showPreviewButton: false,
  showExpandButton: true,
  showCollapseButton: true,
  showFontSizeButtons: true,
  enableFontSizeControl: true,
  isShowPreview: false,
  showTooltips: false,
};

const markdownMermaidProps = {
  showHeader: true,
  showCopyButton: true,
  showExportButton: false,
  showFullscreenButton: true,
  showCollapseButton: false,
  showZoomControls: true,
  showModeToggle: false,
  enableWheelZoom: true,
  showTooltips: false,
};

const markstreamMarkdown = getMarkdown();
const { currentTheme, restoreThemeFromStorage } = useAppTheme();
const tabs = ref<FileTab[]>([]);
const activePath = ref("");
const actionErrorMessage = ref("");
const highlightedCodeHtmlByPath = ref<Record<string, string>>({});
const directoryRootPath = ref("");
const directoryNodes = ref<Record<string, DirectoryNode>>({});
const markdownIsDark = computed(() => currentTheme.value !== "light");
const appWindow = getCurrentWindow();
const maximized = ref(false);
const fileDragActive = ref(false);
const addressScroller = ref<HTMLElement | null>(null);
const addressScrollState = ref({
  scrollable: false,
  left: 0,
  clientWidth: 0,
  scrollWidth: 0,
});
let unlistenOpenPath: UnlistenFn | null = null;
let unlistenFileDrop: UnlistenFn | null = null;

const activeTab = computed(() => tabs.value.find((tab) => tab.path === activePath.value) || tabs.value[0] || null);

const directoryTreeRoot = computed(() => {
  const rootPath = normalizePath(directoryRootPath.value);
  return rootPath ? directoryNodes.value[rootPath] || null : null;
});

const activeMarkdownSource = computed(() => {
  const tab = activeTab.value;
  if (!tab) return "";
  if (tab.rawMode) return "";
  return tab.kind === "markdown" ? stripMarkdownHtmlComments(tab.content) : "";
});

const activeMarkdownNodes = computed(() =>
  parseMarkdownToStructure(activeMarkdownSource.value, markstreamMarkdown, { final: true })
);

const activeHighlightedCodeHtml = computed(() => {
  const tab = activeTab.value;
  if (!tab) return "";
  if (!tab.loaded) return "";
  if (tab.rawMode) return "";
  return highlightedCodeHtmlByPath.value[tab.path] || escapeHtml(tab.content);
});

const addressScrollbarThumbStyle = computed(() => {
  const state = addressScrollState.value;
  if (!state.scrollable || state.clientWidth <= 0 || state.scrollWidth <= 0) {
    return { width: "0px", transform: "translateX(0)" };
  }
  const width = Math.max(20, Math.round((state.clientWidth / state.scrollWidth) * state.clientWidth));
  const maxLeft = Math.max(1, state.scrollWidth - state.clientWidth);
  const maxThumbLeft = Math.max(0, state.clientWidth - width);
  const left = Math.round((state.left / maxLeft) * maxThumbLeft);
  return {
    width: `${width}px`,
    transform: `translateX(${left}px)`,
  };
});

const visibleTreeRows = computed<TreeRow[]>(() => {
  const root = directoryTreeRoot.value;
  if (!root || root.loading || root.error) return [];
  return flattenDirectoryEntries(root.entries, 0);
});

const activePathSegments = computed(() => {
  const tab = activeTab.value;
  if (!tab) return [];
  const normalized = normalizePath(tab.path);
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length <= 1) return [];
  const dirs = parts.slice(0, -1);
  return dirs.map((label, index) => {
    const head = dirs[0]?.endsWith(":") ? `${dirs[0]}/` : dirs[0] || "";
    const path = index === 0
      ? head
      : [head.replace(/\/$/, ""), ...dirs.slice(1, index + 1)].join("/");
    return {
      key: `${index}:${label}`,
      index,
      label,
      path,
    };
  });
});

function updateAddressScrollState() {
  const el = addressScroller.value;
  if (!el) {
    addressScrollState.value = {
      scrollable: false,
      left: 0,
      clientWidth: 0,
      scrollWidth: 0,
    };
    return;
  }
  addressScrollState.value = {
    scrollable: el.scrollWidth > el.clientWidth + 1,
    left: el.scrollLeft,
    clientWidth: el.clientWidth,
    scrollWidth: el.scrollWidth,
  };
}

function scheduleAddressScrollStateUpdate() {
  void nextTick(() => updateAddressScrollState());
}

function handleAddressWheel(event: WheelEvent) {
  const el = addressScroller.value;
  if (!el) return;
  event.preventDefault();
  const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
  el.scrollLeft += delta;
  updateAddressScrollState();
}

function normalizePath(path: string) {
  return String(path || "")
    .trim()
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/^\/\?\//, "")
    .replace(/^\?\//, "")
    .replace(/^\?\\/, "")
    .replace(/\\/g, "/");
}

function createRestoredTab(path: string): FileTab {
  const normalizedPath = normalizePath(path);
  return {
    path: normalizedPath,
    title: titleFromPath(normalizedPath),
    extension: extensionFromPath(normalizedPath),
    kind: fileKindFromPath(normalizedPath),
    content: "",
    rawMode: false,
    loaded: false,
    loading: false,
    error: "",
  };
}

function fileKindFromPath(path: string) {
  const extension = extensionFromPath(path);
  return ["md", "markdown", "mdx"].includes(extension) ? "markdown" : "code";
}

function readFileReaderSessionState(): FileReaderSessionState {
  try {
    return JSON.parse(localStorage.getItem(FILE_READER_SESSION_STORAGE_KEY) || "{}") as FileReaderSessionState;
  } catch {
    return {};
  }
}

function persistFileReaderSession() {
  const uniqueTabs = Array.from(new Set(tabs.value.map((tab) => normalizePath(tab.path)).filter(Boolean)));
  const state: FileReaderSessionState = {
    tabs: uniqueTabs,
    activePath: normalizePath(activePath.value),
    directoryRootPath: normalizePath(directoryRootPath.value),
  };
  localStorage.setItem(FILE_READER_SESSION_STORAGE_KEY, JSON.stringify(state));
}

async function restoreFileReaderSession(loadActiveTab: boolean) {
  const state = readFileReaderSessionState();
  const restoredTabs = Array.from(new Set((state.tabs || []).map((path) => normalizePath(path)).filter(Boolean)));
  if (restoredTabs.length > 0) {
    tabs.value = restoredTabs.map(createRestoredTab);
    const restoredActivePath = normalizePath(state.activePath || "");
    activePath.value = restoredTabs.includes(restoredActivePath) ? restoredActivePath : restoredTabs[0];
    scheduleAddressScrollStateUpdate();
    if (loadActiveTab) {
      void openPath(activePath.value);
    }
  }

  const restoredDirectoryRoot = normalizePath(state.directoryRootPath || "");
  if (restoredDirectoryRoot) {
    directoryRootPath.value = restoredDirectoryRoot;
    void loadDirectory(restoredDirectoryRoot, true);
  }
}

function normalizeDirectoryEntries(entries: FileReaderDirectoryEntry[]) {
  return entries.map((entry) => ({
    ...entry,
    path: normalizePath(entry.path),
    name: String(entry.name || titleFromPath(entry.path)),
    isDirectory: !!entry.isDirectory,
  }));
}

function updateDirectoryNode(path: string, patch: Partial<DirectoryNode>) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const current = directoryNodes.value[normalizedPath] || {
    path: normalizedPath,
    name: titleFromPath(normalizedPath),
    entries: [],
    loaded: false,
    loading: false,
    error: "",
    expanded: false,
  };
  directoryNodes.value = {
    ...directoryNodes.value,
    [normalizedPath]: {
      ...current,
      ...patch,
      path: normalizedPath,
    },
  };
}

function treeDirectoryNode(path: string) {
  return directoryNodes.value[normalizePath(path)] || null;
}

function isTreeDirectoryExpanded(path: string) {
  return !!treeDirectoryNode(path)?.expanded;
}

function flattenDirectoryEntries(entries: FileReaderDirectoryEntry[], depth: number): TreeRow[] {
  const rows: TreeRow[] = [];
  for (const entry of entries) {
    const normalizedPath = normalizePath(entry.path);
    rows.push({
      kind: "entry",
      key: `entry:${normalizedPath}`,
      depth,
      entry: {
        ...entry,
        path: normalizedPath,
      },
    });
    if (!entry.isDirectory || !isTreeDirectoryExpanded(normalizedPath)) {
      continue;
    }
    const node = treeDirectoryNode(normalizedPath);
    if (!node || node.loading) {
      rows.push({ kind: "status", key: `loading:${normalizedPath}`, depth: depth + 1, text: "正在读取目录" });
    } else if (node.error) {
      rows.push({ kind: "status", key: `error:${normalizedPath}`, depth: depth + 1, text: node.error });
    } else if (node.loaded && node.entries.length === 0) {
      rows.push({ kind: "status", key: `empty:${normalizedPath}`, depth: depth + 1, text: "空目录" });
    } else if (node.loaded) {
      rows.push(...flattenDirectoryEntries(node.entries, depth + 1));
    }
  }
  return rows;
}

function extensionFromPath(path: string) {
  const fileName = titleFromPath(path);
  const lowerFileName = fileName.toLowerCase();
  if (CODE_LANGUAGE_BY_EXTENSION[lowerFileName]) return lowerFileName;
  if (SHIKI_LANGUAGE_KEYS.has(lowerFileName)) return lowerFileName;
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === fileName.length - 1) return "";
  const extension = fileName.slice(dotIndex + 1).toLowerCase();
  return CODE_LANGUAGE_BY_EXTENSION[extension] || SHIKI_LANGUAGE_KEYS.has(extension) ? extension : "";
}

function escapeHtml(value: string) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function stripMarkdownHtmlComments(value: string) {
  return String(value || "").replace(/<!--[\s\S]*?-->/g, "");
}

function normalizeShikiLineHtml(html: string) {
  return html.replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"');
}

async function updateHighlightedCode(tab: FileTab) {
  if (tab.kind === "markdown") return;
  const language = resolveShikiLanguage(tab.extension);
  try {
    const html = await codeToHtml(tab.content, {
      lang: language,
      theme: "github-dark",
    });
    highlightedCodeHtmlByPath.value = {
      ...highlightedCodeHtmlByPath.value,
      [tab.path]: normalizeShikiLineHtml(html),
    };
  } catch {
    highlightedCodeHtmlByPath.value = {
      ...highlightedCodeHtmlByPath.value,
      [tab.path]: escapeHtml(tab.content),
    };
  }
}

function resolveShikiLanguage(extension: string) {
  const key = String(extension || "").trim().toLowerCase();
  const mapped = CODE_LANGUAGE_BY_EXTENSION[key] || key;
  return SHIKI_LANGUAGE_KEYS.has(mapped) ? mapped : "text";
}

function titleFromPath(path: string) {
  const normalized = normalizePath(path);
  return normalized.split("/").filter(Boolean).pop() || normalized || "未命名文件";
}

function directoryFromPath(path: string) {
  const normalized = normalizePath(path).replace(/\/+$/, "");
  const slashIndex = normalized.lastIndexOf("/");
  if (slashIndex < 0) return "";
  if (slashIndex === 0) return "/";
  if (slashIndex === 2 && normalized[1] === ":") return normalized.slice(0, 3);
  return normalized.slice(0, slashIndex);
}

function replaceTabState(tab: FileTab, matchPath = tab.path) {
  const normalizedMatchPath = normalizePath(matchPath);
  tabs.value = tabs.value.map((item) => item.path === normalizedMatchPath ? { ...tab } : item);
}

function upsertLoadingTab(path: string, reuseActiveTab = false) {
  const normalizedPath = normalizePath(path);
  const existing = tabs.value.find((tab) => tab.path === normalizedPath);
  if (existing) {
    existing.loading = true;
    existing.error = "";
    existing.loaded = false;
    activePath.value = existing.path;
    replaceTabState(existing);
    persistFileReaderSession();
    scheduleAddressScrollStateUpdate();
    return existing;
  }
  const current = activeTab.value;
  if (reuseActiveTab && current && !current.loading) {
    const previousPath = current.path;
    deleteHighlightedCode(previousPath);
    current.path = normalizedPath;
    current.title = titleFromPath(normalizedPath);
    current.extension = "";
    current.kind = "code";
    current.content = "";
    current.rawMode = false;
    current.loaded = false;
    current.loading = true;
    current.error = "";
    activePath.value = normalizedPath;
    replaceTabState(current, previousPath);
    persistFileReaderSession();
    scheduleAddressScrollStateUpdate();
    return current;
  }
  const tab: FileTab = {
    path: normalizedPath,
    title: titleFromPath(normalizedPath),
    extension: "",
    kind: "code",
    content: "",
    rawMode: false,
    loaded: false,
    loading: true,
    error: "",
  };
  tabs.value = [...tabs.value, tab];
  activePath.value = normalizedPath;
  persistFileReaderSession();
  scheduleAddressScrollStateUpdate();
  return tab;
}

function setActiveTab(path: string) {
  const normalizedPath = normalizePath(path);
  activePath.value = normalizedPath;
  persistFileReaderSession();
  scheduleAddressScrollStateUpdate();
  const tab = tabs.value.find((item) => item.path === normalizedPath);
  if (tab && !tab.loaded && !tab.loading) {
    void openPath(normalizedPath);
  }
}

function toggleActiveRawMode() {
  const tab = activeTab.value;
  if (!tab) return;
  tab.rawMode = !tab.rawMode;
  replaceTabState(tab);
}

function openOrActivatePath(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const existing = tabs.value.some((tab) => tab.path === normalizedPath);
  if (existing) {
    setActiveTab(normalizedPath);
    return;
  }
  const current = activeTab.value;
  const sameDirectory = !!current && directoryFromPath(current.path) === directoryFromPath(normalizedPath);
  void openPath(normalizedPath, { reuseActiveTab: sameDirectory });
}

function deleteHighlightedCode(path: string) {
  const normalizedPath = normalizePath(path);
  const next = { ...highlightedCodeHtmlByPath.value };
  delete next[normalizedPath];
  highlightedCodeHtmlByPath.value = next;
}

function migrateTabPath(tab: FileTab, fromPath: string, toPath: string) {
  if (fromPath === toPath) return tab;
  const normalizedFromPath = normalizePath(fromPath);
  const duplicated = tabs.value.find((item) => item !== tab && item.path === toPath);
  if (duplicated) {
    tabs.value = tabs.value.filter((item) => item.path !== normalizedFromPath);
    deleteHighlightedCode(fromPath);
    activePath.value = toPath;
    persistFileReaderSession();
    scheduleAddressScrollStateUpdate();
    return duplicated;
  }
  const previousHighlightedHtml = highlightedCodeHtmlByPath.value[fromPath];
  if (previousHighlightedHtml) {
    const next = { ...highlightedCodeHtmlByPath.value };
    delete next[fromPath];
    next[toPath] = previousHighlightedHtml;
    highlightedCodeHtmlByPath.value = next;
  } else {
    deleteHighlightedCode(fromPath);
  }
  tab.path = toPath;
  activePath.value = toPath;
  replaceTabState(tab, fromPath);
  persistFileReaderSession();
  scheduleAddressScrollStateUpdate();
  return tab;
}

function reportFileReaderActionFailure(action: string, path: string, error: unknown) {
  const detail = error instanceof Error ? error.message : String(error);
  console.error(`[文件阅读窗口] ${action}失败`, {
    path,
    error,
  });
  actionErrorMessage.value = `${action}失败：${path}；${detail}`;
  window.setTimeout(() => {
    if (actionErrorMessage.value === `${action}失败：${path}；${detail}`) {
      actionErrorMessage.value = "";
    }
  }, 4500);
}

async function openPath(path: string, options: { reuseActiveTab?: boolean } = {}) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const current = tabs.value.find((tab) => tab.path === normalizedPath);
  if (current?.loading) {
    activePath.value = normalizedPath;
    persistFileReaderSession();
    scheduleAddressScrollStateUpdate();
    return;
  }
  let tab = upsertLoadingTab(normalizedPath, !!options.reuseActiveTab);
  try {
    const payload = await invokeTauri<FileReaderFilePayload>("read_file_reader_file", { path: normalizedPath });
    const resolvedPath = normalizePath(payload.path || normalizedPath);
    tab = migrateTabPath(tab, normalizedPath, resolvedPath);
    tab.title = payload.name || titleFromPath(resolvedPath);
    tab.extension = String(payload.extension || extensionFromPath(resolvedPath)).toLowerCase();
    tab.kind = String(payload.kind || "code");
    tab.content = String(payload.content || "");
    tab.rawMode = false;
    tab.loaded = true;
    tab.error = "";
    tab.loading = false;
    activePath.value = resolvedPath;
    replaceTabState(tab);
    await updateHighlightedCode(tab);
    persistFileReaderSession();
    scheduleAddressScrollStateUpdate();
  } catch (error) {
    tab.loaded = true;
    tab.loading = false;
    tab.error = error instanceof Error ? error.message : String(error);
    replaceTabState(tab);
    persistFileReaderSession();
  }
}

async function openDroppedPaths(paths: string[]) {
  const normalizedPaths = paths.map((path) => normalizePath(path)).filter(Boolean);
  for (const path of normalizedPaths) {
    await openPath(path);
  }
}

function closeTab(path: string) {
  const index = tabs.value.findIndex((tab) => tab.path === path);
  if (index < 0) return;
  const wasActive = activePath.value === path;
  tabs.value = tabs.value.filter((tab) => tab.path !== path);
  deleteHighlightedCode(path);
  if (wasActive) {
    activePath.value = tabs.value[Math.max(0, index - 1)]?.path || tabs.value[0]?.path || "";
    scheduleAddressScrollStateUpdate();
    const nextTab = tabs.value.find((tab) => tab.path === activePath.value);
    if (nextTab && !nextTab.loaded && !nextTab.loading) {
      void openPath(nextTab.path);
    }
  }
  persistFileReaderSession();
}

function refreshActiveTab() {
  const tab = activeTab.value;
  if (!tab) return;
  void openPath(tab.path);
}

async function pickFile() {
  const picked = await open({
    multiple: false,
    directory: false,
    title: "打开文件",
  });
  if (!picked || Array.isArray(picked)) return;
  await openPath(String(picked));
}

async function loadDirectory(path: string, expanded: boolean) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  updateDirectoryNode(normalizedPath, {
    loading: true,
    error: "",
    expanded,
  });
  try {
    const payload = await invokeTauri<FileReaderDirectoryPayload>("list_file_reader_directory", { path: normalizedPath });
    const resolvedPath = normalizePath(payload.path || normalizedPath);
    if (directoryRootPath.value === normalizedPath) {
      directoryRootPath.value = resolvedPath;
      persistFileReaderSession();
    }
    updateDirectoryNode(resolvedPath, {
      name: String(payload.name || titleFromPath(resolvedPath)),
      entries: normalizeDirectoryEntries(payload.entries || []),
      loaded: true,
      loading: false,
      error: "",
      expanded,
    });
  } catch (error) {
    updateDirectoryNode(normalizedPath, {
      loaded: false,
      loading: false,
      error: error instanceof Error ? error.message : String(error),
      expanded,
    });
  }
}

async function openDirectoryTree(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  directoryRootPath.value = normalizedPath;
  persistFileReaderSession();
  await loadDirectory(normalizedPath, true);
}

function closeDirectoryTree() {
  directoryRootPath.value = "";
  persistFileReaderSession();
}

async function toggleTreeDirectory(entry: FileReaderDirectoryEntry) {
  if (!entry.isDirectory) return;
  const normalizedPath = normalizePath(entry.path);
  const node = treeDirectoryNode(normalizedPath);
  if (node?.expanded) {
    updateDirectoryNode(normalizedPath, { expanded: false });
    return;
  }
  if (node?.loaded) {
    updateDirectoryNode(normalizedPath, { expanded: true, error: "" });
    return;
  }
  await loadDirectory(normalizedPath, true);
}

async function syncWindowState() {
  try {
    maximized.value = await appWindow.isMaximized();
  } catch {
    maximized.value = false;
  }
}

async function minimizeWindow() {
  await appWindow.minimize();
}

async function toggleMaximizeWindow() {
  await appWindow.toggleMaximize();
  await syncWindowState();
}

async function closeWindow() {
  await appWindow.hide();
}

onMounted(() => {
  restoreThemeFromStorage();
  void syncWindowState();
  window.addEventListener("resize", updateAddressScrollState);
  const path = new URLSearchParams(window.location.search).get("path") || "";
  void restoreFileReaderSession(!path).then(() => {
    if (path) {
      void openPath(path);
    }
  });
  void getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === "enter" || payload.type === "over") {
      fileDragActive.value = true;
      return;
    }
    fileDragActive.value = false;
    if (payload.type === "drop") {
      void openDroppedPaths(payload.paths);
    }
  }).then((unlisten) => {
    unlistenFileDrop = unlisten;
  }).catch((error) => {
    console.error("[文件阅读窗口] 注册拖入打开失败", error);
  });
  void listen<{ path?: string }>("file-reader-open-path", (event) => {
    void openPath(event.payload?.path || "");
  }).then((unlisten) => {
    unlistenOpenPath = unlisten;
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", updateAddressScrollState);
  unlistenOpenPath?.();
  unlistenFileDrop?.();
});
</script>

<style scoped>
.file-reader-address-scroll {
  scrollbar-width: none;
}

.file-reader-address-scroll::-webkit-scrollbar {
  display: none;
}

.file-reader-address-scrollbar-thumb {
  opacity: 0;
  transition: opacity 160ms ease;
}

.file-reader-address-scroll:hover + div .file-reader-address-scrollbar-thumb,
.file-reader-address-scroll:focus-within + div .file-reader-address-scrollbar-thumb {
  opacity: 1;
}

.file-reader-content :deep(.markdown-body),
.file-reader-content :deep(.markstream-body) {
  max-width: none;
}

.file-reader-code-main {
  background: #101828;
}

.file-reader-code-view {
  min-height: 100%;
  background: #101828;
}

.file-reader-code-view :deep(.shiki) {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 0;
  border: 0;
  border-radius: 0;
  background: #101828 !important;
  box-shadow: none;
  overflow: visible;
  counter-reset: file-reader-code-line;
}

.file-reader-raw-pre {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 1rem;
  white-space: pre;
  color: #e5e7eb;
}

.file-reader-code-view :deep(code) {
  display: block;
  min-width: max-content;
}

.file-reader-code-view :deep(.line) {
  display: block;
  min-height: 1.5em;
  line-height: 1.5;
  counter-increment: file-reader-code-line;
}

.file-reader-code-view :deep(.line::before) {
  content: counter(file-reader-code-line);
  display: inline-block;
  width: 2.75rem;
  padding-right: 0.75rem;
  text-align: right;
  color: #64748b;
  user-select: none;
}
</style>
