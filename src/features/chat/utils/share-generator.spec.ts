import { describe, expect, it } from "vitest";
import { isDarkShareTheme } from "./share-generator";

describe("share theme detection", () => {
  it("treats a generated dark theme as dark from its computed color scheme", () => {
    expect(isDarkShareTheme("generated", "dark")).toBe(true);
    expect(isDarkShareTheme("generated", "light")).toBe(false);
  });
});
