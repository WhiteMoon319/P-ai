function isEscapedAt(text: string, index: number): boolean {
  let backslashCount = 0;
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === "\\"; cursor -= 1) {
    backslashCount += 1;
  }
  return backslashCount % 2 === 1;
}

function findClosedInlineCodeEnd(text: string, start: number): number {
  if (isEscapedAt(text, start)) return -1;
  let delimiterLength = 1;
  while (text[start + delimiterLength] === "`") delimiterLength += 1;
  const delimiter = "`".repeat(delimiterLength);
  const end = text.indexOf(delimiter, start + delimiterLength);
  return end < 0 ? -1 : end + delimiterLength;
}

function findUnescapedSingleDollar(text: string, from: number): number {
  let cursor = Math.max(0, from);
  while (cursor < text.length) {
    const index = text.indexOf("$", cursor);
    if (index < 0) return -1;
    if (text[index - 1] === "$" || text[index + 1] === "$") {
      cursor = index + 1;
      continue;
    }
    if (!isEscapedAt(text, index)) return index;
    cursor = index + 1;
  }
  return -1;
}

function inlineMathCanRender(text: string): boolean {
  const content = String(text || "").replace(/\s+/g, " ").trim();
  return !!content && !/[\r\n]/.test(content);
}

/**
 * 流式期间临时隐藏未闭合的行内公式，避免 Markdown 将后续正文吞入公式。
 * 闭合的行内代码范围不参与 `$` 公式判定。
 */
export function hideIncompleteInlineMath(text: string): string {
  if (!text.includes("$")) return text;

  const lines = text.split("\n");
  let offset = 0;
  let inlineCodeEnd = -1;

  for (const line of lines) {
    if (/^\s*```/.test(line)) {
      offset += line.length + 1;
      continue;
    }
    let openMathStartInLine = -1;
    let searchFrom = 0;
    while (searchFrom < line.length) {
      const absoluteSearchFrom = offset + searchFrom;
      if (inlineCodeEnd > absoluteSearchFrom) {
        searchFrom = Math.min(line.length, inlineCodeEnd - offset);
        continue;
      }
      const delimiterIndex = findUnescapedSingleDollar(line, searchFrom);
      const codeStart = text.indexOf("`", absoluteSearchFrom);
      const codeStartInLine = codeStart >= offset && codeStart < offset + line.length
        ? codeStart - offset
        : -1;
      if (codeStartInLine >= 0 && (delimiterIndex < 0 || codeStartInLine < delimiterIndex)) {
        const codeEnd = findClosedInlineCodeEnd(text, codeStart);
        if (codeEnd >= 0) {
          inlineCodeEnd = codeEnd;
          searchFrom = Math.min(line.length, codeEnd - offset);
          continue;
        }
        searchFrom = codeStartInLine + 1;
        continue;
      }
      if (delimiterIndex < 0) break;
      if (openMathStartInLine < 0) {
        openMathStartInLine = delimiterIndex;
      } else {
        const content = line.slice(openMathStartInLine + 1, delimiterIndex);
        if (inlineMathCanRender(content)) {
          openMathStartInLine = -1;
        } else {
          openMathStartInLine = delimiterIndex;
        }
      }
      searchFrom = delimiterIndex + 1;
    }
    if (openMathStartInLine >= 0) {
      const pendingContent = line.slice(openMathStartInLine + 1);
      if (inlineMathCanRender(pendingContent)) {
        return text.slice(0, offset + openMathStartInLine);
      }
    }
    offset += line.length + 1;
  }

  return text;
}
