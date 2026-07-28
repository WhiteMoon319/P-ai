import { computed, defineComponent, h, ref, watch } from "vue";
import { readTransportChatImage } from "../../../services/tauri-api";
import {
  isAbsoluteLocalPath,
  isAssistantSpacePath,
  normalizeAssistantSpacePath,
  normalizeLocalLinkHref,
} from "../utils/local-link";
import { stableMarkdownRuntimeKey } from "./markdown-runtime-key";

export type MarkdownImageSource =
  | { kind: "remote"; src: string }
  | { kind: "local"; path: string }
  | { kind: "blocked"; label: string };

export type MarkdownImagePreviewPayload = {
  src?: string;
  localPath?: string;
  alt?: string;
};

const markdownImageThumbnailCache = new Map<string, string>();
const markdownImageThumbnailPromiseCache = new Map<string, Promise<string>>();
const MARKDOWN_IMAGE_THUMBNAIL_SESSION_PREFIX = "easy_call.markdown_thumbnail.v1:";
const MARKDOWN_IMAGE_THUMBNAIL_SESSION_INDEX_KEY = "easy_call.markdown_thumbnail_index.v1";
const MARKDOWN_IMAGE_THUMBNAIL_SESSION_LIMIT = 40;


function hasUrlScheme(value: string): boolean {
  return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value);
}

export function normalizeMarkdownImageCacheKey(path: string): string {
  const normalized = normalizeLocalLinkHref(path).trim().replace(/\\/g, "/");
  if (/^[A-Za-z]:\//.test(normalized)) {
    return `${normalized.slice(0, 1).toLowerCase()}${normalized.slice(1)}`;
  }
  return normalized.toLowerCase();
}

function readMarkdownThumbnailSessionIndex(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.sessionStorage.getItem(MARKDOWN_IMAGE_THUMBNAIL_SESSION_INDEX_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.map((item) => String(item || "").trim()).filter(Boolean) : [];
  } catch {
    return [];
  }
}

function writeMarkdownThumbnailSessionIndex(index: string[]) {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(MARKDOWN_IMAGE_THUMBNAIL_SESSION_INDEX_KEY, JSON.stringify(index));
  } catch {
    // ignore sessionStorage quota / privacy mode failures
  }
}

function markdownThumbnailSessionStorageKey(cacheKey: string): string {
  return `${MARKDOWN_IMAGE_THUMBNAIL_SESSION_PREFIX}${stableMarkdownRuntimeKey(cacheKey)}`;
}

function readMarkdownThumbnailFromSession(cacheKey: string): string {
  if (typeof window === "undefined") return "";
  try {
    const raw = window.sessionStorage.getItem(markdownThumbnailSessionStorageKey(cacheKey));
    if (!raw) return "";
    const parsed = JSON.parse(raw) as { key?: string; dataUrl?: string } | null;
    if (!parsed || parsed.key !== cacheKey) return "";
    return String(parsed.dataUrl || "").trim();
  } catch {
    return "";
  }
}

function writeMarkdownThumbnailToSession(cacheKey: string, dataUrl: string) {
  if (typeof window === "undefined" || !dataUrl) return;
  try {
    const storageKey = markdownThumbnailSessionStorageKey(cacheKey);
    window.sessionStorage.setItem(storageKey, JSON.stringify({ key: cacheKey, dataUrl }));
    const nextIndex = readMarkdownThumbnailSessionIndex().filter((item) => item !== cacheKey);
    nextIndex.unshift(cacheKey);
    while (nextIndex.length > MARKDOWN_IMAGE_THUMBNAIL_SESSION_LIMIT) {
      const removedKey = nextIndex.pop();
      if (!removedKey) continue;
      window.sessionStorage.removeItem(markdownThumbnailSessionStorageKey(removedKey));
    }
    writeMarkdownThumbnailSessionIndex(nextIndex);
  } catch {
    // ignore sessionStorage quota / privacy mode failures
  }
}

function normalizeBaseLocalPath(value: string): string {
  return String(value || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
}

export function resolveMarkdownImageSource(rawSrc: string, basePath: string): MarkdownImageSource {
  const src = String(rawSrc || "").trim();
  if (!src) return { kind: "blocked", label: "" };
  if (isAssistantSpacePath(src)) {
    return { kind: "local", path: normalizeAssistantSpacePath(src) };
  }
  if (/^(https?:|data:image\/)/i.test(src)) return { kind: "remote", src };
  if (/^(blob:|javascript:|mailto:)/i.test(src)) return { kind: "blocked", label: src };
  const normalized = normalizeLocalLinkHref(src);
  if (isAbsoluteLocalPath(normalized)) return { kind: "local", path: normalized };
  if (hasUrlScheme(normalized)) return { kind: "blocked", label: normalized };
  const root = normalizeBaseLocalPath(basePath);
  if (!root) return { kind: "local", path: normalized };
  return { kind: "local", path: `${root}/${normalized.replace(/^\.\//, "")}` };
}

function isMemeImagePath(path: string): boolean {
  return /(^|[/\\])\.meme([/\\]|$)/.test(String(path || ""));
}

const MarkdownImage = defineComponent({
  name: "MarkdownImage",
  props: {
    src: { type: String, required: true },
    alt: { type: String, default: "" },
    localImageBasePath: { type: String, default: "" },
    onOpenPreview: {
      type: Function as import("vue").PropType<(payload: MarkdownImagePreviewPayload) => void>,
      default: undefined,
    },
  },
  setup(imageProps) {
    const thumbnailSrc = ref("");
    const loadError = ref(false);
    const source = computed(() => resolveMarkdownImageSource(imageProps.src, imageProps.localImageBasePath));

    watch(
      source,
      (next, _previous, onCleanup) => {
        thumbnailSrc.value = "";
        loadError.value = false;
        if (next.kind !== "local" || !next.path.trim()) return;
        const path = next.path.trim();
        const cacheKey = normalizeMarkdownImageCacheKey(path);
        const cached = markdownImageThumbnailCache.get(cacheKey) || readMarkdownThumbnailFromSession(cacheKey);
        if (cached) {
          markdownImageThumbnailCache.set(cacheKey, cached);
          thumbnailSrc.value = cached;
          return;
        }
        let cancelled = false;
        onCleanup(() => {
          cancelled = true;
        });
        const existing = markdownImageThumbnailPromiseCache.get(cacheKey);
        const task = existing || readTransportChatImage({ path })
          .then((result) => {
            const dataUrl = String(result?.dataUrl || "").trim();
            if (dataUrl) {
              markdownImageThumbnailCache.set(cacheKey, dataUrl);
              writeMarkdownThumbnailToSession(cacheKey, dataUrl);
            }
            markdownImageThumbnailPromiseCache.delete(cacheKey);
            return dataUrl;
          })
          .catch((error) => {
            markdownImageThumbnailPromiseCache.delete(cacheKey);
            console.warn("[Markdown图片] 本地缩略图加载失败", { path, error });
            return "";
          });
        if (!existing) markdownImageThumbnailPromiseCache.set(cacheKey, task);
        void task.then((dataUrl) => {
          if (cancelled) return;
          if (dataUrl) {
            thumbnailSrc.value = dataUrl;
          } else {
            loadError.value = true;
          }
        });
      },
      { immediate: true },
    );

    return () => {
      const current = source.value;
      const alt = String(imageProps.alt || "").trim();
      const emitOpenPreview = () => {
        if (typeof imageProps.onOpenPreview !== "function") return;
        if (current.kind === "remote") {
          imageProps.onOpenPreview({ src: current.src, alt });
          return;
        }
        if (current.kind === "local") {
          imageProps.onOpenPreview({ localPath: current.path, alt: alt || current.path });
        }
      };
      const memeImageClass = current.kind === "local" && isMemeImagePath(current.path)
        ? "ecall-md-meme-image"
        : "";
      if (current.kind === "remote") {
        return h("img", {
          class: "ecall-md-image cursor-zoom-in",
          src: current.src,
          alt,
          loading: "lazy",
          decoding: "async",
          onClick: (event: MouseEvent) => {
            event.preventDefault();
            event.stopPropagation();
            emitOpenPreview();
          },
        });
      }
      if (current.kind === "local") {
        const title = alt || current.path;
        if (thumbnailSrc.value) {
          return h("img", {
            class: ["ecall-md-image", "ecall-md-local-image", memeImageClass, "cursor-zoom-in"],
            src: thumbnailSrc.value,
            alt: title,
            title,
            loading: "lazy",
            decoding: "async",
            "data-local-image-path": current.path,
            onClick: (event: MouseEvent) => {
              event.preventDefault();
              event.stopPropagation();
              emitOpenPreview();
            },
          });
        }
        return h("span", {
          class: ["ecall-md-image-placeholder", loadError.value ? "ecall-md-image-error" : "", "cursor-zoom-in"],
          title,
          "data-local-image-path": current.path,
          onClick: (event: MouseEvent) => {
            event.preventDefault();
            event.stopPropagation();
            emitOpenPreview();
          },
        }, alt || current.path.split(/[\\/]/).filter(Boolean).pop() || current.path);
      }
      return h("span", { class: "ecall-md-image-placeholder ecall-md-image-error" }, alt || current.label || imageProps.src);
    };
  },
});

export default MarkdownImage;
