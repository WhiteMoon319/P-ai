// ==================== Markdown Block Parser ====================
// Lightweight markdown parser based on SidebarLightMarkdown's approach,
// extended with math block detection ($$...$$ and $...$) and mermaid awareness.

export type MarkdownBlock =
  | { type: "paragraph"; text: string; key: string }
  | { type: "heading"; level: 1 | 2 | 3 | 4; text: string; key: string }
  | { type: "quote"; text: string; key: string }
  | { type: "list"; ordered: boolean; items: string[]; key: string }
  | { type: "table"; headers: string[]; rows: string[][]; key: string }
  | { type: "code"; lang: string; text: string; key: string }
  | { type: "math"; text: string; raw: string; key: string }
  | { type: "details"; summary: string; body: string; open: boolean; key: string }
  | { type: "footnotes"; items: { id: string; text: string }[]; key: string }
  | { type: "hr"; key: string };

export type InlineSegment =
  | { type: "text"; text: string }
  | { type: "html_br" }
  | { type: "toolcall_ref"; id: string; label: string }
  | { type: "footnote_ref"; id: string }
  | { type: "code"; text: string }
  | { type: "math"; text: string; raw: string; display: boolean }
  | { type: "link"; text: string; href: string }
  | { type: "image"; alt: string; src: string }
  | { type: "imageLink"; alt: string; src: string; href: string }
  | { type: "html_sub"; children: InlineSegment[] }
  | { type: "html_sup"; children: InlineSegment[] }
  | { type: "html_kbd"; children: InlineSegment[] }
  | { type: "html_mark"; children: InlineSegment[] }
  | { type: "strong"; children: InlineSegment[] }
  | { type: "em"; children: InlineSegment[] }
  | { type: "strongEm"; children: InlineSegment[] }
  | { type: "delete"; children: InlineSegment[] };

// ==================== Block Parser ====================

function pushParagraph(blocks: MarkdownBlock[], lines: string[], keyPrefix: string): MarkdownBlock | null {
  const text = lines.join("\n").trim();
  lines.length = 0;
  if (!text) return null;
  const block = { type: "paragraph" as const, text, key: `${keyPrefix}-p-${blocks.length}` };
  blocks.push(block);
  return block;
}

function parseTableRow(line: string | undefined): string[] | null {
  const raw = String(line || "").trim();
  if (!raw.includes("|")) return null;
  const trimmed = raw.replace(/^\|/, "").replace(/\|$/, "");
  const cells = trimmed.split("|").map((cell) => cell.trim());
  if (cells.length < 2) return null;
  return cells;
}

function isTableSeparator(line: string | undefined, expectedCells: number): boolean {
  const cells = parseTableRow(line);
  if (!cells || cells.length < expectedCells) return false;
  return cells.every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

type DetailsMatch = {
  summary: string;
  body: string;
  open: boolean;
  endIndex: number;
};

function tryParseDetailsBlock(lines: string[], startIndex: number, streaming: boolean): DetailsMatch | null {
  const startLine = String(lines[startIndex] || "");
  const openMatch = startLine.match(/^\s*<details(\s+open)?\s*>\s*$/i);
  if (!openMatch) return null;
  let summary = "";
  const bodyLines: string[] = [];
  let endIndex = startIndex;
  let pendingSummary = false;
  let summaryCaptured = false;

  for (let index = startIndex + 1; index < lines.length; index += 1) {
    const line = String(lines[index] || "");
    const trimmed = line.trim();
    endIndex = index;

    if (!summaryCaptured) {
      const sameLineSummary = trimmed.match(/^<summary>(.*?)<\/summary>\s*$/i);
      if (sameLineSummary) {
        summary = String(sameLineSummary[1] || "").trim();
        summaryCaptured = true;
        continue;
      }
      if (/^<summary>\s*$/i.test(trimmed)) {
        pendingSummary = true;
        summaryCaptured = true;
        continue;
      }
    }

    if (pendingSummary) {
      const closingOnly = trimmed.match(/^(.*?)<\/summary>\s*$/i);
      if (closingOnly) {
        summary = String(closingOnly[1] || "").trim();
        pendingSummary = false;
        continue;
      }
      summary = summary ? `${summary}\n${line}` : line;
      continue;
    }

    if (/^\s*<\/details>\s*$/i.test(trimmed)) {
      return {
        summary: summary.trim(),
        body: bodyLines.join("\n").trim(),
        open: !!openMatch[1],
        endIndex: index,
      };
    }
    bodyLines.push(line);
  }

  if (!streaming) return null;
  return {
    summary: summary.trim(),
    body: bodyLines.join("\n").trim(),
    open: !!openMatch[1],
    endIndex,
  };
}

function isEscapedAt(text: string, index: number): boolean {
  let slashCount = 0;
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === "\\"; cursor -= 1) {
    slashCount += 1;
  }
  return slashCount % 2 === 1;
}

function findUnescapedDelimiter(text: string, delimiter: "$" | "$$", from: number): number {
  let cursor = Math.max(0, from);
  while (cursor < text.length) {
    const index = text.indexOf(delimiter, cursor);
    if (index < 0) return -1;
    if (!isEscapedAt(text, index)) return index;
    cursor = index + delimiter.length;
  }
  return -1;
}

function normalizeMathText(value: string): string {
  return String(value || "").trim();
}

function mathBlock(raw: string, text: string, key: string): MarkdownBlock {
  return {
    type: "math",
    text: normalizeMathText(text),
    raw: String(raw || "").trim(),
    key,
  };
}

function parseDisplayMathBlockStart(line: string): { closed: boolean; text: string; raw: string } | null {
  const trimmed = String(line || "").trim();
  if (!trimmed.startsWith("$$")) return null;
  const closeIndex = findUnescapedDelimiter(trimmed, "$$", 2);
  if (closeIndex >= 0 && !trimmed.slice(closeIndex + 2).trim()) {
    const text = trimmed.slice(2, closeIndex);
    if (!normalizeMathText(text)) return null;
    return { closed: true, text, raw: trimmed };
  }
  return {
    closed: false,
    text: String(line || "").slice(String(line || "").indexOf("$$") + 2),
    raw: String(line || ""),
  };
}

export function parseMarkdownBlocks(input: string, streaming = false): MarkdownBlock[] {
  const normalized = String(input || "").replace(/\r\n?/g, "\n");
  const lines = normalized.split("\n");
  const result: MarkdownBlock[] = [];
  const paragraphLines: string[] = [];
  const footnotes = new Map<string, string>();
  const referencedFootnoteIds: string[] = [];
  const referencedFootnoteIdSet = new Set<string>();
  let inCode = false;
  let codeLang = "";
  let codeLines: string[] = [];
  let inMathBlock = false;
  let mathLines: string[] = [];
  let mathRawLines: string[] = [];
  let activeList: { ordered: boolean; items: string[] } | null = null;

  const recordFootnoteRefs = (text: string) => {
    const pattern = /\[\^([^\]\n]+)\]/g;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(text))) {
      const id = String(match[1] || "").trim();
      if (!id || referencedFootnoteIdSet.has(id)) continue;
      referencedFootnoteIdSet.add(id);
      referencedFootnoteIds.push(id);
    }
  };

  const flushList = () => {
    if (!activeList) return;
    activeList.items.forEach(recordFootnoteRefs);
    result.push({
      type: "list",
      ordered: activeList.ordered,
      items: activeList.items,
      key: `list-${result.length}`,
    });
    activeList = null;
  };

  const flushParagraph = () => {
    flushList();
    const block = pushParagraph(result, paragraphLines, "root");
    if (block?.type === "paragraph") recordFootnoteRefs(block.text);
  };

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];

    if (inMathBlock) {
      mathRawLines.push(line);
      const closeIndex = findUnescapedDelimiter(line, "$$", 0);
      if (closeIndex >= 0 && !line.slice(closeIndex + 2).trim()) {
        const beforeClose = line.slice(0, closeIndex);
        if (beforeClose || mathLines.length > 0) mathLines.push(beforeClose);
        result.push(mathBlock(mathRawLines.join("\n"), mathLines.join("\n"), `math-${result.length}`));
        inMathBlock = false;
        mathLines = [];
        mathRawLines = [];
        continue;
      }
      mathLines.push(line);
      continue;
    }

    const mathStart = !inCode ? parseDisplayMathBlockStart(line) : null;
    if (mathStart) {
      flushParagraph();
      if (mathStart.closed) {
        result.push(mathBlock(mathStart.raw, mathStart.text, `math-${result.length}`));
      } else {
        inMathBlock = true;
        mathLines = mathStart.text ? [mathStart.text] : [];
        mathRawLines = [mathStart.raw];
      }
      continue;
    }

    // Code fence
    const fenceMatch = line.match(/^(`{3,})([\w+-]*)\s*$/);
    if (fenceMatch) {
      if (inCode) {
        result.push({
          type: "code",
          lang: codeLang,
          text: codeLines.join("\n"),
          key: `code-${result.length}`,
        });
        inCode = false;
        codeLang = "";
        codeLines = [];
      } else {
        flushParagraph();
        inCode = true;
        codeLang = fenceMatch[2] || "";
        codeLines = [];
      }
      continue;
    }

    if (inCode) {
      codeLines.push(line);
      continue;
    }

    // Footnote definitions are collected and rendered together at the end.
    const footnoteMatch = line.match(/^\s{0,3}\[\^([^\]\n]+)\]:\s*(.*)$/);
    if (footnoteMatch) {
      flushParagraph();
      const id = String(footnoteMatch[1] || "").trim();
      const noteLines = [String(footnoteMatch[2] || "").trim()];
      while (lineIndex + 1 < lines.length) {
        const continuationMatch = lines[lineIndex + 1].match(/^(?: {4}|\t)(.*)$/);
        if (!continuationMatch) break;
        noteLines.push(String(continuationMatch[1] || "").trimEnd());
        lineIndex += 1;
      }
      if (id) footnotes.set(id, noteLines.join("\n").trim());
      continue;
    }

    if (!line.trim()) {
      flushParagraph();
      continue;
    }

    // Horizontal rule
    const hrMatch = line.match(/^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/);
    if (hrMatch) {
      flushParagraph();
      result.push({ type: "hr", key: `hr-${result.length}` });
      continue;
    }

    const detailsMatch = tryParseDetailsBlock(lines, lineIndex, streaming);
    if (detailsMatch) {
      flushParagraph();
      recordFootnoteRefs(detailsMatch.summary);
      recordFootnoteRefs(detailsMatch.body);
      result.push({
        type: "details",
        summary: detailsMatch.summary,
        body: detailsMatch.body,
        open: detailsMatch.open,
        key: `details-${result.length}`,
      });
      lineIndex = detailsMatch.endIndex;
      continue;
    }

    // Table
    const tableHeader = parseTableRow(line);
    if (tableHeader && isTableSeparator(lines[lineIndex + 1], tableHeader.length)) {
      flushParagraph();
      lineIndex += 2;
      const rows: string[][] = [];
      while (lineIndex < lines.length) {
        const row = parseTableRow(lines[lineIndex]);
        if (!row) break;
        rows.push(row);
        lineIndex += 1;
      }
      lineIndex -= 1;
      tableHeader.forEach(recordFootnoteRefs);
      rows.forEach((row) => row.forEach(recordFootnoteRefs));
      result.push({
        type: "table",
        headers: tableHeader,
        rows,
        key: `table-${result.length}`,
      });
      continue;
    }

    // Heading
    const headingMatch = line.match(/^\s{0,3}(#{1,4})\s+(.+?)\s*#*\s*$/);
    if (headingMatch) {
      flushParagraph();
      recordFootnoteRefs(headingMatch[2]);
      result.push({
        type: "heading",
        level: headingMatch[1].length as 1 | 2 | 3 | 4,
        text: headingMatch[2].trim(),
        key: `heading-${result.length}`,
      });
      continue;
    }

    // Blockquote
    const quoteMatch = line.match(/^\s{0,3}>\s?(.*)$/);
    if (quoteMatch) {
      flushParagraph();
      const quoteLines = [quoteMatch[1].trim()];
      while (lineIndex + 1 < lines.length) {
        const nextQuoteMatch = lines[lineIndex + 1].match(/^\s{0,3}>\s?(.*)$/);
        if (!nextQuoteMatch) break;
        quoteLines.push(nextQuoteMatch[1].trim());
        lineIndex += 1;
      }
      const quoteText = quoteLines.join("\n").trim();
      recordFootnoteRefs(quoteText);
      result.push({
        type: "quote",
        text: quoteText,
        key: `quote-${result.length}`,
      });
      continue;
    }

    // List item
    const listMatch = line.match(/^\s{0,3}(?:([-*+])|(\d+)[.)])\s+(.+)$/);
    if (listMatch) {
      const block = pushParagraph(result, paragraphLines, "list-before");
      if (block?.type === "paragraph") recordFootnoteRefs(block.text);
      const ordered = !!listMatch[2];
      if (!activeList || activeList.ordered !== ordered) {
        flushList();
        activeList = { ordered, items: [] };
      }
      activeList.items.push(listMatch[3].trim());
      continue;
    }

    flushList();
    paragraphLines.push(line);
  }

  // Flush remaining
  if (inCode) {
    // 流式和非流式都输出未闭合的代码块（乐观渲染）
    result.push({
      type: "code",
      lang: codeLang,
      text: codeLines.join("\n"),
      key: `code-${result.length}`,
    });
  }
  if (inMathBlock) {
    if (!streaming) {
      result.push(mathBlock(mathRawLines.join("\n"), mathLines.join("\n"), `math-${result.length}`));
    }
  }
  flushParagraph();
  const footnoteItems = referencedFootnoteIds
    .map((id) => ({ id, text: footnotes.get(id) || "" }))
    .filter((item) => item.text.trim());
  if (footnoteItems.length > 0) {
    result.push({
      type: "footnotes",
      items: footnoteItems,
      key: `footnotes-${result.length}`,
    });
  }
  return result.length > 0
    ? result
    : footnotes.size > 0
      ? []
    : [{ type: "paragraph", text: normalized, key: "fallback" }];
}

// ==================== Inline Parser ====================

const URL_PATTERN = /(https?:\/\/[^\s<>()]+|file:\/\/\/[^\s<>()]+)/g;
const MARKDOWN_IMAGE_LINK_PATTERN = /\[!\[([^\]\n]*)\]\(([^)\n]+)\)\]\(([^)\n]+)\)/g;
const MARKDOWN_LINK_PATTERN = /!?\[([^\]\n]*)\]\(([^)\n]+)\)/g;
const TOOLCALL_REF_PATTERN = /\[toolcall:([^\]\n]+)\]/g;
const FOOTNOTE_REF_PATTERN = /\[\^([^\]\n]+)\]/g;

function trimTrailingUrlPunctuation(value: string): { href: string; trailing: string } {
  let href = value;
  let trailing = "";
  while (/[.,;:!?，。！？；：、]$/.test(href)) {
    trailing = `${href.slice(-1)}${trailing}`;
    href = href.slice(0, -1);
  }
  return { href, trailing };
}

function pushTextSegment(segments: InlineSegment[], text: string) {
  if (!text) return;
  const previous = segments[segments.length - 1];
  if (previous?.type === "text") {
    previous.text += text;
    return;
  }
  segments.push({ type: "text", text });
}

type LinkMatch =
  | { kind: "image_link"; start: number; end: number; raw: string; alt: string; src: string; href: string }
  | { kind: "markdown"; start: number; end: number; raw: string; text: string; href: string; image: boolean }
  | { kind: "auto"; start: number; end: number; href: string }
  | { kind: "toolcall_ref"; start: number; end: number; raw: string; id: string }
  | { kind: "footnote_ref"; start: number; end: number; raw: string; id: string };

function pickEarlierLink(left: LinkMatch | null, right: LinkMatch | null): LinkMatch | null {
  if (!left) return right;
  if (!right) return left;
  return left.start <= right.start ? left : right;
}

function nextMarkdownImageLink(input: string, from: number): LinkMatch | null {
  MARKDOWN_IMAGE_LINK_PATTERN.lastIndex = from;
  const match = MARKDOWN_IMAGE_LINK_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "image_link",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    alt: match[1],
    src: match[2],
    href: match[3],
  };
}

function nextMarkdownLink(input: string, from: number): LinkMatch | null {
  MARKDOWN_LINK_PATTERN.lastIndex = from;
  const match = MARKDOWN_LINK_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "markdown",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    text: match[1],
    href: match[2],
    image: match[0].startsWith("!"),
  };
}

function nextAutoLink(input: string, from: number): LinkMatch | null {
  URL_PATTERN.lastIndex = from;
  const match = URL_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "auto",
    start: match.index,
    end: match.index + match[0].length,
    href: match[0],
  };
}

function nextToolcallRef(input: string, from: number): LinkMatch | null {
  TOOLCALL_REF_PATTERN.lastIndex = from;
  const match = TOOLCALL_REF_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "toolcall_ref",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    id: String(match[1] || "").trim(),
  };
}

function nextFootnoteRef(input: string, from: number): LinkMatch | null {
  FOOTNOTE_REF_PATTERN.lastIndex = from;
  const match = FOOTNOTE_REF_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "footnote_ref",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    id: String(match[1] || "").trim(),
  };
}

function toolcallRefLabel(id: string): string {
  const normalized = String(id || "").trim();
  if (!normalized) return "?";
  const numericSuffix = normalized.match(/(\d+)(?!.*\d)/);
  return numericSuffix?.[1] || normalized;
}

function normalizeMarkdownHref(rawHref: string): string {
  let href = String(rawHref || "").trim();
  if (!href) return "";
  const titleMatch = href.match(/^(.+?)\s+["'][^"']*["']$/);
  if (titleMatch) href = titleMatch[1].trim();
  if (href.startsWith("<") && href.endsWith(">")) {
    href = href.slice(1, -1).trim();
  }
  return href;
}

type EmphasisMarkerType = "strong" | "em" | "strongEm" | "delete";
type InlineSyntaxMatch =
  | { kind: "html_br"; start: number; end: number; raw: string }
  | { kind: "html_tag"; start: number; end: number; raw: string; tag: "sub" | "sup" | "kbd" | "mark"; inner: string }
  | { kind: "code"; start: number; end: number; raw: string; text: string }
  | { kind: "math"; start: number; end: number; raw: string; text: string; display: boolean }
  | { kind: "emphasis"; start: number; end: number; inner: string; emphasisType: EmphasisMarkerType }
  | LinkMatch;

type ProtectedInlineRange = { start: number; end: number };

function pickEarlierInline(left: InlineSyntaxMatch | null, right: InlineSyntaxMatch | null): InlineSyntaxMatch | null {
  if (!left) return right;
  if (!right) return left;
  return left.start <= right.start ? left : right;
}

function isIndexInProtectedRange(index: number, ranges: ProtectedInlineRange[]): boolean {
  return ranges.some((range) => index >= range.start && index < range.end);
}

function collectProtectedInlineRanges(text: string): ProtectedInlineRange[] {
  const ranges: ProtectedInlineRange[] = [];
  let cursor = 0;
  while (cursor < text.length) {
    const match = nextCodeOrMath(text, cursor);
    if (!match) break;
    ranges.push({ start: match.start, end: match.end });
    cursor = Math.max(match.end, match.start + 1);
  }
  return ranges;
}

function findMarkerOutsideProtectedRanges(
  text: string,
  marker: string,
  from: number,
  protectedRanges: ProtectedInlineRange[],
): number {
  let cursor = from;
  while (cursor < text.length) {
    const index = text.indexOf(marker, cursor);
    if (index < 0) return -1;
    if (!isIndexInProtectedRange(index, protectedRanges)) return index;
    cursor = index + marker.length;
  }
  return -1;
}

function findNextInlineMarker(
  text: string,
  from: number,
): { type: EmphasisMarkerType; start: number; end: number; inner: string } | null {
  const protectedRanges = collectProtectedInlineRanges(text);
  const patterns: Array<{ type: EmphasisMarkerType; marker: string }> = [
    { type: "strongEm", marker: "***" },
    { type: "delete", marker: "~~" },
    { type: "strong", marker: "**" },
    { type: "em", marker: "*" },
  ];
  let best: { type: EmphasisMarkerType; start: number; end: number; inner: string } | null = null;
  for (const pattern of patterns) {
    const start = findMarkerOutsideProtectedRanges(text, pattern.marker, from, protectedRanges);
    if (start < 0) continue;
    if (pattern.marker === "*" && text[start + 1] === "*") continue;
    const contentStart = start + pattern.marker.length;
    const endMarker = findMarkerOutsideProtectedRanges(text, pattern.marker, contentStart, protectedRanges);
    if (endMarker < 0 || endMarker === contentStart) continue;
    const candidate = {
      type: pattern.type,
      start,
      end: endMarker + pattern.marker.length,
      inner: text.slice(contentStart, endMarker),
    };
    if (!best || candidate.start < best.start || (candidate.start === best.start && candidate.end > best.end)) {
      best = candidate;
    }
  }
  return best;
}

function nextInlineCode(input: string, from: number): InlineSyntaxMatch | null {
  const codePattern = /`([^`]+)`/g;
  codePattern.lastIndex = from;
  const match = codePattern.exec(input);
  if (!match) return null;
  return {
    kind: "code",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    text: match[1],
  };
}

function inlineMathCanRender(text: string): boolean {
  const content = normalizeMathText(text);
  if (!content) return false;
  return !/[\r\n]/.test(content);
}

function nextInlineMath(input: string, from: number): InlineSyntaxMatch | null {
  let cursor = Math.max(0, from);
  while (cursor < input.length) {
    const start = findUnescapedDelimiter(input, "$", cursor);
    if (start < 0) return null;
    if (input[start + 1] === "$" || input[start - 1] === "$") {
      cursor = start + 1;
      continue;
    }
    const contentStart = start + 1;
    const end = findUnescapedDelimiter(input, "$", contentStart);
    if (end < 0) return null;
    if (input[end + 1] === "$") {
      cursor = start + 1;
      continue;
    }
    const raw = input.slice(start, end + 1);
    const text = input.slice(contentStart, end);
    if (!inlineMathCanRender(text)) {
      cursor = start + 1;
      continue;
    }
    return {
      kind: "math",
      start,
      end: end + 1,
      raw,
      text,
      display: false,
    };
  }
  return null;
}

function nextCodeOrMath(input: string, from: number): InlineSyntaxMatch | null {
  return pickEarlierInline(nextInlineCode(input, from), nextInlineMath(input, from));
}

function nextAllowedHtml(input: string, from: number): InlineSyntaxMatch | null {
  const brPattern = /<br\s*\/?>/ig;
  brPattern.lastIndex = from;
  const brMatch = brPattern.exec(input);

  const tagPattern = /<(sub|sup|kbd|mark)>([\s\S]*?)<\/\1>/ig;
  tagPattern.lastIndex = from;
  const tagMatch = tagPattern.exec(input);

  const brCandidate = brMatch
    ? {
      kind: "html_br" as const,
      start: brMatch.index,
      end: brMatch.index + brMatch[0].length,
      raw: brMatch[0],
    }
    : null;
  const tagCandidate = tagMatch
    ? {
      kind: "html_tag" as const,
      start: tagMatch.index,
      end: tagMatch.index + tagMatch[0].length,
      raw: tagMatch[0],
      tag: tagMatch[1].toLowerCase() as "sub" | "sup" | "kbd" | "mark",
      inner: tagMatch[2],
    }
    : null;

  return pickEarlierInline(brCandidate, tagCandidate);
}

function nextLinkLike(input: string, from: number): LinkMatch | null {
  const imageLink = nextMarkdownImageLink(input, from);
  const markdownLink = nextMarkdownLink(input, from);
  const autoLink = nextAutoLink(input, from);
  const toolcallRef = nextToolcallRef(input, from);
  const footnoteRef = nextFootnoteRef(input, from);
  return pickEarlierLink(
    pickEarlierLink(pickEarlierLink(pickEarlierLink(imageLink, markdownLink), autoLink), toolcallRef),
    footnoteRef,
  );
}

function nextEmphasis(input: string, from: number): InlineSyntaxMatch | null {
  const matched = findNextInlineMarker(input, from);
  if (!matched) return null;
  return {
    kind: "emphasis",
    start: matched.start,
    end: matched.end,
    inner: matched.inner,
    emphasisType: matched.type,
  };
}

function emitInlineSyntaxMatch(match: InlineSyntaxMatch, segments: InlineSegment[]) {
  if (match.kind === "html_br") {
    segments.push({ type: "html_br" });
    return;
  }
  if (match.kind === "html_tag") {
    const children = parseInlineSegments(match.inner);
    if (match.tag === "sub") {
      segments.push({ type: "html_sub", children });
      return;
    }
    if (match.tag === "sup") {
      segments.push({ type: "html_sup", children });
      return;
    }
    if (match.tag === "kbd") {
      segments.push({ type: "html_kbd", children });
      return;
    }
    segments.push({ type: "html_mark", children });
    return;
  }
  if (match.kind === "code") {
    segments.push({ type: "code", text: match.text });
    return;
  }
  if (match.kind === "math") {
    segments.push({ type: "math", text: normalizeMathText(match.text), raw: match.raw, display: match.display });
    return;
  }
  if (match.kind === "emphasis") {
    segments.push({
      type: match.emphasisType,
      children: parseInlineSegments(match.inner),
    } as InlineSegment);
    return;
  }
  if (match.kind === "image_link") {
    const src = normalizeMarkdownHref(match.src);
    const href = normalizeMarkdownHref(match.href);
    if (src && href) {
      segments.push({ type: "imageLink", src, href, alt: match.alt });
    } else {
      pushTextSegment(segments, match.raw);
    }
    return;
  }
  if (match.kind === "markdown") {
    const href = normalizeMarkdownHref(match.href);
    if (href) {
      if (match.image) {
        segments.push({ type: "image", src: href, alt: match.text });
      } else {
        segments.push({ type: "link", href, text: match.text });
      }
    } else {
      pushTextSegment(segments, match.raw);
    }
    return;
  }
  if (match.kind === "auto") {
    const { href, trailing } = trimTrailingUrlPunctuation(match.href);
    if (href) segments.push({ type: "link", href, text: href });
    pushTextSegment(segments, trailing);
    return;
  }
  if (match.kind === "toolcall_ref") {
    segments.push({
      type: "toolcall_ref",
      id: match.id,
      label: toolcallRefLabel(match.id),
    });
    return;
  }
  if (match.id) {
    segments.push({ type: "footnote_ref", id: match.id });
  } else {
    pushTextSegment(segments, match.raw);
  }
}

/**
 * Parse inline text into segments, handling: inline code, inline math ($...$),
 * markdown links, auto-links, bold, italic, bold-italic, strikethrough.
 */
export function parseInlineSegments(input: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  const text = String(input || "");
  let cursor = 0;

  while (cursor < text.length) {
    const next = pickEarlierInline(
      pickEarlierInline(
        pickEarlierInline(nextCodeOrMath(text, cursor), nextAllowedHtml(text, cursor)),
        nextLinkLike(text, cursor),
      ),
      nextEmphasis(text, cursor),
    );
    if (!next) break;
    pushTextSegment(segments, text.slice(cursor, next.start));
    emitInlineSyntaxMatch(next, segments);
    cursor = next.end;
  }
  pushTextSegment(segments, text.slice(cursor));
  return segments;
}

export function normalizedTableRow(row: string[], size: number): string[] {
  return Array.from({ length: Math.max(1, size) }, (_item, index) => row[index] || "");
}
