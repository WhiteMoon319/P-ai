import type { IdeContextReferenceItem } from "../../../types/app";
import { formatLineSuffix, titleFromPath } from "../../file-reader/utils";

export type IdeContextReferenceDisplayParts = {
  fileName: string;
  lineSuffix: string;
};

export function ideContextReferenceDisplayParts(
  item: Pick<IdeContextReferenceItem, "fileName" | "filePath" | "relativePath" | "displayLabel" | "startLine" | "endLine">,
): IdeContextReferenceDisplayParts {
  const lineSuffix = formatLineSuffix(item.startLine, item.endLine);
  const displayLabel = String(item.displayLabel || "").trim();
  const displayPath = lineSuffix && displayLabel.endsWith(lineSuffix)
    ? displayLabel.slice(0, -lineSuffix.length)
    : displayLabel;
  const sourcePath = String(
    item.fileName
      || item.relativePath
      || item.filePath
      || displayPath,
  ).trim();
  return {
    fileName: titleFromPath(sourcePath),
    lineSuffix,
  };
}
