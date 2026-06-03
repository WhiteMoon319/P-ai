export function textContentSignature(text?: string): string {
  const input = String(text || "");
  let hash = 0x811c9dc5;
  for (let index = 0; index < input.length; index += 1) {
    hash ^= input.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `${input.length}:${(hash >>> 0).toString(36)}`;
}
