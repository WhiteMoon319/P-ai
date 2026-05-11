import { ref, watch, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";

const USER_KEY = "easy_call.user_bubble_background_hidden.v1";
const ASSISTANT_KEY = "easy_call.assistant_bubble_background_hidden.v1";

function readBool(key: string): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(key) === "1";
}

function writeBool(key: string, value: boolean) {
  if (typeof window === "undefined") return;
  if (value) window.localStorage.setItem(key, "1");
  else window.localStorage.removeItem(key);
}

export function useBubbleBackground(activeConversationId: Ref<string>) {
  const userHidden = ref(false);
  const assistantHidden = ref(false);

  function load() {
    userHidden.value = readBool(USER_KEY);
    assistantHidden.value = readBool(ASSISTANT_KEY);
  }

  watch(activeConversationId, () => load(), { immediate: true });

  function canToggle(block: ChatMessageBlock): boolean {
    if (block.isStreaming || block.dividerKind || block.isExtraTextBlock) return false;
    const role = String(block.role || "").trim();
    return role === "user" || role === "assistant";
  }

  function isHidden(block: ChatMessageBlock): boolean {
    const role = String(block.role || "").trim();
    if (role === "user") return userHidden.value;
    if (role === "assistant") return assistantHidden.value;
    return false;
  }

  function toggle(block: ChatMessageBlock) {
    if (!canToggle(block)) return;
    const role = String(block.role || "").trim();
    if (role === "user") {
      userHidden.value = !userHidden.value;
      writeBool(USER_KEY, userHidden.value);
    } else if (role === "assistant") {
      assistantHidden.value = !assistantHidden.value;
      writeBool(ASSISTANT_KEY, assistantHidden.value);
    }
  }

  return { isHidden, canToggle, toggle };
}
