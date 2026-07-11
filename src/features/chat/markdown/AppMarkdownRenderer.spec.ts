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

  it("groups adjacent toolcall badges into one wrench with count", async () => {
    const html = await renderMarkdown("前文[toolcall:a][toolcall:b] [toolcall:c] 后文");
    expect((html.match(/data-toolcall-pill="true"/g) || []).length).toBe(1);
    expect(html).toContain("ecall-md-toolcall-ref-count");
    expect(html).toContain("+3");
    expect(html).toContain("data-toolcall-id=\"a\"");
    expect(html).toContain("toolcall:a\ntoolcall:b\ntoolcall:c");
  });

  it("groups consecutive marker-only paragraphs into one wrench", async () => {
    const html = await renderMarkdown("[toolcall:a][toolcall:b]\n\n[toolcall:c]\n\n后面正文");
    expect((html.match(/data-toolcall-pill="true"/g) || []).length).toBe(1);
    expect(html).toContain("+3");
    expect(html).toContain("后面正文");
  });

  it("groups tools across blank lines between paragraphs", async () => {
    const html = await renderMarkdown("前文 [toolcall:a][toolcall:b]\n\n[toolcall:c]\n\n[toolcall:d] 后文");
    expect((html.match(/data-toolcall-pill="true"/g) || []).length).toBe(1);
    expect(html).toContain("+4");
    expect(html).toContain("前文");
    expect(html).toContain("后文");
  });
});
