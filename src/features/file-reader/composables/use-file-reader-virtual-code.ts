import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { getSingletonHighlighter, hastToHtml, type GrammarState } from "shiki";
import {
  FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX,
  FILE_READER_VIRTUAL_BLOCK_OVERSCAN,
  FILE_READER_VIRTUAL_BLOCK_PADDING_Y_PX,
} from "../constants";
import type { FileReaderFileBlockPayload, FileTab, VirtualCodeBlock } from "../types";
import {
  buildFileBlockKey,
  escapeHtml,
  normalizePath,
  resolveShikiLanguage,
  sameNormalizedPath,
  splitContentLines,
} from "../utils";

type UseFileReaderVirtualCodeOptions = {
  activeTab: ComputedRef<FileTab | null>;
  markdownIsDark: ComputedRef<boolean>;
  virtualCodeScroller: Ref<HTMLElement | null>;
  isRawMode: (tab: FileTab | null | undefined) => boolean;
  requestFileBlock: (path: string, startLine: number, lineCount: number) => Promise<FileReaderFileBlockPayload>;
};

type HighlightStateEntry = {
  version: string;
  state?: GrammarState;
};

export function useFileReaderVirtualCode(options: UseFileReaderVirtualCodeOptions) {
  const highlightedCodeHtmlByBlockKey = ref<Record<string, string>>({});
  const fileBlockContentByKey = ref<Record<string, string>>({});
  const fileBlockLoadingByKey = ref<Record<string, boolean>>({});
  const fileBlockErrorByKey = ref<Record<string, string>>({});
  const blockLoadPromises = new Map<string, Promise<string>>();
  const grammarStateByBlockKey = new Map<string, HighlightStateEntry>();
  const highlightedVersionByBlockKey = new Map<string, string>();
  let activeHighlightRefreshId = 0;

  const activeShikiTheme = computed(() => (options.markdownIsDark.value ? "github-dark" : "github-light"));

  const activeVirtualCodeBlocks = computed<VirtualCodeBlock[]>(() => {
    const tab = options.activeTab.value;
    if (!tab || !tab.virtualized) return [];
    const totalLines = Math.max(0, tab.totalLines);
    const blockLineCount = Math.max(1, tab.blockLineCount || 120);
    const blocks: VirtualCodeBlock[] = [];
    for (let startLine = 1; startLine <= totalLines; startLine += blockLineCount) {
      const endLine = Math.min(totalLines, startLine + blockLineCount - 1);
      blocks.push({
        key: buildFileBlockKey(tab.path, startLine, endLine),
        path: tab.path,
        startLine,
        endLine,
        lineCount: endLine - startLine + 1,
      });
    }
    return blocks;
  });

  const virtualCodeBlockVirtualizer = useVirtualizer(
    computed(() => ({
      count: activeVirtualCodeBlocks.value.length,
      getScrollElement: () => options.virtualCodeScroller.value,
      getItemKey: (index: number) => activeVirtualCodeBlocks.value[index]?.key ?? `file-block-${index}`,
      estimateSize: (index: number) => {
        const block = activeVirtualCodeBlocks.value[index];
        if (!block) return FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX;
        return block.lineCount * FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX + FILE_READER_VIRTUAL_BLOCK_PADDING_Y_PX * 2;
      },
      overscan: FILE_READER_VIRTUAL_BLOCK_OVERSCAN,
      measureElement: (element: Element) => (element as HTMLElement).getBoundingClientRect().height,
    })),
  );

  const activeVirtualCodeEntries = computed(() => {
    const rows = virtualCodeBlockVirtualizer.value.getVirtualItems();
    return rows.map((row) => ({
      row,
      block: activeVirtualCodeBlocks.value[row.index],
    })).filter((entry): entry is { row: (typeof rows)[number]; block: VirtualCodeBlock } => Boolean(entry.block));
  });

  const activeVirtualCodeTotalSize = computed(() => virtualCodeBlockVirtualizer.value.getTotalSize());

  const virtualCodeLineNumberDigits = computed(() => {
    const totalLines = Math.max(1, options.activeTab.value?.totalLines || 1);
    return Math.max(2, String(totalLines).length);
  });

  function blockContentText(blockKey: string) {
    return fileBlockContentByKey.value[blockKey] || "";
  }

  function blockContentHtml(blockKey: string) {
    return highlightedCodeHtmlByBlockKey.value[blockKey] || escapeHtml(blockContentText(blockKey));
  }

  function normalizeShikiLineHtml(html: string) {
    return html
      .replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"')
      .replace(/<span class="line"><\/span>/g, '<span class="line"><span class="file-reader-code-empty-line">&#8203;</span></span>');
  }

  async function renderHighlightedCodeHtml(tab: FileTab, content: string, grammarState?: GrammarState) {
    const language = resolveShikiLanguage(tab.extension);
    const theme = activeShikiTheme.value;
    const highlighter = await getSingletonHighlighter({ langs: [language], themes: [theme] });
    const root = highlighter.codeToHast(content, {
      lang: language,
      theme,
      ...(grammarState ? { grammarState } : {}),
    });
    return {
      html: normalizeShikiLineHtml(hastToHtml(root)),
      grammarState: highlighter.getLastGrammarState(root),
    };
  }

  function highlightVersion(tab: FileTab) {
    return [
      normalizePath(tab.path),
      tab.extension,
      tab.totalLines,
      tab.blockLineCount,
      activeShikiTheme.value,
    ].join("::");
  }

  async function loadVirtualCodeBlockContent(block: VirtualCodeBlock): Promise<string> {
    const existing = fileBlockContentByKey.value[block.key];
    if (existing !== undefined) return existing;
    const pending = blockLoadPromises.get(block.key);
    if (pending) return pending;

    const promise = (async () => {
      fileBlockLoadingByKey.value = { ...fileBlockLoadingByKey.value, [block.key]: true };
      try {
        const payload = await options.requestFileBlock(block.path, block.startLine, block.lineCount);
        const normalizedKey = buildFileBlockKey(payload.path || block.path, payload.startLine, payload.endLine);
        const content = String(payload.content || "");
        fileBlockContentByKey.value = { ...fileBlockContentByKey.value, [normalizedKey]: content };
        const errorNext = { ...fileBlockErrorByKey.value };
        delete errorNext[block.key];
        fileBlockErrorByKey.value = errorNext;
        return content;
      } catch (error) {
        fileBlockErrorByKey.value = { ...fileBlockErrorByKey.value, [block.key]: error instanceof Error ? error.message : String(error) };
        throw error;
      } finally {
        const loadingNext = { ...fileBlockLoadingByKey.value };
        delete loadingNext[block.key];
        fileBlockLoadingByKey.value = loadingNext;
        blockLoadPromises.delete(block.key);
      }
    })();

    blockLoadPromises.set(block.key, promise);
    return promise;
  }

  async function ensureHighlightedThroughBlock(tab: FileTab, targetBlock: VirtualCodeBlock) {
    if (options.isRawMode(tab) || tab.kind === "markdown") return;
    const version = highlightVersion(tab);
    if (highlightedVersionByBlockKey.get(targetBlock.key) === version) return;

    const blocks = activeVirtualCodeBlocks.value;
    const targetIndex = blocks.findIndex((item) => item.key === targetBlock.key);
    if (targetIndex < 0) return;

    let startIndex = 0;
    let grammarState: GrammarState | undefined;
    for (let index = targetIndex - 1; index >= 0; index -= 1) {
      const entry = grammarStateByBlockKey.get(blocks[index].key);
      if (entry?.version === version) {
        startIndex = index + 1;
        grammarState = entry.state;
        break;
      }
    }

    const refreshId = activeHighlightRefreshId;
    for (let index = startIndex; index <= targetIndex; index += 1) {
      const block = blocks[index];
      if (!block) continue;
      const currentActiveTab = options.activeTab.value;
      if (refreshId !== activeHighlightRefreshId || !currentActiveTab || !sameNormalizedPath(currentActiveTab.path, tab.path)) return;
      if (highlightedVersionByBlockKey.get(block.key) === version) {
        grammarState = grammarStateByBlockKey.get(block.key)?.state;
        continue;
      }

      const content = await loadVirtualCodeBlockContent(block);
      if (refreshId !== activeHighlightRefreshId) return;

      try {
        const result = await renderHighlightedCodeHtml(tab, content, grammarState);
        highlightedCodeHtmlByBlockKey.value = {
          ...highlightedCodeHtmlByBlockKey.value,
          [block.key]: result.html,
        };
        highlightedVersionByBlockKey.set(block.key, version);
        grammarStateByBlockKey.set(block.key, { version, state: result.grammarState });
        grammarState = result.grammarState;
      } catch {
        highlightedCodeHtmlByBlockKey.value = { ...highlightedCodeHtmlByBlockKey.value, [block.key]: escapeHtml(content) };
        highlightedVersionByBlockKey.set(block.key, version);
        grammarStateByBlockKey.set(block.key, { version, state: undefined });
        grammarState = undefined;
      }
    }
  }

  async function ensureVirtualCodeBlockLoaded(block: VirtualCodeBlock) {
    if (!block.path) return;
    const tab = options.activeTab.value;
    if (!tab || !sameNormalizedPath(tab.path, block.path)) return;
    try {
      await loadVirtualCodeBlockContent(block);
      await ensureHighlightedThroughBlock(tab, block);
    } catch {
      // loadVirtualCodeBlockContent records the visible block error state.
    }
  }

  function clearFileBlockCaches(path: string) {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) return;
    const contentNext = { ...fileBlockContentByKey.value };
    const loadingNext = { ...fileBlockLoadingByKey.value };
    const errorNext = { ...fileBlockErrorByKey.value };
    const htmlNext = { ...highlightedCodeHtmlByBlockKey.value };
    for (const key of new Set([
      ...Object.keys(contentNext),
      ...Object.keys(loadingNext),
      ...Object.keys(errorNext),
      ...Object.keys(htmlNext),
      ...Array.from(highlightedVersionByBlockKey.keys()),
      ...Array.from(grammarStateByBlockKey.keys()),
    ])) {
      if (!key.startsWith(`${normalizedPath}::`)) continue;
      delete contentNext[key];
      delete loadingNext[key];
      delete errorNext[key];
      delete htmlNext[key];
      blockLoadPromises.delete(key);
      highlightedVersionByBlockKey.delete(key);
      grammarStateByBlockKey.delete(key);
    }
    fileBlockContentByKey.value = contentNext;
    fileBlockLoadingByKey.value = loadingNext;
    fileBlockErrorByKey.value = errorNext;
    highlightedCodeHtmlByBlockKey.value = htmlNext;
  }

  async function refreshActiveCodeHighlights() {
    const active = options.activeTab.value;
    if (!active || options.isRawMode(active) || active.kind === "markdown") return;
    activeHighlightRefreshId += 1;
    clearHighlightCachesForPath(active.path);
    const refreshId = activeHighlightRefreshId;
    for (const entry of activeVirtualCodeEntries.value) {
      if (refreshId !== activeHighlightRefreshId) return;
      await ensureVirtualCodeBlockLoaded(entry.block);
    }
  }

  function clearHighlightCachesForPath(path: string) {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) return;
    const htmlNext = { ...highlightedCodeHtmlByBlockKey.value };
    for (const key of new Set([
      ...Object.keys(htmlNext),
      ...Array.from(highlightedVersionByBlockKey.keys()),
      ...Array.from(grammarStateByBlockKey.keys()),
    ])) {
      if (!key.startsWith(`${normalizedPath}::`)) continue;
      delete htmlNext[key];
      highlightedVersionByBlockKey.delete(key);
      grammarStateByBlockKey.delete(key);
    }
    highlightedCodeHtmlByBlockKey.value = htmlNext;
  }

  function collectVirtualizedVisibleContent(tab: FileTab, lineRange: { startLine: number; endLine: number }) {
    const chunks: string[] = [];
    for (const block of activeVirtualCodeBlocks.value) {
      if (block.path !== tab.path) continue;
      if (block.endLine < lineRange.startLine || block.startLine > lineRange.endLine) continue;
      const blockContent = blockContentText(block.key);
      if (!blockContent) continue;
      const blockLines = splitContentLines(blockContent);
      const sliceStart = Math.max(0, lineRange.startLine - block.startLine);
      const sliceEndExclusive = Math.min(blockLines.length, lineRange.endLine - block.startLine + 1);
      if (sliceEndExclusive <= sliceStart) continue;
      chunks.push(blockLines.slice(sliceStart, sliceEndExclusive).join("\n"));
    }
    return chunks.join("\n").trim();
  }

  function measureVirtualCodeRow(element: Element | { $el?: Element } | null) {
    if (!element) return;
    const target = element instanceof Element ? element : element.$el;
    if (!(target instanceof Element)) return;
    virtualCodeBlockVirtualizer.value.measureElement(target);
  }

  watch(
    activeVirtualCodeEntries,
    (entries) => {
      for (const entry of entries) {
        void ensureVirtualCodeBlockLoaded(entry.block);
      }
    },
    { immediate: true },
  );

  watch(activeShikiTheme, () => {
    void refreshActiveCodeHighlights();
  });

  return {
    highlightedCodeHtmlByBlockKey,
    fileBlockContentByKey,
    fileBlockLoadingByKey,
    fileBlockErrorByKey,
    activeVirtualCodeBlocks,
    virtualCodeBlockVirtualizer,
    activeVirtualCodeEntries,
    activeVirtualCodeTotalSize,
    virtualCodeLineNumberDigits,
    activeShikiTheme,
    blockContentText,
    blockContentHtml,
    clearFileBlockCaches,
    ensureVirtualCodeBlockLoaded,
    refreshActiveCodeHighlights,
    collectVirtualizedVisibleContent,
    measureVirtualCodeRow,
  };
}
