import { afterEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";
import { useChatMessageActions } from "../composables/use-chat-message-actions";
import { useChatSelection } from "../composables/use-chat-selection";
import { prepareShareEntries } from "./share-export";

function chatBlock(overrides: Partial<ChatMessageBlock> = {}): ChatMessageBlock {
  return {
    id: "block-1",
    role: "assistant",
    text: "正文 [toolcall:call-1]",
    images: [],
    audios: [],
    attachmentFiles: [],
    toolCallCount: 0,
    lastToolName: "",
    toolCalls: [],
    activityItems: [],
    activityReasoningCharCount: 0,
    activityToolCountsByName: {},
    activityRunning: false,
    activityStatus: "idle",
    ...overrides,
  };
}

describe("copy/share toolcall marker filtering", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("filters toolcall markers when copying a single message", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const { copyMessage } = useChatMessageActions();

    await copyMessage(chatBlock({ text: "可读正文 [toolcall:call-abc]" }));

    expect(writeText).toHaveBeenCalledWith("可读正文");
  });

  it("does not write clipboard for marker-only single message copy", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const { copyMessage } = useChatMessageActions();

    await copyMessage(chatBlock({ text: "[toolcall:call-only]" }));

    expect(writeText).not.toHaveBeenCalled();
  });

  it("filters toolcall markers when copying selected messages", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const block = chatBlock({
      id: "selected-1",
      text: "多选正文 [toolcall:call-selected]",
    });
    const { copySelectedMessages } = useChatSelection({
      chatRenderItems: ref([{ renderId: "render-1", block }]),
      messageSelectionModeEnabled: ref(true),
      selectedMessageRenderIds: ref(["render-1"]),
      personaNameMap: {},
      userAlias: "我",
      t: (key, params) => {
        if (key === "archives.roleUser") return "用户";
        if (key === "chat.imageCount") return `${params?.count} 张图片`;
        if (key === "chat.audioCount") return `${params?.count} 条音频`;
        if (key === "chat.attachmentList") return String(params?.names || "");
        return key;
      },
      onEmit: {
        selectionActionCopy: vi.fn(),
        selectionActionCopyError: vi.fn(),
        selectionActionBranch: vi.fn(),
        selectionActionForward: vi.fn(),
        selectionActionDelegate: vi.fn(),
        selectionActionShare: vi.fn(),
      },
    });

    await copySelectedMessages();

    expect(writeText).toHaveBeenCalledWith("[我]: 多选正文");
  });

  it("filters toolcall markers from share entry text while keeping tool summaries", async () => {
    const entries = await prepareShareEntries({
      blocks: [
        chatBlock({
          text: "分享正文 [toolcall:call-share]",
          toolCalls: [{ toolCallId: "call-share", name: "read_file", argsText: "{}", status: "done" }],
        }),
      ],
      userAlias: "我",
      userAvatarUrl: "",
      personaNameMap: {},
      personaAvatarUrlMap: {},
      trigger: "test",
    });

    expect(entries[0]?.text).toBe("分享正文");
    expect(entries[0]?.toolCalls).toEqual([{ name: "read_file", argsText: "{}", status: "done" }]);
  });
});
