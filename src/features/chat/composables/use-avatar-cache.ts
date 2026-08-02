import { ref, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { PersonaProfile } from "../../../types/app";

type UseAvatarCacheOptions = {
  personas: Ref<PersonaProfile[]>;
};

export function useAvatarCache(options: UseAvatarCacheOptions) {
  const avatarDataUrlCache = ref<Record<string, string>>({});
  const BRAND_AVATAR_KEY = "__pai_brand_avatar__";

  function avatarCacheKey(path?: string, updatedAt?: string): string {
    if (!path) return "";
    return `${path}|${updatedAt || ""}`;
  }

  function resolveAvatarUrl(path?: string, updatedAt?: string): string {
    const key = avatarCacheKey(path, updatedAt);
    if (!key) return "";
    return avatarDataUrlCache.value[key] || "";
  }

  function resolveBrandAvatarUrl(): string {
    return avatarDataUrlCache.value[BRAND_AVATAR_KEY] || "";
  }

  async function ensureAvatarCached(path?: string, updatedAt?: string) {
    const key = avatarCacheKey(path, updatedAt);
    if (!key || avatarDataUrlCache.value[key]) return;
    try {
      const result = await invokeTauri<{ dataUrl: string }>("read_avatar_data_url", {
        input: { path },
      });
      avatarDataUrlCache.value = {
        ...avatarDataUrlCache.value,
        [key]: result.dataUrl || "",
      };
    } catch {
      // ignore avatar load failures, fallback to initial avatar.
    }
  }

  async function ensureBrandAvatarCached() {
    if (avatarDataUrlCache.value[BRAND_AVATAR_KEY]) return;
    try {
      const result = await invokeTauri<{ dataUrl: string }>("read_avatar_data_url", {
        input: { path: "" },
      });
      avatarDataUrlCache.value = {
        ...avatarDataUrlCache.value,
        [BRAND_AVATAR_KEY]: result.dataUrl || "",
      };
    } catch {
      // ignore brand avatar load failures.
    }
  }

  async function preloadPersonaAvatars() {
    const tasks: Promise<void>[] = [];
    for (const p of options.personas.value) {
      const isUserPersona = !!p.isBuiltInUser || p.id === "user-persona";
      if (isUserPersona) {
        if (p.avatarPath) tasks.push(ensureAvatarCached(p.avatarPath, p.avatarUpdatedAt));
        continue;
      }
      if (p.avatarPath) {
        tasks.push(ensureAvatarCached(p.avatarPath, p.avatarUpdatedAt));
      } else {
        tasks.push(ensureBrandAvatarCached());
      }
    }
    await Promise.all(tasks);
  }

  return {
    resolveAvatarUrl,
    resolveBrandAvatarUrl,
    ensureAvatarCached,
    ensureBrandAvatarCached,
    preloadPersonaAvatars,
  };
}

