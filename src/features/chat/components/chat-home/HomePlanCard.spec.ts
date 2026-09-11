import { describe, expect, it } from "vitest";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import HomePlanCard from "./HomePlanCard.vue";
import { i18n } from "../../../../i18n";

describe("HomePlanCard", () => {
  it("renders plan title and checklist items with direct markdownContent", async () => {
    const markdown = `
# 幕墙计划卡功能

## 任务清单
- [x] 第一步已完成
- [ ] 第二步待执行
`;
    const app = createSSRApp({
      render: () =>
        h(HomePlanCard, {
          plan: {
            path: ".pai/plan/chat/20260911_plan.md",
            markdownContent: markdown,
          },
        }),
    });
    app.use(i18n);
    const html = await renderToString(app);

    expect(html).toContain("幕墙计划卡功能");
    expect(html).toContain("第一步已完成");
    expect(html).toContain("第二步待执行");
    expect(html).toContain("line-through");
    expect(html).toContain("最近计划");
    expect(html).not.toContain("执行中");
    expect(html).not.toContain("项");
  });

  it("renders plan title and numbered steps without capsules", async () => {
    const app = createSSRApp({
      render: () =>
        h(HomePlanCard, {
          plan: {
            path: ".pai/plan/chat/sample.md",
            markdownContent: "# 重构方案\n1. 步骤一\n2. 步骤二",
          },
        }),
    });
    app.use(i18n);
    const html = await renderToString(app);
    expect(html).toContain("最近计划");
    expect(html).toContain("重构方案");
    expect(html).toContain("步骤一");
    expect(html).toContain("步骤二");
    expect(html).not.toContain("badge");
  });

  it("renders fallback filename title when markdown is empty", async () => {
    const app = createSSRApp({
      render: () =>
        h(HomePlanCard, {
          plan: {
            path: ".pai/plan/chat/20260911_测试计划.md",
            markdownContent: "",
          },
        }),
    });
    app.use(i18n);
    const html = await renderToString(app);
    expect(html).toContain("测试计划");
  });
});
