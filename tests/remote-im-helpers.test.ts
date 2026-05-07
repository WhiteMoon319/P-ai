import { describe, expect, it } from "vitest";
import { parseKeywordList } from "../src/features/config/views/config-tabs/remote-im/helpers";

describe("remote im helpers", () => {
  it("normalizes fullwidth and halfwidth comma separated keyword lists", () => {
    expect(parseKeywordList("闭嘴， 张嘴, 闭嘴\n继续说\r\n张嘴")).toEqual([
      "闭嘴",
      "张嘴",
      "继续说",
    ]);
  });
});
