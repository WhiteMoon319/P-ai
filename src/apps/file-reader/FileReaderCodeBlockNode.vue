<template>
  <div ref="wrapperRef" class="file-reader-shiki-codeblock markstream-vue">
    <MarkdownCodeBlockNode v-bind="resolvedProps" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { MarkdownCodeBlockNode } from "markstream-vue";
import type { CodeBlockNodeProps } from "markstream-vue";

const props = defineProps<CodeBlockNodeProps>();
const wrapperRef = ref<HTMLElement | null>(null);
const canExpand = ref(false);
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let resizeObserver: ResizeObserver | null = null;
let pendingResizeFrame = 0;
let pendingExpandFrame = 0;

const resolvedNode = computed(() => {
  if (!props.node?.diff) return props.node;
  return {
    ...props.node,
    language: "diff",
    code: String(props.node.raw ?? ""),
    diff: false,
    originalCode: undefined,
    updatedCode: undefined,
  };
});

function resolveCollapsedMaxHeightPx(contentEl: HTMLElement): number | null {
  if (typeof window === "undefined") return null;
  const computedStyle = window.getComputedStyle(contentEl);
  const rawMaxHeight = String(computedStyle.maxHeight || "").trim().toLowerCase();
  if (!rawMaxHeight || rawMaxHeight === "none") return null;
  const parsed = Number.parseFloat(rawMaxHeight);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function refreshExpandAvailability() {
  const wrapperEl = wrapperRef.value;
  if (!wrapperEl) return;
  const contentEl = wrapperEl.querySelector<HTMLElement>(".code-block-content");
  if (!contentEl) return;
  const collapsedMaxHeightPx = resolveCollapsedMaxHeightPx(contentEl);
  if (!collapsedMaxHeightPx) {
    canExpand.value = false;
    return;
  }
  canExpand.value = contentEl.scrollHeight > collapsedMaxHeightPx + 1;
}

function scheduleResizeDrivenRefresh() {
  if (typeof window === "undefined") {
    refreshExpandAvailability();
    return;
  }
  if (pendingResizeFrame) return;
  pendingResizeFrame = window.requestAnimationFrame(() => {
    pendingResizeFrame = 0;
    refreshExpandAvailability();
  });
}

function scheduleExpandAvailabilityRefresh() {
  if (refreshTimer) {
    clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  void nextTick(() => {
    refreshExpandAvailability();
    if (typeof window !== "undefined") {
      pendingExpandFrame = window.requestAnimationFrame(() => {
        pendingExpandFrame = 0;
        refreshExpandAvailability();
      });
    }
    refreshTimer = setTimeout(() => {
      refreshExpandAvailability();
      refreshTimer = null;
    }, 120);
  });
}

const resolvedProps = computed(() => {
  const { themes: _themes, isDark, ...rest } = props as CodeBlockNodeProps & { themes?: unknown; isDark?: boolean };
  const dark = Boolean(isDark);
  return {
    ...rest,
    node: resolvedNode.value,
    showExpandButton: Boolean(rest.showExpandButton && canExpand.value),
    darkTheme: "github-dark",
    lightTheme: "github-light",
    isDark: dark,
    stream: false,
    loading: false,
  };
});

onMounted(() => {
  if (typeof ResizeObserver !== "undefined" && wrapperRef.value) {
    resizeObserver = new ResizeObserver(() => {
      scheduleResizeDrivenRefresh();
    });
    resizeObserver.observe(wrapperRef.value);
  }
});

watch(
  () => [props.node?.code, props.node?.raw, props.node?.language],
  () => {
    scheduleExpandAvailabilityRefresh();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (refreshTimer) {
    clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  if (pendingResizeFrame && typeof window !== "undefined") {
    window.cancelAnimationFrame(pendingResizeFrame);
    pendingResizeFrame = 0;
  }
  if (pendingExpandFrame && typeof window !== "undefined") {
    window.cancelAnimationFrame(pendingExpandFrame);
    pendingExpandFrame = 0;
  }
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<style scoped>
.file-reader-shiki-codeblock {
  min-width: 0;
  --code-bg: var(--color-base-200);
  --code-border: hsl(var(--ms-border));
  --file-reader-code-header-bg: var(--color-base-200);
  --file-reader-code-header-border: var(--code-border);
  --file-reader-code-header-fg: color-mix(in srgb, var(--color-base-content) 68%, transparent);
  --file-reader-code-header-hover-bg: color-mix(in srgb, var(--color-base-content) 8%, transparent);
  --file-reader-code-content-bg: #101828;
  --file-reader-code-content-fg: #e5e7eb;
  --file-reader-code-line-number: #64748b;
}

.file-reader-shiki-codeblock :deep(.code-block-container) {
  background: var(--file-reader-code-header-bg) !important;
  border: 1px solid var(--file-reader-code-header-border);
  border-radius: 0.85rem;
  overflow: hidden;
  box-shadow: none;
}

.file-reader-shiki-codeblock :deep(.code-block-header),
.file-reader-shiki-codeblock :deep(.code-header-main) {
  background: var(--file-reader-code-header-bg) !important;
  border-color: var(--file-reader-code-header-border) !important;
  color: var(--file-reader-code-header-fg) !important;
  box-shadow: none !important;
}

.file-reader-shiki-codeblock :deep(.code-block-header) {
  border-bottom: 1px solid var(--file-reader-code-header-border) !important;
}

.file-reader-shiki-codeblock :deep(.code-header-title),
.file-reader-shiki-codeblock :deep(.code-header-caption),
.file-reader-shiki-codeblock :deep(.code-header-actions),
.file-reader-shiki-codeblock :deep(.code-block-header .icon-slot),
.file-reader-shiki-codeblock :deep(.code-block-header button),
.file-reader-shiki-codeblock :deep(.code-block-header [role="button"]) {
  color: var(--file-reader-code-header-fg) !important;
}

.file-reader-shiki-codeblock :deep(.code-block-header button:hover),
.file-reader-shiki-codeblock :deep(.code-block-header [role="button"]:hover) {
  background: var(--file-reader-code-header-hover-bg) !important;
  color: var(--color-base-content) !important;
}

.file-reader-shiki-codeblock :deep(.dropdown-content),
.file-reader-shiki-codeblock :deep(.menu),
.file-reader-shiki-codeblock :deep([role="menu"]),
.file-reader-shiki-codeblock :deep([role="menuitem"]),
.file-reader-shiki-codeblock :deep(.code-block-header [data-radix-menu-content]),
.file-reader-shiki-codeblock :deep(.code-block-header [data-slot="dropdown-content"]) {
  color: var(--color-base-content) !important;
}

.file-reader-shiki-codeblock :deep(.dropdown-content),
.file-reader-shiki-codeblock :deep(.menu),
.file-reader-shiki-codeblock :deep([role="menu"]),
.file-reader-shiki-codeblock :deep(.code-block-header [data-radix-menu-content]),
.file-reader-shiki-codeblock :deep(.code-block-header [data-slot="dropdown-content"]) {
  background: var(--color-base-100) !important;
  border-color: var(--file-reader-code-header-border) !important;
}

.file-reader-shiki-codeblock :deep(.dropdown-content button:hover),
.file-reader-shiki-codeblock :deep(.dropdown-content [role="menuitem"]:hover),
.file-reader-shiki-codeblock :deep(.menu button:hover),
.file-reader-shiki-codeblock :deep(.menu [role="menuitem"]:hover),
.file-reader-shiki-codeblock :deep([role="menuitem"]:hover),
.file-reader-shiki-codeblock :deep(.code-block-header [data-radix-menu-content] button:hover),
.file-reader-shiki-codeblock :deep(.code-block-header [data-radix-menu-content] [role="menuitem"]:hover),
.file-reader-shiki-codeblock :deep(.code-block-header [data-slot="dropdown-content"] button:hover),
.file-reader-shiki-codeblock :deep(.code-block-header [data-slot="dropdown-content"] [role="menuitem"]:hover) {
  color: var(--color-base-content) !important;
  background: var(--color-base-200) !important;
}

.file-reader-shiki-codeblock :deep(.code-block-content),
.file-reader-shiki-codeblock :deep(.code-block-render),
.file-reader-shiki-codeblock :deep(.code-fallback-plain),
.file-reader-shiki-codeblock :deep(pre.code-pre-fallback),
.file-reader-shiki-codeblock :deep(pre),
.file-reader-shiki-codeblock :deep(.shiki) {
  background: var(--file-reader-code-content-bg) !important;
  color: var(--file-reader-code-content-fg) !important;
  box-shadow: none !important;
}

.file-reader-shiki-codeblock :deep(.code-block-content) {
  border-top: 0 !important;
  padding-top: 0 !important;
}

.file-reader-shiki-codeblock :deep(.code-block-content > :first-child),
.file-reader-shiki-codeblock :deep(.code-block-render > :first-child) {
  margin-top: 0 !important;
}

.file-reader-shiki-codeblock :deep(.shiki span),
.file-reader-shiki-codeblock :deep(.line),
.file-reader-shiki-codeblock :deep(.line span) {
  color: inherit;
}

.file-reader-shiki-codeblock :deep(.line-number),
.file-reader-shiki-codeblock :deep(.line::before) {
  color: var(--file-reader-code-line-number);
}
</style>
