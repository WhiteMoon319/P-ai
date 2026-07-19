export type ChatAttachmentKind = "image" | "audio" | "file" | "context" | "text";

export type ChatAttachmentView = {
  id?: string;
  kind: ChatAttachmentKind;
  label: string;
  detail?: string;
  src?: string;
};

export function fileNameFromPath(value: unknown): string {
  const normalized = String(value || "").trim().replace(/\\/g, "/");
  if (!normalized) return "";
  const withoutLineSuffix = normalized.replace(/:\d+(?:-\d+)?$/, "");
  return withoutLineSuffix.split("/").filter(Boolean).pop() || withoutLineSuffix;
}

export function displayFileName(fileName: unknown, path?: unknown): string {
  return fileNameFromPath(fileName) || fileNameFromPath(path) || "attachment";
}

export function displayLabelFromExtraTextReference(text: unknown): string {
  const normalized = String(text || "").trim();
  if (!normalized) return "文件片段";

  const ideFileLine = normalized.match(/^文件:\s*(.+)$/m)?.[1];
  if (ideFileLine) return fileNameFromPath(ideFileLine);

  const translatedReference = normalized.match(/^用户引用了文件片段：([^\n\r（]+)/)?.[1];
  if (translatedReference) return fileNameFromPath(translatedReference);

  return fileNameFromPath(normalized.split("\n")[0]) || "文件片段";
}
