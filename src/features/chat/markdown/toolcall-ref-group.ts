import type { InlineSegment, MarkdownBlock } from "./parse-markdown";
import { parseInlineSegments } from "./parse-markdown";

export type GroupedToolcallRefs = {
  ids: string[];
  endIndex: number;
};

export type ParagraphToolLayout = {
  leadingIds: string[];
  trailingIds: string[];
  bodySegments: InlineSegment[];
  allIds: string[];
  markerOnly: boolean;
};

function isWhitespaceOnlyText(segment: InlineSegment | undefined): boolean {
  return segment?.type === "text" && segment.text.trim() === "";
}

function isIgnorableSpacer(segment: InlineSegment | undefined): boolean {
  return isWhitespaceOnlyText(segment);
}

/**
 * 从 startIndex 开始收束同段连续 toolcall_ref。
 * 允许直接相邻，也允许中间只有空白 text；遇到正文或其他 inline 即断组。
 */
export function consumeGroupedToolcallRefs(
  segments: InlineSegment[],
  startIndex: number,
): GroupedToolcallRefs | null {
  const startSegment = segments[startIndex];
  if (startSegment?.type !== "toolcall_ref") return null;

  const ids = [startSegment.id];
  let cursor = startIndex;

  while (cursor + 1 < segments.length) {
    const next = segments[cursor + 1];
    if (next?.type === "toolcall_ref") {
      ids.push(next.id);
      cursor += 1;
      continue;
    }
    if (!isWhitespaceOnlyText(next)) break;
    const afterSpacer = segments[cursor + 2];
    if (afterSpacer?.type !== "toolcall_ref") break;
    ids.push(afterSpacer.id);
    cursor += 2;
  }

  return { ids, endIndex: cursor };
}

function collectToolcallIdsFromSegments(segments: InlineSegment[]): string[] {
  const ids: string[] = [];
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    if (segment.type !== "toolcall_ref") {
      if (isWhitespaceOnlyText(segment)) continue;
      return [];
    }
    const grouped = consumeGroupedToolcallRefs(segments, index);
    if (!grouped) return [];
    ids.push(...grouped.ids);
    index = grouped.endIndex;
  }
  return ids;
}

/**
 * 拆分段落：前导 tool 组 / 正文 segments / 尾部 tool 组。
 * 中间只允许空白；空白不进入 bodySegments。
 */
export function splitParagraphToolLayout(segments: InlineSegment[]): ParagraphToolLayout {
  let start = 0;
  while (start < segments.length && isIgnorableSpacer(segments[start])) start += 1;

  const leadingIds: string[] = [];
  let cursor = start;
  while (cursor < segments.length && segments[cursor]?.type === "toolcall_ref") {
    const grouped = consumeGroupedToolcallRefs(segments, cursor);
    if (!grouped) break;
    leadingIds.push(...grouped.ids);
    cursor = grouped.endIndex + 1;
    while (cursor < segments.length && isIgnorableSpacer(segments[cursor])) cursor += 1;
  }

  let end = segments.length - 1;
  while (end >= cursor && isIgnorableSpacer(segments[end])) end -= 1;

  const trailingIds: string[] = [];
  let trailingStart = end + 1;
  while (end >= cursor && segments[end]?.type === "toolcall_ref") {
    let runStart = end;
    while (runStart > cursor) {
      const prev = segments[runStart - 1];
      if (prev?.type === "toolcall_ref") {
        runStart -= 1;
        continue;
      }
      if (isWhitespaceOnlyText(prev) && segments[runStart - 2]?.type === "toolcall_ref") {
        runStart -= 2;
        continue;
      }
      break;
    }
    const grouped = consumeGroupedToolcallRefs(segments, runStart);
    if (!grouped || grouped.endIndex !== end) break;
    trailingIds.unshift(...grouped.ids);
    trailingStart = runStart;
    end = runStart - 1;
    while (end >= cursor && isIgnorableSpacer(segments[end])) end -= 1;
  }

  const bodyEnd = trailingIds.length > 0 ? trailingStart : segments.length;
  const rawBody = segments.slice(cursor, bodyEnd);
  let bodyStart = 0;
  let bodyStop = rawBody.length;
  while (bodyStart < bodyStop && isIgnorableSpacer(rawBody[bodyStart])) bodyStart += 1;
  while (bodyStop > bodyStart && isIgnorableSpacer(rawBody[bodyStop - 1])) bodyStop -= 1;
  const bodySegments = rawBody.slice(bodyStart, bodyStop);

  const allIds = collectToolcallIdsFromSegments(segments);
  const markerOnly = bodySegments.length === 0 && (leadingIds.length > 0 || trailingIds.length > 0 || allIds.length > 0);

  if (markerOnly) {
    const ids = allIds.length > 0 ? allIds : [...leadingIds, ...trailingIds];
    return {
      leadingIds: ids,
      trailingIds: [],
      bodySegments: [],
      allIds: ids,
      markerOnly: true,
    };
  }

  return {
    leadingIds,
    trailingIds,
    bodySegments,
    allIds: [...leadingIds, ...trailingIds],
    markerOnly: false,
  };
}

export function isMarkerOnlyParagraph(block: MarkdownBlock): boolean {
  if (block.type !== "paragraph") return false;
  return splitParagraphToolLayout(parseInlineSegments(block.text)).markerOnly;
}

export function collectMarkerOnlyParagraphToolcallIds(block: MarkdownBlock): string[] {
  if (block.type !== "paragraph") return [];
  return splitParagraphToolLayout(parseInlineSegments(block.text)).allIds;
}

export function getParagraphToolLayout(block: MarkdownBlock): ParagraphToolLayout | null {
  if (block.type !== "paragraph") return null;
  return splitParagraphToolLayout(parseInlineSegments(block.text));
}

export type CrossParagraphToolGroup = {
  ids: string[];
  endIndex: number;
  mode: "replace" | "trailing";
  stripLeadingOnEnd: boolean;
  startBodySegments: InlineSegment[];
  endBodySegments: InlineSegment[];
};

function appendTrailingToolSegments(body: InlineSegment[], trailingIds: string[]): InlineSegment[] {
  if (trailingIds.length === 0) return body;
  const trailing: InlineSegment[] = trailingIds.flatMap((id, index) => {
    const nodes: InlineSegment[] = [{ type: "toolcall_ref", id, label: id }];
    if (index < trailingIds.length - 1) nodes.push({ type: "text", text: " " });
    return nodes;
  });
  if (body.length === 0) return trailing;
  return [...body, { type: "text", text: " " }, ...trailing];
}

/**
 * 从 startIndex 收束跨换行的连续 toolcall。
 * 1. 连续 marker-only 段落全部并入
 * 2. 上一段尾部 tool 组 + 后续 marker-only + 下一段前导 tool 组 并入同一组
 * 中间只要出现非 tool 正文断组。
 */
export function consumeCrossParagraphToolGroup(
  blocks: MarkdownBlock[],
  startIndex: number,
): CrossParagraphToolGroup | null {
  const startLayout = getParagraphToolLayout(blocks[startIndex]);
  if (!startLayout) return null;

  if (startLayout.markerOnly) {
    const ids = [...startLayout.allIds];
    if (ids.length === 0) return null;
    let cursor = startIndex;
    let stripLeadingOnEnd = false;
    let endBodySegments: InlineSegment[] = [];

    while (cursor + 1 < blocks.length) {
      const nextLayout = getParagraphToolLayout(blocks[cursor + 1]);
      if (!nextLayout) break;
      if (nextLayout.markerOnly) {
        ids.push(...nextLayout.allIds);
        cursor += 1;
        continue;
      }
      if (nextLayout.leadingIds.length > 0) {
        ids.push(...nextLayout.leadingIds);
        stripLeadingOnEnd = true;
        endBodySegments = appendTrailingToolSegments(nextLayout.bodySegments, nextLayout.trailingIds);
        cursor += 1;
      }
      break;
    }

    return {
      ids,
      endIndex: cursor,
      mode: "replace",
      stripLeadingOnEnd,
      startBodySegments: [],
      endBodySegments,
    };
  }

  if (startLayout.trailingIds.length === 0) return null;

  const ids = [...startLayout.trailingIds];
  let cursor = startIndex;
  let stripLeadingOnEnd = false;
  let endBodySegments: InlineSegment[] = [];
  let merged = false;

  while (cursor + 1 < blocks.length) {
    const nextLayout = getParagraphToolLayout(blocks[cursor + 1]);
    if (!nextLayout) break;
    if (nextLayout.markerOnly) {
      ids.push(...nextLayout.allIds);
      cursor += 1;
      merged = true;
      continue;
    }
    if (nextLayout.leadingIds.length > 0) {
      ids.push(...nextLayout.leadingIds);
      stripLeadingOnEnd = true;
      endBodySegments = appendTrailingToolSegments(nextLayout.bodySegments, nextLayout.trailingIds);
      cursor += 1;
      merged = true;
    }
    break;
  }

  if (!merged) return null;

  return {
    ids,
    endIndex: cursor,
    mode: "trailing",
    stripLeadingOnEnd,
    startBodySegments: startLayout.bodySegments,
    endBodySegments,
  };
}

export function consumeGroupedMarkerOnlyParagraphs(
  blocks: MarkdownBlock[],
  startIndex: number,
): GroupedToolcallRefs | null {
  const grouped = consumeCrossParagraphToolGroup(blocks, startIndex);
  if (!grouped || grouped.mode !== "replace") return null;
  return { ids: grouped.ids, endIndex: grouped.endIndex };
}
