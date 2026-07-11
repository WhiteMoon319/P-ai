export function commonPrefixLength(left: string, right: string): number {
  const a = String(left || "");
  const b = String(right || "");
  const max = Math.min(a.length, b.length);
  let index = 0;
  while (index < max && a.charCodeAt(index) === b.charCodeAt(index)) {
    index += 1;
  }
  return index;
}

/**
 * 完成态正文合并：优先只在尾部追加，不回写已渲染前缀。
 * - final 是 current 的延伸 → 返回 final
 * - current 已包含 final → 保留 current
 * - 中间分叉 → 保留 current（避免整段替换导致气泡重绘/跳动）
 */
export function mergeAssistantText(currentText: string, finalText: string): string {
  const current = String(currentText || "");
  const finalValue = String(finalText || "");
  if (!current) return finalValue;
  if (!finalValue) return current;
  if (finalValue.startsWith(current)) return finalValue;
  if (current.startsWith(finalValue)) return current;
  return current;
}

export function hasAssistantVisibleOutput(result: {
  assistantText: string;
}): boolean {
  return !!result.assistantText.trim();
}

export function consumeClosedMarkdownBlocks(input: string): { chunks: string[]; tail: string } {
  // 乐观渲染策略：直接返回所有内容作为 chunks，tail 为空
  // 这样所有 markdown 元素（标题、粗体、引用等）都能立即渲染
  if (!input) return { chunks: [], tail: "" };

  return { chunks: [input], tail: "" };
}
