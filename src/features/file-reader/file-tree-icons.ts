import { titleFromPath } from "./utils";

const iconModules = import.meta.glob("./assets/material-icons/*.svg", {
  eager: true,
  import: "default",
  query: "?url",
}) as Record<string, string>;

const iconUrlByName = Object.fromEntries(
  Object.entries(iconModules).map(([path, url]) => [path.split("/").pop() || "", url]),
);

const FILE_ICON_BY_NAME: Record<string, string> = {
  "package.json": "npm.svg",
  "pnpm-lock.yaml": "pnpm.svg",
  "cargo.toml": "rust.svg",
  "cargo.lock": "rust.svg",
  ".gitignore": "git.svg",
  ".gitattributes": "git.svg",
  ".editorconfig": "file.svg",
  "readme.md": "markdown.svg",
  "readme.mdx": "markdown.svg",
};

const FILE_ICON_BY_EXTENSION: Record<string, string> = {
  ts: "typescript.svg",
  tsx: "react_ts.svg",
  js: "javascript.svg",
  jsx: "react.svg",
  vue: "vue.svg",
  rs: "rust.svg",
  md: "markdown.svg",
  mdx: "markdown.svg",
  json: "json.svg",
  yaml: "yaml.svg",
  yml: "yaml.svg",
  toml: "toml.svg",
  html: "html.svg",
  htm: "html.svg",
  css: "css.svg",
  py: "python.svg",
  xml: "xml.svg",
  svg: "image.svg",
  png: "image.svg",
  jpg: "image.svg",
  jpeg: "image.svg",
  gif: "image.svg",
  webp: "image.svg",
  ico: "image.svg",
  icns: "image.svg",
  zip: "zip.svg",
};

const FOLDER_ICON_BY_NAME: Record<string, { closed: string; open: string }> = {
  src: { closed: "folder-src.svg", open: "folder-src-open.svg" },
  "src-tauri": { closed: "folder-src-tauri.svg", open: "folder-src-tauri-open.svg" },
  docs: { closed: "folder-docs.svg", open: "folder-docs-open.svg" },
  test: { closed: "folder-test.svg", open: "folder-test-open.svg" },
  tests: { closed: "folder-test.svg", open: "folder-test-open.svg" },
  core: { closed: "folder-core.svg", open: "folder-core-open.svg" },
  models: { closed: "folder-core.svg", open: "folder-core-open.svg" },
  media: { closed: "folder-images.svg", open: "folder-images-open.svg" },
  images: { closed: "folder-images.svg", open: "folder-images-open.svg" },
  icons: { closed: "folder-images.svg", open: "folder-images-open.svg" },
  prompts: { closed: "folder-prompts.svg", open: "folder-prompts-open.svg" },
  skills: { closed: "folder-skills.svg", open: "folder-skills-open.svg" },
  scripts: { closed: "folder-scripts.svg", open: "folder-scripts-open.svg" },
  config: { closed: "folder-config.svg", open: "folder-config-open.svg" },
  ".git": { closed: "folder-git.svg", open: "folder-git-open.svg" },
};

const DEFAULT_FILE_ICON = "file.svg";
const DEFAULT_FOLDER_CLOSED_ICON = "folder.svg";
const DEFAULT_FOLDER_OPEN_ICON = "folder-open.svg";

function iconUrl(name: string) {
  return iconUrlByName[name] || "";
}

function rawExtensionFromPath(path: string) {
  const fileName = titleFromPath(path);
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === fileName.length - 1) return "";
  return fileName.slice(dotIndex + 1).toLowerCase();
}

export function resolveFileTreeIcon(path: string, isDirectory: boolean, expanded = false) {
  const lowerName = titleFromPath(path).toLowerCase();

  if (isDirectory) {
    const folderIcons = FOLDER_ICON_BY_NAME[lowerName];
    const iconName = folderIcons ? (expanded ? folderIcons.open : folderIcons.closed) : (expanded ? DEFAULT_FOLDER_OPEN_ICON : DEFAULT_FOLDER_CLOSED_ICON);
    return iconUrl(iconName) || iconUrl(expanded ? DEFAULT_FOLDER_OPEN_ICON : DEFAULT_FOLDER_CLOSED_ICON);
  }

  const namedIcon = FILE_ICON_BY_NAME[lowerName];
  if (namedIcon) return iconUrl(namedIcon) || iconUrl(DEFAULT_FILE_ICON);

  const extension = rawExtensionFromPath(path);
  const extensionIcon = FILE_ICON_BY_EXTENSION[extension];
  return iconUrl(extensionIcon || DEFAULT_FILE_ICON) || "";
}
