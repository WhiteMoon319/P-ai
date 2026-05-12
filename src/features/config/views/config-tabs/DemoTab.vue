<template>
  <div class="grid gap-3">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">{{ t("config.demo.nativeNotificationTitle") }}</h3>
          <p class="text-sm text-base-content/70">
            {{ t("config.demo.nativeNotificationSummary") }}
          </p>
          <p class="text-xs text-base-content/60">
            {{ t("config.demo.nativeNotificationDevHint") }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="sending"
            @click="sendNativeNotification"
          >
            {{ sending ? t("config.demo.sending") : t("config.demo.sendNativeNotification") }}
          </button>
          <span class="text-xs text-base-content/60">{{ t("config.demo.backgroundHint") }}</span>
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
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();

async function sendNativeNotification() {
  sending.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    const result = await invokeTauri<NativeNotificationDemoResult>("demo_send_native_notification");
    resultText.value = [
      t("config.demo.nativeNotificationSent"),
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
