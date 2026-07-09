import { describe, expect, it } from "vitest";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import AppMarkdownRenderer from "./AppMarkdownRenderer.vue";
import { i18n } from "../../../i18n";

async function renderMarkdown(text: string): Promise<string> {
  const app = createSSRApp({
    render: () => h(AppMarkdownRenderer, { text, streaming: false }),
  });
  app.use(i18n);
  return renderToString(app);
}

describe("AppMarkdownRenderer", () => {
  it("renders display math blocks nested inside blockquotes", async () => {
    const html = await renderMarkdown([
      "> The force of interest, \\( \\delta(t) \\), is a function of time ...",
      ">",
      "> \\[",
      "> \\delta(t)=",
      "> \\begin{cases}",
      "> 0.07-0.005t, & t\\le 8,\\\\",
      "> 0.06, & t>8.",
      "> \\end{cases}",
      "> \\]",
      ">",
      "> Calculate the present value ...",
    ].join("\n"));

    expect(html).toContain("ecall-md-quote");
    expect(html).toContain("ecall-md-math-block-shell");
    expect(html).toContain("\\delta(t)=");
    expect(html).toContain("Calculate the present value");
  });
});
