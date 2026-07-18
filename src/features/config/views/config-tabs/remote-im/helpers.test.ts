import { describe, expect, it } from "vitest";
import {
  DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS,
  DEFAULT_REMOTE_IM_GROUP_REPLY_PACING,
  cloneChannelBehaviorSettings,
  normalizeChannelBehaviorSettings,
  normalizeGroupReplyPacing,
  parseSpaceSeparatedList,
  resolveBehaviorDraftSave,
} from "./helpers";

describe("remote IM group behavior helpers", () => {
  it("uses channel defaults as the only source for legacy or incomplete channel behavior", () => {
    expect(normalizeChannelBehaviorSettings(undefined)).toEqual(DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS);
    const normalized = normalizeChannelBehaviorSettings({
      blockedMessagePrefixes: ["!", "!", " "],
      patienceSeconds: -3,
      muteDurationSeconds: Number.NaN,
      groupReplyPacing: { assistantDebounceSeconds: 0 },
    });
    expect(normalized.blockedMessagePrefixes).toEqual(["!"]);
    expect(normalized.patienceSeconds).toBe(0);
    expect(normalized.muteDurationSeconds).toBe(600);
    expect(normalized.groupReplyPacing.assistantDebounceSeconds).toBe(1);

    const cloned = cloneChannelBehaviorSettings(normalized);
    cloned.muteKeywords.push("新词");
    expect(normalized.muteKeywords).not.toContain("新词");
  });

  it("defaults legacy contacts and repairs invalid numeric values", () => {
    expect(normalizeGroupReplyPacing(undefined)).toEqual(DEFAULT_REMOTE_IM_GROUP_REPLY_PACING);
    const normalized = normalizeGroupReplyPacing({
      assistantDebounceSeconds: 0,
      secretaryInspectionSeconds: Number.NaN,
      inspectionJitterRatio: 4,
      maximumEnergy: -1,
      negativeEnergyDelta: 5,
      normalReplyMaxChars: 30,
      focusReplyMaxChars: 10,
    });
    expect(normalized.assistantDebounceSeconds).toBe(1);
    expect(normalized.secretaryInspectionSeconds).toBe(7);
    expect(normalized.inspectionJitterRatio).toBe(1);
    expect(normalized.maximumEnergy).toBe(0.01);
    expect(normalized.negativeEnergyDelta).toBe(0);
    expect(normalized.normalReplyMaxChars).toBe(30);
    expect(normalized.focusReplyMaxChars).toBe(10);
  });

  it("provides demonstrative default phrases for energy and focused replies", () => {
    const defaults = normalizeGroupReplyPacing(undefined);

    expect(defaults.positiveEnergyPhrases).toEqual(["厉害", "像人"]);
    expect(defaults.negativeEnergyPhrases).toEqual(["够了", "烦", "串了"]);
    expect(defaults.focusInstructions).toEqual(["分析", "总结", "好好想想", "为什么", "到底"]);
  });

  it("parses space-separated phrases without copying runtime ledger state", () => {
    expect(parseSpaceSeparatedList("谢谢  谢谢\n继续")).toEqual(["谢谢", "继续"]);
  });

  it("keeps dirty drafts after save failure and preserves edits made while saving", () => {
    const submitted = { mute: "旧草稿", energy: 100 };
    const submittedSnapshot = JSON.stringify(submitted);
    const failed = resolveBehaviorDraftSave(
      submitted,
      JSON.stringify({ mute: "原值", energy: 100 }),
      submittedSnapshot,
      null,
      new Error("disk busy"),
    );
    expect(failed.draft).toEqual(submitted);
    expect(failed.savedSnapshot).toBe(JSON.stringify({ mute: "原值", energy: 100 }));
    expect(failed.error).toContain("disk busy");

    const editedWhileSaving = { mute: "请求期间的新编辑", energy: 100 };
    const succeeded = resolveBehaviorDraftSave(
      editedWhileSaving,
      JSON.stringify({ mute: "原值", energy: 100 }),
      submittedSnapshot,
      { mute: "旧草稿", energy: 100 },
    );
    expect(succeeded.draft).toEqual(editedWhileSaving);
    expect(succeeded.savedSnapshot).toBe(submittedSnapshot);
  });
});
