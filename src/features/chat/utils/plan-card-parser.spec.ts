import { describe, expect, it } from "vitest";
import { parsePlanMarkdown } from "./plan-card-parser";

describe("plan-card-parser", () => {
  it("empty markdown falls back to path filename without date prefix", () => {
    const result = parsePlanMarkdown("", ".pai/plan/chat/20260911_测试计划.md");
    expect(result.title).toBe("测试计划");
    expect(result.items).toHaveLength(0);
    expect(result.totalItemCount).toBe(0);
  });

  it("extracts H1 title and checklist items", () => {
    const md = `
# 侧边栏预览卡计划

## 任务清单
- [x] 完成解析器
- [ ] 编写前端卡片
- [ ] 冒烟测试
`;
    const result = parsePlanMarkdown(md, ".pai/plan/chat/card.md");
    expect(result.title).toBe("侧边栏预览卡计划");
    expect(result.items).toEqual([
      { text: "完成解析器", status: "completed", kind: "checkbox" },
      { text: "编写前端卡片", status: "pending", kind: "checkbox" },
      { text: "冒烟测试", status: "pending", kind: "checkbox" },
    ]);
    expect(result.totalItemCount).toBe(3);
  });

  it("extracts numbered steps when no checklist exists", () => {
    const md = `
# 数据库迁移

1. 备份现有数据
2. 执行迁移脚本
3. 校验完整性
`;
    const result = parsePlanMarkdown(md);
    expect(result.title).toBe("数据库迁移");
    expect(result.items).toEqual([
      { text: "备份现有数据", kind: "numbered" },
      { text: "执行迁移脚本", kind: "numbered" },
      { text: "校验完整性", kind: "numbered" },
    ]);
    expect(result.totalItemCount).toBe(3);
  });

  it("extracts section headings when neither checklist nor numbered items exist", () => {
    const md = `
# 重构计划

## 目标
详细目标说明...

## 根因
深入分析...

## 方案设计
具体方案...

## 验收
验收标准...
`;
    const result = parsePlanMarkdown(md);
    expect(result.title).toBe("重构计划");
    expect(result.items).toEqual([
      { text: "目标", kind: "heading" },
      { text: "根因", kind: "heading" },
      { text: "方案设计", kind: "heading" },
      { text: "验收", kind: "heading" },
    ]);
    expect(result.totalItemCount).toBe(4);
  });

  it("does not treat decimal or date-like numbers as numbered steps", () => {
    const md = `
# 版本说明

1.1 目标与范围
2026.09.11 版本说明

## 小节标题
`;
    const result = parsePlanMarkdown(md);
    expect(result.title).toBe("版本说明");
    expect(result.items).toEqual([{ text: "小节标题", kind: "heading" }]);
  });

  it("cleans inline formatting like backticks and bold text", () => {
    const md = `
# **加粗标题**

1. 检查 \`src/main.rs\` 文件
2. 运行 **全量测试**
`;
    const result = parsePlanMarkdown(md);
    expect(result.title).toBe("加粗标题");
    expect(result.items).toEqual([
      { text: "检查 src/main.rs 文件", kind: "numbered" },
      { text: "运行 全量测试", kind: "numbered" },
    ]);
  });
});
