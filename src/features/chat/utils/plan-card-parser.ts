export interface ParsedPlanItem {
  text: string;
  status?: "pending" | "completed";
  kind: "checkbox" | "numbered" | "heading" | "bullet";
}

export interface ParsedPlanOutline {
  title: string;
  items: ParsedPlanItem[];
  totalItemCount: number;
}

function cleanMarkdownInline(text: string): string {
  return String(text || "")
    .replace(/\*\*(.*?)\*\*/g, "$1")
    .replace(/\*(.*?)\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .trim();
}

function extractFallbackTitle(path: string): string {
  const normalized = String(path || "").replace(/\\/g, "/").trim();
  const filename = normalized.split("/").filter(Boolean).pop() || "";
  const withoutExt = filename.replace(/\.md$/i, "");
  // 去除类似 20260911_ 的 8 位日期前缀
  const cleaned = withoutExt.replace(/^\d{8}_/, "");
  return cleaned.trim() || withoutExt || "未命名计划";
}

/**
 * 解析计划 Markdown 内容，提炼出标题和结构化任务/阶段大纲。
 * 优先级：Checklist 复选框 > 数字编号步骤 > 二级/三级标题 > 无序列表项。
 */
export function parsePlanMarkdown(markdown: string, fallbackPath = ""): ParsedPlanOutline {
  const content = String(markdown || "").trim();
  const fallbackTitle = extractFallbackTitle(fallbackPath);

  if (!content) {
    return {
      title: fallbackTitle,
      items: [],
      totalItemCount: 0,
    };
  }

  // 1. 提取一级标题
  const h1Match = content.match(/^#\s+(.+)$/m);
  const title = h1Match ? cleanMarkdownInline(h1Match[1]) : fallbackTitle;

  const lines = content.split(/\r?\n/);

  // 2. 检查 Checklist 项 (- [ ] 或 - [x])
  const checklistItems: ParsedPlanItem[] = [];
  const checklistRegex = /^\s*[-*]\s*\[([ xX])\]\s*(.+)$/;

  for (const line of lines) {
    const match = line.match(checklistRegex);
    if (match) {
      const isChecked = match[1].toLowerCase() === "x";
      const itemText = cleanMarkdownInline(match[2]);
      if (itemText) {
        checklistItems.push({
          text: itemText,
          status: isChecked ? "completed" : "pending",
          kind: "checkbox",
        });
      }
    }
  }

  if (checklistItems.length > 0) {
    return {
      title,
      items: checklistItems,
      totalItemCount: checklistItems.length,
    };
  }

  // 3. 检查数字编号步骤 (1. xxx, 步骤 1: xxx)
  // 数字序号后必须跟空白，避免把 `1.1 小节`、`2026.09.11` 这类小数/日期误当步骤
  const numberedItems: ParsedPlanItem[] = [];
  const numberedRegex = /^\s*(?:\d+[.)]\s+|步骤\s*\d+\s*[:：]\s*)(.+)$/;

  for (const line of lines) {
    const match = line.match(numberedRegex);
    if (match) {
      const itemText = cleanMarkdownInline(match[1]);
      if (itemText) {
        numberedItems.push({
          text: itemText,
          kind: "numbered",
        });
      }
    }
  }

  if (numberedItems.length > 0) {
    return {
      title,
      items: numberedItems,
      totalItemCount: numberedItems.length,
    };
  }

  // 4. 检查二级/三级标题 (## 目标, ### 修改点)
  const headingItems: ParsedPlanItem[] = [];
  const headingRegex = /^#{2,3}\s+(.+)$/;

  for (const line of lines) {
    const match = line.match(headingRegex);
    if (match) {
      const headingText = cleanMarkdownInline(match[1]);
      if (headingText) {
        headingItems.push({
          text: headingText,
          kind: "heading",
        });
      }
    }
  }

  if (headingItems.length > 0) {
    return {
      title,
      items: headingItems,
      totalItemCount: headingItems.length,
    };
  }

  // 5. 兜底：无序列表项
  const bulletItems: ParsedPlanItem[] = [];
  const bulletRegex = /^\s*[-*+]\s+(.+)$/;

  for (const line of lines) {
    const match = line.match(bulletRegex);
    if (match) {
      const bulletText = cleanMarkdownInline(match[1]);
      if (bulletText) {
        bulletItems.push({
          text: bulletText,
          kind: "bullet",
        });
      }
    }
  }

  return {
    title,
    items: bulletItems,
    totalItemCount: bulletItems.length,
  };
}
