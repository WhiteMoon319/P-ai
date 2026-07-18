import { describe, expect, it } from "vitest";
import type { IdeContextReferenceItem, IdeContextWorkspaceGroup } from "../../../types/app";
import {
  useChatComposerAppearance,
  visibleChatComposerContextGroups,
} from "./use-chat-composer-appearance";

const sideReference = { id: "side-file" } as IdeContextReferenceItem;
const ideBridgeGroup = {
  workspacePath: "E:/repo",
  workspaceName: "repo",
  references: [{ id: "ide-file" } as IdeContextReferenceItem],
} satisfies IdeContextWorkspaceGroup;

describe("use-chat-composer-appearance", () => {
  it("文件候选标签开关默认关闭", () => {
    const appearance = useChatComposerAppearance();

    expect(appearance.sideFileTagsEnabled.value).toBe(false);
    expect(appearance.ideBridgeFileTagsEnabled.value).toBe(false);
  });

  it("只返回已开启来源的候选标签组", () => {
    const baseInput = {
      sideReferences: [sideReference],
      sideWorkspacePath: "E:/repo",
      sideWorkspaceName: "repo",
      ideBridgeGroups: [ideBridgeGroup],
    };

    expect(visibleChatComposerContextGroups({
      ...baseInput,
      sideFileTagsEnabled: false,
      ideBridgeFileTagsEnabled: false,
    })).toEqual([]);
    expect(visibleChatComposerContextGroups({
      ...baseInput,
      sideFileTagsEnabled: true,
      ideBridgeFileTagsEnabled: false,
    }).map((group) => group.references[0]?.id)).toEqual(["side-file"]);
    expect(visibleChatComposerContextGroups({
      ...baseInput,
      sideFileTagsEnabled: false,
      ideBridgeFileTagsEnabled: true,
    }).map((group) => group.references[0]?.id)).toEqual(["ide-file"]);
  });

  it("VS Code 宿主固定显示 IDE 桥标签并隐藏侧边文件标签", () => {
    expect(visibleChatComposerContextGroups({
      sideReferences: [sideReference],
      sideWorkspacePath: "E:/repo",
      sideWorkspaceName: "repo",
      ideBridgeGroups: [ideBridgeGroup],
      sideFileTagsEnabled: true,
      ideBridgeFileTagsEnabled: false,
      host: "vscode",
    }).map((group) => group.references[0]?.id)).toEqual(["ide-file"]);
  });
});
