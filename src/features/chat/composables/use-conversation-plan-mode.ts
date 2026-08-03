import { computed, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

type ConversationPlanModeSummary = {
  conversationId: string;
  planModeEnabled?: boolean;
};

type UseConversationPlanModeOptions<T extends ConversationPlanModeSummary> = {
  currentConversationId: Ref<string>;
  unarchivedConversations: Ref<T[]>;
};

export function useConversationPlanMode<T extends ConversationPlanModeSummary>(options: UseConversationPlanModeOptions<T>) {
  function getConversationPlanModeEnabledById(conversationId: string): boolean {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return false;
    return !!options.unarchivedConversations.value.find((item) =>
      String(item.conversationId || "").trim() === normalizedConversationId
    )?.planModeEnabled;
  }

  function patchConversationPlanModeInOverview(conversationId: string, planModeEnabled: boolean) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    let changed = false;
    const next = options.unarchivedConversations.value.map((item) => {
      if (String(item.conversationId || "").trim() !== normalizedConversationId) {
        return item;
      }
      if (!!item.planModeEnabled === !!planModeEnabled) {
        return item;
      }
      changed = true;
      return {
        ...item,
        planModeEnabled: !!planModeEnabled,
      };
    });
    if (changed) {
      options.unarchivedConversations.value = next;
    }
  }

  async function setConversationPlanMode(conversationId: string, value: boolean): Promise<boolean> {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return false;
    const nextValue = !!value;
    const previousValue = getConversationPlanModeEnabledById(normalizedConversationId);
    if (previousValue === nextValue) return true;
    patchConversationPlanModeInOverview(normalizedConversationId, nextValue);
    try {
      await invokeTauri<{ conversationId: string; planModeEnabled: boolean }>("conversation.planMode.set", {
        input: {
          conversationId: normalizedConversationId,
          planModeEnabled: nextValue,
        },
      });
      return true;
    } catch (error) {
      if (getConversationPlanModeEnabledById(normalizedConversationId) === nextValue) {
        patchConversationPlanModeInOverview(normalizedConversationId, previousValue);
      }
      console.warn("[计划模式] 切换会话计划模式失败", {
        conversationId: normalizedConversationId,
        nextValue,
        error,
      });
      return false;
    }
  }

  async function setCurrentConversationPlanMode(value: boolean): Promise<boolean> {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) return false;
    return setConversationPlanMode(conversationId, value);
  }

  const currentConversationPlanModeEnabled = computed(() => {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) return false;
    return getConversationPlanModeEnabledById(conversationId);
  });

  return {
    currentConversationPlanModeEnabled,
    setConversationPlanMode,
    setCurrentConversationPlanMode,
    updatePlanModeEnabled: setCurrentConversationPlanMode,
  };
}
