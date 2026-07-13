import { describe, expect, it } from "vitest";
import { normalizeMermaidCodeForRender } from "./MermaidBlock";

describe("normalizeMermaidCodeForRender", () => {
  it("将 LLM 输出的字面换行标记转为 Mermaid 可渲染换行", () => {
    expect(normalizeMermaidCodeForRender("A[alpha\\nBeta]"))
      .toBe("A[alpha<br/>Beta]");
    expect(normalizeMermaidCodeForRender("A[alpha\\NBeta]"))
      .toBe("A[alpha<br/>Beta]");
  });

  it("不改写 Mermaid 源码中的真实换行", () => {
    expect(normalizeMermaidCodeForRender("flowchart TD\nA --> B"))
      .toBe("flowchart TD\nA --> B");
  });
});
