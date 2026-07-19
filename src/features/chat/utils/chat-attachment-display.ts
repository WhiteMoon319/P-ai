export type ChatAttachmentKind = "image" | "audio" | "file" | "context" | "text";

export type ChatAttachmentView = {
  id?: string;
  kind: ChatAttachmentKind;
  label: string;
  detail?: string;
  src?: string;
};

export type ExtraTextReferenceDisplayParts = {
  fileName: string;
  lineSuffix: string;
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

export function extraTextReferenceDisplayParts(text: unknown): ExtraTextReferenceDisplayParts {
  const normalized = String(text || "").trim();
  if (!normalized) return { fileName: "文件片段", lineSuffix: "" };

  const ideFileLine = normalized.match(/^文件:\s*(.+)$/m)?.[1];
  if (ideFileLine) {
    const lineText = normalized.match(/^行号:\s*(.+)$/m)?.[1];
    const lineSuffix = String(lineText || "").trim();
    return {
      fileName: fileNameFromPath(ideFileLine) || "文件片段",
      lineSuffix: lineSuffix ? `:${lineSuffix}` : "",
    };
  }

  const translatedReference = normalized.match(/^用户引用了文件片段：([^\n\r（]+)/)?.[1];
  if (translatedReference) {
    const matched = String(translatedReference).trim().match(/^(.*?)(:\d+(?:-\d+)?)?$/);
    return {
      fileName: fileNameFromPath(matched?.[1] || translatedReference) || "文件片段",
      lineSuffix: String(matched?.[2] || "").trim(),
    };
  }

  const firstLine = normalized.split("\n")[0];
  const matched = firstLine.match(/^(.*?)(:\d+(?:-\d+)?)?$/);
  return {
    fileName: fileNameFromPath(matched?.[1] || firstLine) || "文件片段",
    lineSuffix: String(matched?.[2] || "").trim(),
  };
}

export function displayLabelFromExtraTextReference(text: unknown): string {
  const parts = extraTextReferenceDisplayParts(text);
  return `${parts.fileName}${parts.lineSuffix}`.trim();
}
