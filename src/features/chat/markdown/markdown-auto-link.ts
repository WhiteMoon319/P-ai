export type MarkdownAutoLinkMatch = {
  start: number;
  end: number;
  href: string;
};

const AUTO_LINK_PATTERN = /<((?:(?:https?:\/\/|file:\/\/\/)[^\s<>\r\n]+|(?:[A-Za-z]:[\\/]|\\\\)[^<>\r\n]+))>|(?<!<)(https?:\/\/[^\s<>()]+|file:\/\/\/[^\s<>()]+)/g;

export function findNextMarkdownAutoLink(input: string, from: number): MarkdownAutoLinkMatch | null {
  AUTO_LINK_PATTERN.lastIndex = from;
  const match = AUTO_LINK_PATTERN.exec(input);
  if (!match) return null;

  const href = String(match[1] || match[2] || "").trim();
  return {
    start: match.index,
    end: match.index + match[0].length,
    href,
  };
}
