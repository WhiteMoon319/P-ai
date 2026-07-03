import { bundledLanguagesInfo } from "shiki";

export const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

export const CODE_LANGUAGE_BY_EXTENSION: Record<string, string> = {
  ts: "typescript", tsx: "tsx", c: "c", cc: "cpp", cpp: "cpp", cxx: "cpp",
  h: "c", hpp: "cpp", cs: "csharp", java: "java", kt: "kotlin", kts: "kotlin",
  go: "go", js: "javascript", jsx: "jsx", vue: "vue", rs: "rust", py: "python",
  rb: "ruby", php: "php", swift: "swift", scala: "scala", dart: "dart", lua: "lua",
  r: "r", m: "objective-c", mm: "objective-cpp", pl: "perl", pm: "perl",
  json: "json", jsonc: "jsonc", json5: "json5", toml: "toml", yaml: "yaml", yml: "yaml",
  css: "css", scss: "scss", sass: "sass", less: "less", html: "html", htm: "html",
  xml: "xml", svg: "xml", sql: "sql", sh: "bash", bash: "bash", zsh: "bash",
  fish: "fish", ps1: "powershell", bat: "bat", cmd: "bat", dockerfile: "dockerfile",
  ini: "ini", env: "dotenv", gitignore: "gitignore", gitattributes: "gitignore",
  editorconfig: "ini", lock: "text", csv: "csv", tsv: "tsv", txt: "text", log: "log",
  md: "markdown", markdown: "markdown", mdx: "mdx",
};

export const CONTEXT_TEXT_BLOCK_CONTENT_LIMIT = 2000;
export const FILE_READER_VIRTUAL_BLOCK_OVERSCAN = 6;
export const FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX = 24;
export const FILE_READER_VIRTUAL_BLOCK_PADDING_Y_PX = 8;
