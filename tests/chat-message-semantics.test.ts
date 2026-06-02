import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../src/types/app";
import {
  appendReasoningToStreamActivityItems,
  applyToolStatusToStreamActivityItems,
  inspectUndoablePatchCalls,
  normalizeMessageToolHistoryEvents,
  projectChatActivityForDisplay,
  projectMessageForDisplay,
  projectStreamingChatActivityForDisplay,
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
      ...textMessage("a-1", "assistant", "我查好了\n[标准思考]\n先检查上下文"),
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
    expect(projection.reasoningStandard).toContain("先检查上下文");
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

  it("stitches tool round reasoning with final answer reasoning for display only", () => {
    const message: ChatMessage = {
      ...textMessage("a-4", "assistant", "终端版本是 PowerShell 7.5.4"),
      providerMeta: {
        reasoningStandard: "我已经拿到工具结果，现在直接回答用户终端版本。",
      },
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

    expect(projection.reasoningStandard).toBe([
      "先调用终端工具查看 PowerShell 版本。",
      "我已经拿到工具结果，现在直接回答用户终端版本。",
    ].join("\n\n"));
    expect(message.providerMeta?.reasoningStandard).toBe("我已经拿到工具结果，现在直接回答用户终端版本。");
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
      providerMeta: {
        reasoningStandard: "旧汇总思考不作为活动来源",
      },
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
      reasoningStandard: "正在分析",
      toolCalls: [{ toolCallId: "t-1", name: "exec", argsText: "{}", status: "doing" }],
      running: true,
    }).activityStatus).toBe("running_tool");

    expect(projectStreamingChatActivityForDisplay({
      reasoningStandard: "正在分析",
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

  it("keeps streaming tool update in place without changing event order", () => {
    const withReasoning = appendReasoningToStreamActivityItems([], "先看文件。");
    const withTool = applyToolStatusToStreamActivityItems(withReasoning, {
      toolName: "exec",
      toolCallId: "tool-1",
      toolStatus: "running",
      toolArgs: "{\"command\":\"ls\"}",
    });
    const withMoreReasoning = appendReasoningToStreamActivityItems(withTool, "再确认结果。");
    const done = applyToolStatusToStreamActivityItems(withMoreReasoning, {
      toolName: "exec",
      toolCallId: "tool-1",
      toolStatus: "done",
      toolArgs: "",
    });

    expect(done.map((item) => item.kind === "tool" ? `${item.name}:${item.status}` : item.text)).toEqual([
      "先看文件。",
      "exec:done",
      "再确认结果。",
    ]);
    expect(done[1]).toMatchObject({
      kind: "tool",
      argsText: "{\"command\":\"ls\"}",
      status: "done",
    });
  });

  it("uses event-level streaming activity items before legacy reasoning and tools", () => {
    const activity = projectStreamingChatActivityForDisplay({
      reasoningStandard: "旧拼接思考",
      toolCalls: [{ toolCallId: "legacy-tool", name: "legacy", argsText: "{}", status: "done" }],
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
});
