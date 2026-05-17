<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100 text-base-content">
    <header class="flex h-10 shrink-0 items-center gap-1 border-b border-base-300 px-2">
      <button
        v-if="view === 'chat'"
        class="btn btn-ghost btn-sm h-8 min-h-8 min-w-0 flex-1 justify-start gap-1.5 px-2 normal-case"
        title="返回会话列表"
        @click="$emit('showList')"
      >
        <ChevronLeft class="h-4 w-4 shrink-0" />
        <span class="truncate">{{ activeTitle || "PAI" }}</span>
      </button>
      <div v-else class="min-w-0 flex-1 truncate px-2 text-xs font-medium">会话列表</div>
      <button
        v-if="view === 'chat'"
        class="btn btn-ghost btn-xs btn-square"
        title="压缩当前会话"
        :disabled="compacting || !activeConversationId"
        @click="$emit('compactConversation')"
      >
        <span v-if="compacting" class="loading loading-spinner loading-xs"></span>
        <FoldVertical v-else class="h-4 w-4" />
      </button>
      <div
        v-if="view === 'chat' && chatUsagePercent > 0"
        class="inline-flex h-6 w-6 items-center justify-center text-base-content/60"
        :title="`上下文用量 ${chatUsagePercent}%`"
      >
        <svg class="h-4 w-4 -rotate-90" viewBox="0 0 36 36">
          <circle cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" class="opacity-20" />
          <circle cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round"
            :stroke-dasharray="usageRingCircumference" :stroke-dashoffset="usageRingOffset" />
        </svg>
      </div>
      <button class="btn btn-ghost btn-xs btn-square" title="新建会话" @click="$emit('newConversation')">
        <Plus class="h-4 w-4" />
      </button>
      <button class="btn btn-ghost btn-xs btn-square" title="设置" @click="$emit('openSettings')">
        <Settings class="h-4 w-4" />
      </button>
    </header>
    <div v-if="!connected" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
      <div class="text-sm font-medium">{{ connecting ? "正在连接 PAI" : (errorText || "PAI 未运行") }}</div>
      <button class="btn btn-sm btn-primary" :disabled="connecting" @click="$emit('reconnect')">
        <RefreshCcw class="h-4 w-4" />
        重连
      </button>
    </div>
    <main v-else class="min-h-0 flex-1">
      <slot />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, FoldVertical, Plus, RefreshCcw, Settings } from "lucide-vue-next";

const props = defineProps<{
  view: "list" | "chat";
  connected: boolean;
  connecting: boolean;
  errorText: string;
  activeTitle: string;
  activeConversationId: string;
  compacting: boolean;
  chatUsagePercent?: number;
}>();

const usageRingCircumference = 2 * Math.PI * 14;
const usageRingOffset = computed(() => {
  const percent = Math.min(100, Math.max(0, Number(props.chatUsagePercent || 0)));
  return usageRingCircumference * (1 - percent / 100);
});

defineEmits<{
  showList: [];
  showChat: [];
  newConversation: [];
  openSettings: [];
  compactConversation: [];
  reconnect: [];
}>();
</script>
