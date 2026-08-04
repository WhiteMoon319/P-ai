<template>
  <div class="flex flex-col gap-4 p-4">
    <div class="text-sm text-base-content/70">
      {{ t("config.remoteFrontend.summary") }}
    </div>

    <div
      v-if="connected"
      class="alert alert-success flex items-center justify-between gap-2"
    >
      <span class="min-w-0 text-sm">
        {{ t("config.remoteFrontend.connectedTo", { target: remoteTargetText }) }}
      </span>
      <button class="btn btn-sm shrink-0" @click="disconnect">
        {{ t("config.remoteFrontend.disconnect") }}
      </button>
    </div>

    <div class="grid gap-2.5">
      <label class="grid gap-1">
        <span class="text-sm">{{ t("config.remoteFrontend.host") }}</span>
        <input
          v-model="host"
          class="input input-bordered input-sm w-full font-mono"
          :placeholder="t('config.remoteFrontend.hostPlaceholder')"
          @keydown.enter="connect"
        />
      </label>
      <label class="grid gap-1">
        <span class="text-sm">{{ t("config.remoteFrontend.port") }}</span>
        <input
          v-model="port"
          class="input input-bordered input-sm w-full font-mono"
          type="number"
          min="1"
          max="65535"
          :placeholder="String(DEFAULT_REMOTE_PORT)"
          @keydown.enter="connect"
        />
      </label>
      <label class="grid gap-1">
        <span class="text-sm">{{ t("config.remoteFrontend.password") }}</span>
        <input
          v-model="password"
          class="input input-bordered input-sm w-full font-mono"
          type="password"
          autocomplete="off"
          :placeholder="t('config.remoteFrontend.passwordPlaceholder')"
          @keydown.enter="connect"
        />
      </label>
      <button class="btn btn-primary btn-sm w-fit" :disabled="!canConnect" @click="connect">
        {{ t("config.remoteFrontend.connect") }}
      </button>
      <div v-if="errorText" class="text-xs text-error">{{ errorText }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  DEFAULT_REMOTE_PORT,
  parseRemoteTargetInput,
  useRemoteMode,
} from "../../../shell/composables/use-remote-mode";

const { t } = useI18n();
const { remoteActive, remoteTarget, remoteTargetText, enterRemote, exitRemote } = useRemoteMode();

const host = ref("");
const port = ref("");
const password = ref("");
const errorText = ref("");

const connected = computed(() => remoteActive.value && !!remoteTarget.value);
const canConnect = computed(() => String(host.value || "").trim() !== "" && String(port.value || "").trim() !== "");

onMounted(() => {
  // 已保存过目标时回填表单，方便快速重连。
  if (remoteTarget.value) {
    host.value = remoteTarget.value.host;
    port.value = String(remoteTarget.value.port);
    password.value = remoteTarget.value.password || "";
  }
});

function connect() {
  const target = parseRemoteTargetInput(host.value, port.value);
  if (!target) {
    errorText.value = t("config.remoteFrontend.invalidTarget");
    return;
  }
  errorText.value = "";
  const passwordText = String(password.value || "").trim();
  enterRemote({
    ...target,
    ...(passwordText ? { password: passwordText } : {}),
  });
  // Android 单 WebView：settings 页与 chat 页同目录，相对导航回 chat 进入远程模式。
  window.location.href = "chat.html";
}

function disconnect() {
  exitRemote();
}
</script>
