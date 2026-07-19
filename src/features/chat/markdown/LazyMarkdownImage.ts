import { ImageIcon } from "@lucide/vue";
import { computed, defineComponent, h, nextTick, onBeforeUnmount, onMounted, ref, watch, type PropType } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { isAbsoluteLocalPath, normalizeLocalLinkHref } from "../utils/local-link";
import type { MarkdownImagePreviewPayload } from "./MarkdownImage";

type MarkdownImageSource =
  | { kind: "remote"; src: string }
  | { kind: "local"; path: string }
  | { kind: "blocked"; label: string };

function hasUrlScheme(value: string): boolean {
  return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value);
}

function normalizeBaseLocalPath(value: string): string {
  return String(value || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
}

function resolveLazyMarkdownImageSource(rawSrc: string, basePath: string): MarkdownImageSource {
  const src = String(rawSrc || "").trim();
  if (!src) return { kind: "blocked", label: "" };
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

export default defineComponent({
  name: "LazyMarkdownImage",
  props: {
    src: { type: String, required: true },
    alt: { type: String, default: "" },
    localImageBasePath: { type: String, default: "" },
    onOpenPreview: {
      type: Function as PropType<(payload: MarkdownImagePreviewPayload) => void>,
      default: undefined,
    },
  },
  setup(imageProps) {
    const rootRef = ref<HTMLElement | null>(null);
    const inViewport = ref(false);
    const imageLoaded = ref(false);
    const imageErrored = ref(false);
    const source = computed(() => resolveLazyMarkdownImageSource(imageProps.src, imageProps.localImageBasePath));
    let observer: IntersectionObserver | null = null;

    function ensureVisible() {
      inViewport.value = true;
      observer?.disconnect();
      observer = null;
    }

    function resetObserver() {
      observer?.disconnect();
      observer = null;
      inViewport.value = false;
      imageLoaded.value = false;
      imageErrored.value = false;
    }

    function observeRoot() {
      if (typeof IntersectionObserver === "undefined") {
        ensureVisible();
        return;
      }
      observer = new IntersectionObserver((entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          ensureVisible();
        }
      }, { rootMargin: "200px 0px" });
      if (rootRef.value) observer.observe(rootRef.value);
    }

    onMounted(observeRoot);

    onBeforeUnmount(() => {
      observer?.disconnect();
      observer = null;
    });

    watch(source, async () => {
      resetObserver();
      await nextTick();
      observeRoot();
    });

    return () => {
      const current = source.value;
      const alt = String(imageProps.alt || "").trim();
      const openPreview = () => {
        if (typeof imageProps.onOpenPreview !== "function") return;
        if (current.kind === "remote") {
          imageProps.onOpenPreview({ src: current.src, alt });
          return;
        }
        if (current.kind === "local") {
          imageProps.onOpenPreview({ localPath: current.path, alt: alt || current.path });
        }
      };

      if (current.kind === "blocked") {
        return h("span", { ref: rootRef, class: "ecall-md-image-placeholder ecall-md-image-error" }, alt || current.label || imageProps.src);
      }

      const resolvedSrc = current.kind === "remote"
        ? current.src
        : convertFileSrc(current.path);
      const title = current.kind === "local" ? (alt || current.path) : alt;
      const imageClass = current.kind === "local" && isMemeImagePath(current.path)
        ? "ecall-md-meme-image"
        : "";
      const errorLabel = current.kind === "local"
        ? (alt || current.path.split(/[\\/]/).filter(Boolean).pop() || current.path)
        : (alt || current.src);

      return h("span", {
        ref: rootRef,
        class: "relative inline-block max-w-full align-middle",
      }, [
        !imageLoaded.value && !imageErrored.value
          ? h("span", {
            class: "ecall-md-image-skeleton skeleton inline-flex aspect-video w-64 max-w-full items-center justify-center rounded-lg",
            "aria-hidden": "true",
          }, [h(ImageIcon, { class: "h-8 w-8 text-base-content/20" })])
          : null,
        inViewport.value
          ? h("img", {
            class: ["ecall-md-image", imageClass, "cursor-zoom-in"],
            src: resolvedSrc,
            alt: title,
            title,
            // IntersectionObserver 已经负责懒加载；进入视口后必须立即发起请求。
            loading: "eager",
            decoding: "async",
            // 不能使用 display:none，否则浏览器可能永远不加载 lazy 图片，形成骨架死锁。
            style: imageLoaded.value
              ? undefined
              : { position: "absolute", inset: "0", width: "100%", height: "100%", opacity: "0", pointerEvents: "none" },
            onLoad: () => {
              imageLoaded.value = true;
              imageErrored.value = false;
            },
            onError: () => {
              imageLoaded.value = false;
              imageErrored.value = true;
            },
            onClick: (event: MouseEvent) => {
              event.preventDefault();
              event.stopPropagation();
              openPreview();
            },
          })
          : null,
        imageErrored.value
          ? h("span", {
            class: "ecall-md-image-placeholder ecall-md-image-error cursor-zoom-in",
            title,
            onClick: (event: MouseEvent) => {
              event.preventDefault();
              event.stopPropagation();
              openPreview();
            },
          }, errorLabel)
          : null,
      ]);
    };
  },
});
