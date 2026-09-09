<script setup lang="ts">
// 动态数字更新器：props.target 是真实目标值，显示值用 rAF 追赶。
// 目标值变化只替换目标、不清零进度，流式高频更新下数字持续增长而不是停在原地。
// 只 re-render 本组件，不拖累消息主体。
import { onBeforeUnmount, ref, watch } from "vue";

const props = defineProps<{ target: number }>();

// 时间常数：每帧按剩余差距的固定比例推进，约 3 个时间常数内收敛到目标。
const CHASE_TIME_MS = 180;
const SNAP_EPSILON = 0.5;
const MAX_FRAME_MS = 64;

const display = ref(props.target);
let rafId = 0;
let lastTs = 0;

function stopRaf() {
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
  lastTs = 0;
}

function tick(ts: number) {
  rafId = 0;
  const elapsed = lastTs > 0 ? Math.min(MAX_FRAME_MS, ts - lastTs) : MAX_FRAME_MS / 4;
  lastTs = ts;
  const diff = props.target - display.value;
  if (Math.abs(diff) <= SNAP_EPSILON) {
    display.value = props.target;
    stopRaf();
    return;
  }
  display.value += diff * (1 - Math.exp(-elapsed / CHASE_TIME_MS));
  rafId = requestAnimationFrame(tick);
}

watch(
  () => props.target,
  (next) => {
    if (next === display.value) return;
    if (!rafId) {
      lastTs = 0;
      rafId = requestAnimationFrame(tick);
    }
  },
);

onBeforeUnmount(stopRaf);
</script>

<template>
  <span v-if="display > 0" class="tabular-nums">({{ Math.round(display).toLocaleString("zh-CN") }})</span>
</template>
