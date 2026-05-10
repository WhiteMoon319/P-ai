import { preloadExtendedLanguageIcons, setCustomComponents } from "markstream-vue";
import FileReaderCodeBlockNode from "./FileReaderCodeBlockNode.vue";

let registered = false;

export function registerFileReaderMarkstreamComponents() {
  if (registered) return;

  setCustomComponents("file-reader-markstream", {
    code_block: FileReaderCodeBlockNode,
  });
  void preloadExtendedLanguageIcons();
  registered = true;
}
