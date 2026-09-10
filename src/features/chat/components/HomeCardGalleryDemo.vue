<template>
  <div class="space-y-5">
    <p class="text-sm text-base-content/70">
      每张卡单独放进首页流式布局里；卡片尺寸固定（小卡 140px 见方、大卡横跨两格），面板变宽只会多排几张，不会把卡片拉大。模拟数据只为看样式，点击不产生副作用。
    </p>

    <section v-for="section in sections" :key="section.key" class="space-y-2">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-sm font-medium">{{ section.title }}</span>
        <span class="text-xs text-base-content/50">{{ section.hint }}</span>
      </div>
      <div class="ecall-gallery-panel" :style="{ height: section.height }">
        <ChatHomePanel v-bind="section.data" />
      </div>
    </section>

    <section class="space-y-2">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-sm font-medium">全部卡片</span>
        <span class="text-xs text-base-content/50">真实首页：大卡横跨整行，小卡按方形两两排开</span>
      </div>
      <div class="ecall-gallery-panel" style="height: 680px">
        <ChatHomePanel v-bind="allCards" />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import type { BackgroundShellTaskSummary, ConversationDelegateStatusSummary } from "../../../types/app";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import type { ToolReviewBatchSummary } from "../composables/use-chat-tool-review";
import ChatHomePanel from "./ChatHomePanel.vue";

const MOCK_WORKSPACE = "E:/github/easy_call_ai";

const MOCK_GIT = {
  workspaceRootPath: MOCK_WORKSPACE,
  branch: "main",
  changes: [
    { path: "src/features/chat/components/ChatHomePanel.vue", status: "M" },
    { path: "src/features/chat/components/chat-home/CardShell.vue", status: "M" },
    { path: "src/features/chat/components/chat-home/HomeWorkspaceCard.vue", status: "A" },
    { path: "src/locales/zh-TW.json", status: "M" },
    { path: "src/features/config/views/config-tabs/OldDemoTab.vue", status: "D" },
    { path: "src/features/chat/views/ChatView.vue", status: "R" },
  ],
  changeCount: 9,
};

const MOCK_FILES = {
  openFiles: [
    { path: `${MOCK_WORKSPACE}/src/features/chat/views/ChatView.vue`, label: "ChatView.vue" },
    { path: `${MOCK_WORKSPACE}/src/features/chat/components/ChatHomePanel.vue`, label: "ChatHomePanel.vue" },
    { path: `${MOCK_WORKSPACE}/src/features/chat/components/chat-home/CardShell.vue`, label: "CardShell.vue" },
    { path: `${MOCK_WORKSPACE}/src/locales/zh-CN.json`, label: "zh-CN.json" },
    { path: `${MOCK_WORKSPACE}/src/features/chat/composables/chat-ui-layout-storage.ts`, label: "chat-ui-layout-storage.ts" },
  ],
  activePath: `${MOCK_WORKSPACE}/src/features/chat/components/ChatHomePanel.vue`,
  openFileCount: 6,
};

const MOCK_SIDE_CHATS = {
  sideChats: [{ id: "demo-side-1", title: "压缩重开后收尾标识为什么会对不上？" }],
};

const MOCK_SHELLS = {
  shells: [makeShell(0, "ready in 1243 ms\nLocal: http://localhost:1420/")],
};

const MOCK_DELEGATES = {
  delegates: [makeDelegate(0, "running")],
};

const MOCK_TASKS = {
  runningTasks: [makeTask(0)],
};

const MOCK_SIDE_CHAT_ENABLED = { sideChatEnabled: true };

// 第 2 条 itemCount 为 0：用来确认「没有工具调用的轮次」不会进卡片。
// 卡片只看最后一条（最新一轮）——它改了 6 个文件，所以头部是「修改了 6 个文件」，卡内 4 行文件 + 「还有 2 个文件被修改」。
const MOCK_TOOL_BATCHES = {
  toolBatches: [
    makeBatch(1, 6, 4, 96, 128, [
      `${MOCK_WORKSPACE}/src/features/chat/components/ChatHomePanel.vue`,
      `${MOCK_WORKSPACE}/src/features/chat/components/chat-home/CardShell.vue`,
      `${MOCK_WORKSPACE}/src/features/chat/composables/use-chat-tool-review.ts`,
      `${MOCK_WORKSPACE}/src/locales/zh-CN.json`,
    ]),
    makeBatch(2, 0, 0, 0, 0, []),
    makeBatch(3, 3, 2, 41, 6, [
      `${MOCK_WORKSPACE}/src/features/config/views/config-tabs/DemoTab.vue`,
      `${MOCK_WORKSPACE}/src/features/chat/components/HomeCardGalleryDemo.vue`,
    ]),
    makeBatch(4, 9, 1, 18, 3, [`${MOCK_WORKSPACE}/src/features/shell/components/AppWindowContent.vue`]),
    makeBatch(5, 2, 6, 96, 12, [
      `${MOCK_WORKSPACE}/src/features/chat/components/chat-home/HomeToolCard.vue`,
      `${MOCK_WORKSPACE}/src/features/chat/components/ChatHomePanel.vue`,
      `${MOCK_WORKSPACE}/src/features/chat/views/ChatView.vue`,
      `${MOCK_WORKSPACE}/src/locales/zh-CN.json`,
      `${MOCK_WORKSPACE}/src/locales/zh-TW.json`,
      `${MOCK_WORKSPACE}/src/locales/en-US.json`,
    ]),
  ],
};

function makeBatch(
  index: number,
  itemCount: number,
  changedFiles: number,
  addedLines: number,
  deletedLines: number,
  affectedPaths: string[],
): ToolReviewBatchSummary {
  return {
    batchKey: `demo-batch-${index}`,
    userMessageId: `demo-message-${index}`,
    userMessageText: `演示用第 ${index} 轮用户消息`,
    itemCount,
    unreviewedCount: itemCount,
    changedFiles,
    addedLines,
    deletedLines,
    items: affectedPaths.map((path, pathIndex) => ({
      callId: `demo-call-${index}-${pathIndex}`,
      toolName: "apply_patch",
      orderIndex: pathIndex,
      hasReview: false,
      affectedPaths: [path],
      patchOperation: "update",
      isSuccess: true,
      addedLines: 0,
      deletedLines: 0,
    })),
  };
}

function makeShell(index: number, outputTail: string): BackgroundShellTaskSummary {
  return {
    id: `demo-shell-${index}`,
    kind: "shell",
    status: "running",
    exitCode: null,
    description: "后台启动 vite dev server 用于本地视觉自检",
    command: "pnpm dev",
    cwd: MOCK_WORKSPACE,
    startedAt: new Date(Date.now() - 154000).toISOString(),
    timeoutMs: null,
    log: "C:/temp/bg-shell-demo.log",
    outputTail,
  };
}

function makeDelegate(index: number, status: string): ConversationDelegateStatusSummary {
  return {
    delegateId: `demo-delegate-${index}`,
    kind: "delegate",
    conversationId: "demo",
    rootConversationId: "demo",
    title: "排查后台任务页加载失败并补齐回归测试",
    status,
    active: true,
    startedAt: new Date(Date.now() - 62000).toISOString(),
    updatedAt: new Date().toISOString(),
    elapsedMs: 62000,
    requestCount: 12,
    toolCallCount: 34,
    lastToolName: "exec",
    tokenCount: 29000,
  };
}

function makeTask(index: number): TaskEntry {
  return {
    taskId: `demo-task-${index}`,
    orderIndex: index,
    goal: "每周整理一次 changelog 未发布条目",
    why: "保持发布流程可追溯",
    todo: "检查清单并逐项归类",
    completionState: "active",
    completionConclusion: "",
    progressNotes: [],
    trigger: { next_run_at: new Date(Date.now() + 125 * 60000).toISOString() },
    createdAtLocal: new Date().toISOString(),
    updatedAtLocal: new Date().toISOString(),
  };
}

const sections = [
  { key: "git", title: "Git 更改大卡", hint: "横跨 2 列、占 1 行，点击进阅读面板", height: "200px", data: MOCK_GIT },
  { key: "files", title: "已打开文件大卡", hint: "横跨 2 列、占 1 行，行内点击直接打开文件", height: "200px", data: MOCK_FILES },
  { key: "workspace", title: "工作目录小卡", hint: "1×1 方形，进入工作区目录树；有工作区时 Git 大卡会一起出现", height: "340px", data: MOCK_GIT },
  { key: "side-chat-create", title: "新建追问小卡", hint: "1×1 方形，进追问新建页", height: "200px", data: MOCK_SIDE_CHAT_ENABLED },
  { key: "side-chat", title: "追问小卡", hint: "1×1 方形，切到该追问会话", height: "200px", data: MOCK_SIDE_CHATS },
  { key: "shell", title: "后台终端小卡", hint: "1×1 方形，仅展示不可点", height: "200px", data: MOCK_SHELLS },
  { key: "delegate", title: "委托小卡", hint: "1×1 方形，进监控面板委托页", height: "200px", data: MOCK_DELEGATES },
  { key: "task", title: "任务小卡", hint: "1×1 方形，进监控面板任务页", height: "200px", data: MOCK_TASKS },
  { key: "tool", title: "最近工具大卡", hint: "横跨 2 列、占 1 行，只讲最近一轮工具调用；无工具调用时整张不出现", height: "260px", data: MOCK_TOOL_BATCHES },
];

const allCards = {
  ...MOCK_GIT,
  ...MOCK_FILES,
  ...MOCK_SIDE_CHAT_ENABLED,
  sideChats: [
    ...MOCK_SIDE_CHATS.sideChats,
    { id: "demo-side-2", title: "右侧主页网格的列宽是怎么算出来的" },
    { id: "demo-side-3", title: "日志窗滚动条为什么改成常驻" },
  ],
  shells: MOCK_SHELLS.shells,
  delegates: [
    ...MOCK_DELEGATES.delegates,
    makeDelegate(1, "delivered"),
  ],
  runningTasks: [makeTask(0), makeTask(1)],
  toolBatches: MOCK_TOOL_BATCHES.toolBatches,
};
</script>

<style scoped>
.ecall-gallery-panel {
  width: 320px;
  max-width: 100%;
  overflow: hidden;
  border-radius: 0.75rem;
  border: 1px solid var(--color-base-300);
  background-color: var(--color-base-200);
}
</style>
