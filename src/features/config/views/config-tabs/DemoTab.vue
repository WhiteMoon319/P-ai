<template>
  <div class="grid gap-3">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">Windows 原生通知测试</h3>
          <p class="text-sm text-base-content/70">
            点击下面按钮后，会通过 Tauri 后端触发一次系统原生通知，方便先验证基础能力。
          </p>
          <p class="text-xs text-base-content/60">
            说明：开发模式下，Windows 可能显示 PowerShell 名称或默认图标，这不代表正式安装版最终效果。
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="sending"
            @click="sendNativeNotification"
          >
            {{ sending ? "发送中..." : "弹一个原生通知" }}
          </button>
          <span class="text-xs text-base-content/60">建议先把应用最小化或切到后台再测试。</span>
        </div>

        <div v-if="errorText" class="alert alert-error text-sm">
          <span>{{ errorText }}</span>
        </div>

        <div v-else-if="resultText" class="alert alert-success text-sm whitespace-pre-wrap">
          <span>{{ resultText }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { invokeTauri } from "../../../../services/tauri-api";

type NativeNotificationDemoResult = {
  permissionBefore: string;
  permissionAfter: string;
  title: string;
  body: string;
  sentAt: string;
};

const sending = ref(false);
const errorText = ref("");
const resultText = ref("");

async function sendNativeNotification() {
  sending.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    const result = await invokeTauri<NativeNotificationDemoResult>("demo_send_native_notification");
    resultText.value = [
      `已尝试发送原生通知。`,
      `title: ${result.title}`,
      `permissionBefore: ${result.permissionBefore}`,
      `permissionAfter: ${result.permissionAfter}`,
      `sentAt: ${result.sentAt}`,
    ].join("\n");
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
  } finally {
    sending.value = false;
  }
}
</script>
