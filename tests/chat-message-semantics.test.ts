import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../src/types/app";
import {
  appendReasoningDeltaToStreamBlocks,
  appendTextDeltaToStreamBlocks,
  appendReasoningToStreamActivityItems,
  applyAssistantToolEventToStreamBlocks,
  assistantStreamBlocksFromMessageForDisplay,
  inspectUndoablePatchCalls,
  normalizeMessageToolHistoryEvents,
  projectChatActivityForDisplay,
  projectMessageForDisplay,
  projectStreamingChatActivityForDisplay,
  streamBlocksToToolHistoryEvents,
} from "../src/utils/chat-message-semantics";

function textMessage(id: string, role: ChatMessage["role"], text: string): ChatMessage {
  return {
    id,
    role,
    createdAt: "2026-03-28T10:00:00Z",
    parts: [{ type: "text", text }],
  };
}

describe("chat-message semantics", () => {
  it("projects display fields from unified message semantics", () => {
    const message: ChatMessage = {
      ...textMessage("a-1", "assistant", "我查好了"),
      providerMeta: {
        origin: {
          kind: "remote_im",
          sender_name: "张三",
          contact_name: "测试群",
          contact_type: "group",
          channel_id: "remote-im-1",
          contact_id: "group-1",
        },
      },
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先检查上下文",
          tool_calls: [{
            id: "fc_1",
            call_id: "call_1",
            type: "function",
            function: {
              name: "remote_im_send",
              arguments: "{\"text\":\"你好\"}",
            },
          }],
        },
      ],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.text).toBe("我查好了");
    expect(projection.activityItems[0]).toMatchObject({ kind: "reasoning", text: "先检查上下文" });
    expect(projection.toolCallCount).toBe(1);
    expect(projection.lastToolName).toBe("remote_im_send");
    expect(projection.toolCalls[0]).toEqual({
      name: "remote_im_send",
      argsText: "{\"text\":\"你好\"}",
    });
    expect(projection.remoteImOrigin).toEqual({
      senderName: "张三",
      remoteContactName: "测试群",
      remoteContactType: "group",
      channelId: "remote-im-1",
      contactId: "group-1",
    });
  });

  it("drops orphan assistant tool call only for prompt replay view", () => {
    const message: ChatMessage = {
      ...textMessage("a-2", "assistant", "处理中"),
      toolCall: [
        {
          role: "assistant",
          tool_calls: [{
            id: "fc_2",
            type: "function",
            function: {
              name: "bing_search",
              arguments: "{\"query\":\"rust\"}",
            },
          }],
        },
      ],
    };

    expect(normalizeMessageToolHistoryEvents(message, "display")).toHaveLength(1);
    expect(normalizeMessageToolHistoryEvents(message, "prompt")).toHaveLength(0);
  });

  it("uses only event-level reasoning for display activity", () => {
    const message: ChatMessage = {
      ...textMessage("a-4", "assistant", "终端版本是 PowerShell 7.5.4"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先调用终端工具查看 PowerShell 版本。",
          tool_calls: [{
            id: "fc_3",
            type: "function",
            function: {
              name: "shell_command",
              arguments: "{\"command\":\"$PSVersionTable.PSVersion\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "fc_3",
          content: "7.5.4",
        },
      ],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.activityItems).toHaveLength(2);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      text: "先调用终端工具查看 PowerShell 版本。",
    });
  });

  it("projects message-level activity when there are no tool events", () => {
    const message: ChatMessage = {
      ...textMessage("a-activity", "assistant", "不太确定，展开说说？"),
      activityItems: [{
        kind: "reasoning",
        id: "stream-reasoning-0",
        text: "先判断用户提到的工具指代。",
      }],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.text).toBe("不太确定，展开说说？");
    expect(projection.activityItems).toHaveLength(1);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      id: "stream-reasoning-0",
      text: "先判断用户提到的工具指代。",
    });
    expect(projection.activityStatus).toBe("complete");
  });

  it("projects assistant-only activity events when there are no tools", () => {
    const message: ChatMessage = {
      ...textMessage("a-event-activity", "assistant", "不太确定，展开说说？"),
      toolCall: [{
        role: "assistant",
        content: "不太确定，展开说说？",
        reasoning_content: "先判断用户提到的工具指代。",
      }],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.activityItems).toHaveLength(1);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      text: "先判断用户提到的工具指代。",
    });
    expect(projection.toolCallCount).toBe(0);
  });

  it("inspects undoable patch calls through normalized tool history", () => {
    const messages: ChatMessage[] = [
      textMessage("u-1", "user", "请改一下文件"),
      {
        ...textMessage("a-3", "assistant", ""),
        toolCall: [
          {
            role: "assistant",
            tool_calls: [{
              id: "patch_1",
              type: "function",
              function: {
                name: "apply_patch",
                arguments: "*** Begin Patch\n*** Update File: C:/demo.txt\n*** End Patch",
              },
            }],
          },
          {
            role: "tool",
            tool_call_id: "patch_1",
            content: "{\"ok\":true}",
          },
        ],
      },
    ];

    const availability = inspectUndoablePatchCalls(messages, "u-1", {
      isApplyPatchArgsUndoable: (rawArgs) => rawArgs.includes("*** Begin Patch"),
    });

    expect(availability).toEqual({ canUndo: true, hint: "" });
  });

  it("does not create activity items without event-level reasoning or tools", () => {
    const message: ChatMessage = {
      ...textMessage("a-5", "assistant", "直接回答"),
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items).toEqual([]);
    expect(activity.activityReasoningCharCount).toBe(0);
    expect(activity.activityStatus).toBe("idle");
  });

  it("projects ordered reasoning and adjacent tool result activity items", () => {
    const message: ChatMessage = {
      ...textMessage("a-6", "assistant", "完成"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先读文件。",
          tool_calls: [{
            id: "read_1",
            type: "function",
            function: {
              name: "read_file",
              arguments: "{\"path\":\"src/main.ts\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "read_1",
          content: "文件内容",
        },
      ],
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items).toHaveLength(2);
    expect(activity.items[0]).toMatchObject({ kind: "reasoning", text: "先读文件。" });
    expect(activity.items[1]).toMatchObject({
      kind: "tool",
      name: "read_file",
      argsText: "{\"path\":\"src/main.ts\"}",
      resultText: "文件内容",
    });
    expect(activity.activityReasoningCharCount).toBe("先读文件。".length);
    expect(activity.activityToolCountsByName).toEqual({ read_file: 1 });
    expect(activity.activityStatus).toBe("complete");
  });

  it("keeps multi-tool activity order and aggregates counts by name", () => {
    const message: ChatMessage = {
      ...textMessage("a-7", "assistant", "完成"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先列目录。",
          tool_calls: [{
            id: "exec_1",
            type: "function",
            function: { name: "exec", arguments: "{\"command\":\"ls\"}" },
          }],
        },
        { role: "tool", tool_call_id: "exec_1", content: "a.txt" },
        {
          role: "assistant",
          reasoning_content: "再读文件。",
          tool_calls: [{
            id: "read_2",
            type: "function",
            function: { name: "read", arguments: "{\"path\":\"a.txt\"}" },
          }],
        },
        { role: "tool", tool_call_id: "read_2", content: "hello" },
        {
          role: "assistant",
          tool_calls: [{
            id: "exec_3",
            type: "function",
            function: { name: "exec", arguments: "{\"command\":\"pwd\"}" },
          }],
        },
        { role: "tool", tool_call_id: "exec_3", content: "/repo" },
      ],
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先列目录。",
      "exec",
      "再读文件。",
      "read",
      "exec",
    ]);
    expect(activity.activityToolCountsByName).toEqual({ exec: 2, read: 1 });
  });

  it("prioritizes streaming activity status by tool, reasoning, then request", () => {
    expect(projectStreamingChatActivityForDisplay({
      activityItems: [
        { kind: "reasoning", id: "r-1", text: "正在分析" },
        { kind: "tool", id: "t-1", toolCallId: "t-1", name: "exec", argsText: "{}", status: "doing" },
      ],
      running: true,
    }).activityStatus).toBe("running_tool");

    expect(projectStreamingChatActivityForDisplay({
      activityItems: [{ kind: "reasoning", id: "r-1", text: "正在分析" }],
      running: true,
    }).activityStatus).toBe("thinking");

    expect(projectStreamingChatActivityForDisplay({
      running: true,
    }).activityStatus).toBe("requesting");
  });

  it("merges consecutive streaming reasoning into one activity item", () => {
    const first = appendReasoningToStreamActivityItems([], "先检查");
    const second = appendReasoningToStreamActivityItems(first, "目录。");

    expect(second).toEqual([{
      kind: "reasoning",
      id: "stream-reasoning-0",
      text: "先检查目录。",
      running: true,
    }]);
  });

  it("uses event-level streaming activity items before stream tool calls", () => {
    const activity = projectStreamingChatActivityForDisplay({
      toolCalls: [{ toolCallId: "stream-tool", name: "stream", argsText: "{}", status: "done" }],
      activityItems: [
        { kind: "reasoning", id: "r-1", text: "事件思考。" },
        { kind: "tool", id: "tool-1", toolCallId: "tool-1", name: "exec", argsText: "{\"command\":\"pwd\"}", status: "done" },
      ],
      running: true,
    });

    expect(activity.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "事件思考。",
      "exec",
    ]);
    expect(activity.activityToolCountsByName).toEqual({ exec: 1 });
  });

  it("projects streaming blocks through the same activity semantics as persisted tool history", () => {
    const streamBlocks = [{
      reasoning: "先看文件。",
      text: "结论",
      tools: [{
        toolCallId: "tool-1",
        name: "read_file",
        argsText: "{\"path\":\"README.md\"}",
        resultText: "文件内容",
        status: "done" as const,
      }],
    }];

    const streaming = projectStreamingChatActivityForDisplay({
      streamBlocks,
      running: true,
    });
    const persisted = projectMessageForDisplay({
      ...textMessage("a-block", "assistant", "结论"),
      toolCall: streamBlocksToToolHistoryEvents(streamBlocks),
    });

    expect(streaming.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先看文件。",
      "read_file",
    ]);
    expect(persisted.activityItems.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先看文件。",
      "read_file",
    ]);
  });

  it("reconstructs assistant display blocks from persisted tool history", () => {
    const streamBlocks = [{
      text: "先说明我要等待。",
    }, {
      reasoning: "准备调用等待工具。",
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\",\"seconds\":3}",
        resultText: "等待完成",
        status: "done" as const,
      }],
      text: "等待完成，现在汇报。",
    }];

    const message = {
      ...textMessage("a-blocks", "assistant", "先说明我要等待。等待完成，现在汇报。"),
      providerMeta: {
        _streamBlocks: streamBlocks,
      },
      toolCall: streamBlocksToToolHistoryEvents(streamBlocks),
    };

    expect(assistantStreamBlocksFromMessageForDisplay(message, "先说明我要等待。等待完成，现在汇报。")).toEqual([{
      reasoning: "准备调用等待工具。",
      text: "等待完成，现在汇报。",
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\",\"seconds\":3}",
        resultText: "等待完成",
        status: "done",
      }],
    }]);
  });

  it("folds local streaming deltas into ordered assistant stream blocks", () => {
    let blocks = appendReasoningDeltaToStreamBlocks([], "先想。");
    blocks = appendTextDeltaToStreamBlocks(blocks, "正文");
    blocks = appendReasoningDeltaToStreamBlocks(blocks, "后续思考。");
    blocks = applyAssistantToolEventToStreamBlocks(blocks, JSON.stringify({
      role: "assistant",
      content: null,
      tool_calls: [{
        id: "tool-1",
        type: "function",
        function: {
          name: "operate",
          arguments: "{\"action\":\"wait\"}",
        },
      }],
    }));

    expect(blocks).toEqual([
      { reasoning: "先想。", text: "正文", tools: [] },
      {
        reasoning: "后续思考。",
        text: "",
        tools: [{
          toolCallId: "tool-1",
          name: "operate",
          argsText: "{\"action\":\"wait\"}",
          status: "doing",
        }],
      },
    ]);
  });
});
