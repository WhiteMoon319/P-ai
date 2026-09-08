<template>
  <div class="flex justify-center">
    <div class="w-full max-w-2xl">
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <span class="text-xs text-base-content/60">形态：</span>
        <button type="button" class="btn btn-xs" :class="isFilled ? 'btn-primary' : 'btn-ghost'" @click="isFilled = !isFilled">{{ isFilled ? '灌满' : '悬浮' }}</button>
        <button type="button" class="btn btn-xs" :class="hasExtra ? 'btn-primary' : 'btn-ghost'" @click="hasExtra = !hasExtra">{{ hasExtra ? '[[abc]def]' : '[[abc]]' }}</button>
        <button type="button" class="btn btn-xs btn-primary" @click="cycleWindowBg">{{ windowBgLabel }}</button>
      </div>
      <p class="mb-2 text-xs text-base-content/40">双层卡 · 底卡包面卡 · 露头在面卡上方 · 窗口底色{{ windowBgLabel }}（100→200→300循环，底卡固定base200）</p>
      <!-- 模拟窗口：矩形容器，只管衬底色 -->
      <div class="p-3 transition-all duration-300 ease-in-out" :class="windowBgClass">
        <DoubleDeckCard :extra-visible="hasExtra" :is-rounded="!isFilled" bg="base-200">
          <template #extra>
            <div class="flex flex-col">
              <div class="flex items-center gap-2 px-2 py-1 text-xs">
                <span class="badge badge-xs badge-primary shrink-0">红豆</span>
                <span class="flex-1 truncate opacity-80">def · 队列消息1</span>
              </div>
              <div class="flex items-center gap-2 px-2 py-1 text-xs">
                <span class="badge badge-xs badge-info shrink-0">任务</span>
                <span class="flex-1 truncate opacity-80">def · 队列消息2</span>
              </div>
            </div>
          </template>
          <template #main>
            <div class="px-2 py-1 text-sm leading-6">abc · 输入正文占位</div>
            <div class="mt-2 flex items-center gap-2 px-2 pb-1 text-xs opacity-60">
              <span>目标</span>
              <span>附件</span>
              <span>录音</span>
              <span class="flex-1" />
              <span>发送</span>
            </div>
          </template>
        </DoubleDeckCard>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import DoubleDeckCard from "./input-panel/DoubleDeckCard.vue";

const isFilled = ref(true);
const hasExtra = ref(false);

const WINDOW_BG_CYCLE = ["base-100", "base-200", "base-300"] as const;
type WindowBg = (typeof WINDOW_BG_CYCLE)[number];
const windowBg = ref<WindowBg>("base-300");

const windowBgLabel = computed(() => {
  if (windowBg.value === "base-100") return "base100";
  if (windowBg.value === "base-200") return "base200";
  return "base300";
});

const windowBgClass = computed(() => {
  if (windowBg.value === "base-100") return "bg-base-100";
  if (windowBg.value === "base-200") return "bg-base-200";
  return "bg-base-300";
});

function cycleWindowBg(): void {
  const idx = WINDOW_BG_CYCLE.indexOf(windowBg.value);
  windowBg.value = WINDOW_BG_CYCLE[(idx + 1) % WINDOW_BG_CYCLE.length];
}
</script>
