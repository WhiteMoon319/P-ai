<template>
  <div
    class="ecall-home-card flex min-w-0 flex-col gap-2 overflow-hidden rounded-box bg-base-100 p-2.5 shadow transition-colors duration-200"
    :class="[
      variant === 'wide' ? 'ecall-home-card-wide' : 'ecall-home-card-small',
      layout === 'tile' ? 'ecall-home-card-tile' : '',
      interactive
        ? 'cursor-pointer hover:bg-base-200 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-primary/50'
        : '',
    ]"
    :role="interactive ? 'button' : undefined"
    :tabindex="interactive ? 0 : undefined"
    @click="handleSelect"
    @keydown.enter.prevent="handleSelect"
    @keydown.space.prevent="handleSelect"
  >
    <template v-if="layout === 'tile'">
      <span class="ecall-home-card-icon ecall-home-card-icon-tile" :class="toneClass">
        <component :is="icon" class="size-5" aria-hidden="true" />
      </span>
      <span class="min-w-0 max-w-full text-sm font-medium text-base-content/85">{{ label }}</span>
      <slot />
    </template>
    <template v-else>
      <div class="flex min-w-0 shrink-0 items-center gap-2">
        <span class="ecall-home-card-icon" :class="toneClass">
          <component :is="icon" class="size-3.5" aria-hidden="true" />
        </span>
        <span class="min-w-0 flex-1 truncate text-xs font-medium text-base-content/70">{{ label }}</span>
        <slot name="trailing" />
      </div>
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <slot />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Component } from "vue";

const props = withDefaults(defineProps<{
  label: string;
  icon: Component;
  tone?: "primary" | "secondary" | "info" | "success" | "warning" | "neutral";
  /** wide 为大卡（横跨 2 列、占 1 行），small 为小卡（方形，占 1 行 1 列） */
  variant?: "small" | "wide";
  /** row 为带标题行的内容卡；tile 为入口磁贴（图标与标题居中，不出现标题行） */
  layout?: "row" | "tile";
  interactive?: boolean;
}>(), {
  tone: "neutral",
  variant: "small",
  layout: "row",
  interactive: false,
});

const emit = defineEmits<{
  (e: "select"): void;
}>();

const TONE_CLASS: Record<string, string> = {
  primary: "bg-primary/15 text-primary",
  secondary: "bg-secondary/15 text-secondary",
  info: "bg-info/15 text-info",
  success: "bg-success/15 text-success",
  warning: "bg-warning/18 text-warning",
  neutral: "bg-base-content/10 text-base-content/70",
};

const toneClass = computed(() => TONE_CLASS[props.tone] || TONE_CLASS.neutral);

function handleSelect() {
  if (!props.interactive) return;
  emit("select");
}
</script>

<style scoped>
/* 小卡：固定 1×1 方形；大卡：横跨两格长条，窄面板时收窄到整行 */
.ecall-home-card-small {
  width: var(--ecall-home-tile, 8.75rem);
  max-width: 100%;
  height: var(--ecall-home-tile, 8.75rem);
  flex: 0 0 auto;
}

.ecall-home-card-wide {
  width: calc(var(--ecall-home-tile, 8.75rem) * 2 + 0.625rem);
  max-width: 100%;
  height: var(--ecall-home-tile, 8.75rem);
  flex: 0 0 auto;
}

.ecall-home-card-icon {
  display: grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  flex-shrink: 0;
  border-radius: 0.5rem;
}

/* 入口磁贴：整卡居中，图标与标题竖排，不与内容卡的标题行同构 */
.ecall-home-card-tile {
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 0.5rem;
}

.ecall-home-card-icon-tile {
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 0.75rem;
}
</style>
