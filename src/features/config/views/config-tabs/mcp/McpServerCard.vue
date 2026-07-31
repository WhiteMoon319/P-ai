<template>
  <div>
    <div class="space-y-3">
      <div class="flex items-center gap-2">
        <button class="btn btn-sm bg-base-100" type="button" :disabled="disabled" @click="emitValidate">{{ t('config.mcpServerCard.validate') }}</button>
        <button class="btn btn-sm btn-ghost" type="button" :disabled="disabled" @click="emitFix">{{ t('config.mcp.fixFormat') }}</button>
        <button
          class="btn btn-sm"
          :class="draft.enabled ? 'btn-warning' : 'btn-success'"
          type="button"
          :disabled="disabled"
          @click="emitToggleDeploy"
        >
          {{ draft.enabled ? t('config.mcpServerCard.stop') : t('config.mcpServerCard.deploy') }}
        </button>
        <div class="flex-1 rounded-md border border-base-300 bg-base-100 px-3 py-1.5 text-sm leading-5">
          {{ draft.name || t('config.mcpServerCard.displayNamePlaceholder') }}
        </div>
        <button class="btn btn-sm btn-warning" type="button" :disabled="disabled" @click="$emit('remove', draft.id)">
          <Trash2 class="h-4 w-4" />
          {{ t('config.mcpServerCard.delete') }}
        </button>
      </div>

      <div v-if="members.length > 0" class="rounded-md border border-base-300 bg-base-100 px-3 py-2 text-xs">
        <div class="mb-1 flex items-center justify-between">
          <span class="font-semibold opacity-70">组内成员（{{ members.length }}）</span>
        </div>
        <div v-for="m in members" :key="m.name" class="flex items-center justify-between gap-2 py-0.5">
          <span class="font-mono truncate">{{ m.name }}</span>
          <span class="flex items-center gap-2 shrink-0">
            <span v-if="m.toolCount > 0" class="opacity-60">{{ m.toolCount }} 个工具</span>
            <span class="badge badge-sm badge-ghost">{{ m.transport }}</span>
          </span>
        </div>
      </div>

      <div class="collapse collapse-arrow bg-base-100 border-base-300 border">
        <input type="checkbox" />
        <div class="collapse-title font-semibold">{{ t('config.mcpServerCard.configJson') }}</div>
        <div class="collapse-content">
          <textarea
            v-model="draft.definitionJson"
            class="textarea textarea-sm font-mono min-h-40 w-full bg-base-100"
            :placeholder="t('config.mcpServerCard.configPlaceholder')"
            @input="emitChange"
          ></textarea>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2 text-xs">
          <span class="opacity-70">{{ t('config.mcpServerCard.status') }}</span>
          <span v-if="draft.lastStatus === 'ready' || draft.lastStatus === 'deployed'" class="badge badge-sm badge-success">已就绪</span>
          <span v-else-if="draft.lastStatus === 'stopped'" class="badge badge-sm badge-neutral">已停止</span>
          <span v-else-if="draft.lastStatus === 'starting' || draft.lastStatus === 'deploying'" class="badge badge-sm badge-warning">后台启动中</span>
          <span v-else-if="draft.lastStatus === 'stale'" class="badge badge-sm badge-warning">使用旧缓存</span>
          <span v-else-if="draft.lastStatus === 'timeout'" class="badge badge-sm badge-error">超时</span>
          <span v-else-if="draft.lastStatus === 'disabled'" class="badge badge-sm badge-neutral">未启用</span>
          <span v-else-if="draft.lastStatus === 'failed'" class="badge badge-sm badge-error">失败</span>
          <span v-else class="badge badge-sm badge-ghost">{{ draft.lastStatus || "-" }}</span>
          <span v-if="draft.lastError" class="text-error truncate max-w-50" :title="draft.lastError"> | {{ draft.lastError }}</span>
        </div>
        <div></div>
      </div>

      <McpToolList
        :tools="draft.toolItems"
        :elapsed-ms="draft.lastElapsedMs"
        :disabled="disabled"
        @toggle-tool="(payload) => $emit('toggleTool', { serverId: draft.id, ...payload })"
        @refresh-tools="$emit('refreshTools', draft.id)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Trash2 } from "@lucide/vue";
import type { McpServerConfig, McpToolDescriptor } from "../../../../../types/app";
import McpToolList from "./McpToolList.vue";

const { t } = useI18n();

type McpServerView = McpServerConfig & {
  toolItems: McpToolDescriptor[];
  lastElapsedMs: number;
  isDraft: boolean;
  isDirty: boolean;
};

type McpMemberView = {
  name: string;
  transport: string;
  toolCount: number;
};

const props = defineProps<{
  server: McpServerView;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "change", server: McpServerView): void;
  (e: "remove", serverId: string): void;
  (e: "validate", server: McpServerView): void;
  (e: "fix", server: McpServerView): void;
  (e: "toggleDeploy", server: McpServerView): void;
  (e: "toggleTool", payload: { serverId: string; toolName: string; enabled: boolean }): void;
  (e: "refreshTools", serverId: string): void;
}>();

const draft = reactive<McpServerView>({ ...props.server });

watch(
  () => props.server,
  (next) => {
    Object.assign(draft, next);
  },
  { deep: true },
);

function inferTransport(obj: Record<string, unknown>): string {
  const transport = String(obj.transport ?? obj.type ?? "").toLowerCase();
  if (transport === "sse") return "sse";
  if (obj.command) return "stdio";
  if (obj.url) return "streamable_http";
  return "-";
}

const members = computed<McpMemberView[]>(() => {
  const list: McpMemberView[] = [];
  try {
    const parsed = JSON.parse(draft.definitionJson) as unknown;
    const push = (name: string, obj: Record<string, unknown>) => {
      list.push({ name, transport: inferTransport(obj), toolCount: 0 });
    };
    if (Array.isArray(parsed)) {
      for (const item of parsed) {
        if (item && typeof item === "object") {
          push(String((item as Record<string, unknown>).name ?? "(未命名)"), item as Record<string, unknown>);
        }
      }
    } else if (parsed && typeof parsed === "object") {
      const root = parsed as Record<string, unknown>;
      const mcpServers = root.mcpServers;
      if (Array.isArray(mcpServers)) {
        for (const item of mcpServers) {
          if (item && typeof item === "object") {
            push(String((item as Record<string, unknown>).name ?? "(未命名)"), item as Record<string, unknown>);
          }
        }
      } else if (mcpServers && typeof mcpServers === "object") {
        for (const [name, obj] of Object.entries(mcpServers as Record<string, unknown>)) {
          if (obj && typeof obj === "object") push(name, obj as Record<string, unknown>);
        }
      } else {
        const hasDirectField = ["command", "url", "transport", "type", "args", "env", "cwd", "headers", "httpHeaders", "envHttpHeaders", "bearerTokenEnvVar", "enabledTools", "disabledTools"].some(
          (key) => key in root,
        );
        if (hasDirectField) {
          push(String(root.name ?? "(未命名)"), root);
        } else {
          for (const [name, obj] of Object.entries(root)) {
            if (obj && typeof obj === "object") push(name, obj as Record<string, unknown>);
          }
        }
      }
    }
  } catch {
    // JSON 未解析时保持空列表
  }
  // 工具数按前缀归组（toolName 形如 {成员名}_{工具名}）
  for (const tool of draft.toolItems) {
    const idx = tool.toolName.lastIndexOf("_");
    if (idx <= 0) continue;
    const memberName = tool.toolName.slice(0, idx);
    const member = list.find((m) => m.name === memberName);
    if (member) member.toolCount += 1;
  }
  return list;
});

function emitChange() {
  emit("change", { ...draft });
}

function emitValidate() {
  emit("validate", { ...draft });
}

function emitFix() {
  emit("fix", { ...draft });
}

function emitToggleDeploy() {
  emit("toggleDeploy", { ...draft });
}
</script>
