import { ImageIcon } from "@lucide/vue";
import { computed, defineComponent, h, nextTick, onBeforeUnmount, onMounted, ref, watch, type PropType } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { invokeTauri } from "../../../services/tauri-api";
import { isAssistantSpacePath } from "../utils/local-link";
import { resolveMarkdownImageSource, type MarkdownImagePreviewPayload } from "./MarkdownImage";

const assistantSpaceThumbnailCache = new Map<string, string>();
const assistantSpaceThumbnailPromiseCache = new Map<string, Promise<string>>();
const ASSISTANT_SPACE_THUMBNAIL_CACHE_LIMIT = 40;

function cacheAssistantSpaceThumbnail(path: string, dataUrl: string) {
  assistantSpaceThumbnailCache.delete(path);
  assistantSpaceThumbnailCache.set(path, dataUrl);
  while (assistantSpaceThumbnailCache.size > ASSISTANT_SPACE_THUMBNAIL_CACHE_LIMIT) {
    const oldestPath = assistantSpaceThumbnailCache.keys().next().value;
    if (!oldestPath) break;
    assistantSpaceThumbnailCache.delete(oldestPath);
  }
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
    const assistantSpaceThumbnailSrc = ref("");
    const source = computed(() => resolveMarkdownImageSource(imageProps.src, imageProps.localImageBasePath));
    let observer: IntersectionObserver | null = null;
    let thumbnailLoadVersion = 0;

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
      assistantSpaceThumbnailSrc.value = "";
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
      thumbnailLoadVersion += 1;
      observer?.disconnect();
      observer = null;
    });

    watch(source, async () => {
      resetObserver();
      await nextTick();
      observeRoot();
    });

    watch(
      [source, inViewport],
      ([current, visible]) => {
        const version = ++thumbnailLoadVersion;
        assistantSpaceThumbnailSrc.value = "";
        if (!visible || current.kind !== "local" || !isAssistantSpacePath(current.path)) return;
        const path = current.path;
        const cached = assistantSpaceThumbnailCache.get(path);
        if (cached) {
          assistantSpaceThumbnailSrc.value = cached;
          return;
        }
        const existing = assistantSpaceThumbnailPromiseCache.get(path);
        const task = existing || invokeTauri<{ dataUrl: string }>("read_local_chat_image_thumbnail", {
          input: { path },
        })
          .then((result) => {
            const dataUrl = String(result?.dataUrl || "").trim();
            if (dataUrl) cacheAssistantSpaceThumbnail(path, dataUrl);
            assistantSpaceThumbnailPromiseCache.delete(path);
            return dataUrl;
          })
          .catch((error) => {
            assistantSpaceThumbnailPromiseCache.delete(path);
            console.warn("[Markdown图片] Assistant Space 缩略图加载失败", { path, error });
            return "";
          });
        if (!existing) assistantSpaceThumbnailPromiseCache.set(path, task);
        void task.then((dataUrl) => {
          if (version !== thumbnailLoadVersion) return;
          assistantSpaceThumbnailSrc.value = dataUrl;
          if (!dataUrl) imageErrored.value = true;
        });
      },
      { immediate: true },
    );

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

      const assistantSpaceImage = current.kind === "local" && isAssistantSpacePath(current.path);
      const resolvedSrc = current.kind === "remote"
        ? current.src
        : assistantSpaceImage
          ? assistantSpaceThumbnailSrc.value
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
        inViewport.value && resolvedSrc
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
