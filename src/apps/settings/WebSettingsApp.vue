<template>
  <div v-if="ready" class="h-full">
    <ConfigWindowApp />
  </div>
  <div v-else class="flex min-h-screen items-center justify-center bg-base-200 px-4 text-base-content">
    <div class="w-full max-w-sm rounded-box border border-base-300 bg-base-100 p-5 shadow-xl">
      <div class="text-base font-semibold">P-ai 设置</div>
      <div class="mt-2 text-sm text-base-content/70">{{ statusText }}</div>
      <button class="btn btn-sm btn-primary mt-4 w-full" type="button" :disabled="connecting" @click="initialize">
        <span v-if="connecting" class="loading loading-spinner loading-xs"></span>
        重试连接
      </button>
      <div v-if="errorText" class="mt-3 text-xs text-error">{{ errorText }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import ConfigWindowApp from "../../ConfigWindowApp.vue";
import { ensureTransportReady, getTransportConnectionState } from "../../services/tauri-api";

const ready = ref(false);
const connecting = ref(false);
const errorText = ref("");
const statusText = ref("正在连接 PAI...");

function applyTransportState() {
  const state = getTransportConnectionState();
  if (state.connected) {
    statusText.value = "连接成功，正在加载设置...";
  } else {
    statusText.value = state.errorText || "PAI 未运行。";
  }
}

async function initialize() {
  if (ready.value) return;
  connecting.value = true;
  errorText.value = "";
  statusText.value = "正在连接 PAI...";
  try {
    const state = await ensureTransportReady();
    applyTransportState();
    ready.value = state.ready;
  } catch (error) {
    applyTransportState();
    errorText.value = String(error || "连接失败");
  } finally {
    connecting.value = false;
  }
}

onMounted(() => {
  void initialize();
});
</script>
