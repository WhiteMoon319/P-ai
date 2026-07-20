import { describe, expect, it } from "vitest";
import { hideIncompleteInlineMath } from "./streaming-math";

describe("hideIncompleteInlineMath", () => {
  it("does not treat a dollar sign in closed inline code as unfinished math", () => {
    expect(hideIncompleteInlineMath("我把 `$` 逃掉，直接拿环境变量值。")).toBe(
      "我把 `$` 逃掉，直接拿环境变量值。",
    );
  });

  it("does not treat a dollar sign in closed multiline inline code as unfinished math", () => {
    expect(hideIncompleteInlineMath("说明 `代码\n$` 后续")).toBe("说明 `代码\n$` 后续");
  });

  it("keeps text after closed inline math visible", () => {
    expect(hideIncompleteInlineMath("结果是 $x$，继续执行后续步骤。")).toBe(
      "结果是 $x$，继续执行后续步骤。",
    );
  });

  it("hides only the unfinished inline math tail", () => {
    expect(hideIncompleteInlineMath("结果是 $x + 1")).toBe("结果是 ");
  });
});
