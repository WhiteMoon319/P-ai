const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const vscode = require("vscode");

const DISCOVERY_FILE = "p-ai-ide-context-bridge.json";

function readDiscovery() {
  const discoveryPath = path.join(os.tmpdir(), DISCOVERY_FILE);
  try {
    const raw = fs.readFileSync(discoveryPath, "utf8");
    const parsed = JSON.parse(raw);
    const chatUrl = String(parsed.chatUrl || parsed.url || "").replace(/\/ide-context$/, "/chat");
    const token = String(parsed.token || "");
    if (!chatUrl || !token) return null;
    return { ...parsed, chatUrl, token };
  } catch {
    return null;
  }
}

function readWorkspaceRoots() {
  return (vscode.workspace.workspaceFolders || []).map((folder) => ({
    path: folder.uri.fsPath,
    name: folder.name,
  }));
}

function nonce() {
  return Array.from({ length: 16 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
}

function readSidebarAssets(extensionUri) {
  const candidates = [
    path.join(extensionUri.fsPath, "dist"),
    path.resolve(extensionUri.fsPath, "..", "..", "..", "..", "dist"),
  ];
  const sidebarDistPath = candidates.find((dir) => fs.existsSync(path.join(dir, "sidebar.html")));
  if (!sidebarDistPath) return { scripts: [], styles: [], localResourceRoots: [] };
  const sidebarHtmlPath = path.join(sidebarDistPath, "sidebar.html");
  try {
    const html = fs.readFileSync(sidebarHtmlPath, "utf8");
    const scripts = Array.from(html.matchAll(/<script[^>]+src="([^"]+\.js)"[^>]*>/g))
      .map((match) => match[1]);
    const styles = Array.from(html.matchAll(/<link[^>]+href="([^"]+\.css)"[^>]*>/g))
      .map((match) => match[1]);
    const distUri = vscode.Uri.file(sidebarDistPath);
    return {
      scripts: scripts.map((asset) => vscode.Uri.joinPath(distUri, asset.replace(/^\//, ""))),
      styles: styles.map((asset) => vscode.Uri.joinPath(distUri, asset.replace(/^\//, ""))),
      localResourceRoots: [distUri],
    };
  } catch {
    return { scripts: [], styles: [], localResourceRoots: [] };
  }
}

function safeDecode(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function normalizeLocalReference(rawHref) {
  let href = String(rawHref || "").trim();
  if (!href) return "";
  if ((href.startsWith("<") && href.endsWith(">")) || (href.startsWith("\"") && href.endsWith("\""))) {
    href = href.slice(1, -1).trim();
  }
  if (/^https?:\/\//i.test(href)) return "";
  if (/^file:/i.test(href)) {
    try {
      const url = new URL(href);
      const decodedPath = safeDecode(url.pathname || "");
      if (url.host && url.host !== "localhost") {
        return `\\\\${url.host}${decodedPath.replace(/\//g, "\\")}`;
      }
      return decodedPath.replace(/^\/([A-Za-z]:)/, "$1");
    } catch {
      return safeDecode(href);
    }
  }
  return safeDecode(href).replace(/%5C/gi, "\\");
}

function parseLocalFileReference(rawHref) {
  const normalized = normalizeLocalReference(rawHref);
  if (!normalized) return null;
  const match = normalized.match(/^(.*?)(?::(\d+))(?::(\d+))?$/);
  const filePath = (match ? match[1] : normalized).trim();
  if (!/^[A-Za-z]:[\\/]/.test(filePath) && !filePath.startsWith("\\\\") && !filePath.startsWith("/")) {
    return null;
  }
  const line = match && match[2] ? Math.max(Number.parseInt(match[2], 10), 1) : null;
  const column = match && match[3] ? Math.max(Number.parseInt(match[3], 10), 1) : null;
  return { filePath, line, column };
}

async function openLocalFileReference(rawHref) {
  const reference = parseLocalFileReference(rawHref);
  if (!reference) {
    void vscode.window.showWarningMessage("无法识别文件引用。");
    return;
  }
  const uri = vscode.Uri.file(reference.filePath);
  const options = { preview: true };
  if (reference.line !== null) {
    const position = new vscode.Position(reference.line - 1, (reference.column || 1) - 1);
    options.selection = new vscode.Range(position, position);
  }
  try {
    await vscode.window.showTextDocument(uri, options);
  } catch (error) {
    if (reference.line !== null) throw error;
    await vscode.commands.executeCommand("vscode.open", uri);
  }
}

class PaiSidebarProvider {
  constructor(extensionUri) {
    this.extensionUri = extensionUri;
    this.view = null;
  }

  resolveWebviewView(webviewView) {
    this.view = webviewView;
    const webview = webviewView.webview;
    const assets = readSidebarAssets(this.extensionUri);
    webview.options = {
      enableScripts: true,
      localResourceRoots: [
        ...assets.localResourceRoots,
      ],
    };
    webview.onDidReceiveMessage((message) => {
      if (message && message.type === "pai-refresh-discovery") {
        this.postDiscovery();
        return;
      }
      if (message && message.type === "pai-open-file") {
        openLocalFileReference(message.href).catch((error) => {
          const detail = error && error.message ? error.message : String(error);
          void vscode.window.showWarningMessage(`打开文件引用失败：${detail}`);
        });
      }
    });
    webview.html = this.html(webview, assets);
    this.postDiscovery();
  }

  postDiscovery() {
    const discovery = readDiscovery();
    if (this.view) {
      void this.view.webview.postMessage({
        type: "pai-discovery",
        discovery: discovery ? { ...discovery, workspaceRoots: readWorkspaceRoots() } : discovery,
      });
    }
  }

  html(webview, preloadedAssets) {
    const scriptNonce = nonce();
    const assets = preloadedAssets || readSidebarAssets(this.extensionUri);
    const scriptUris = assets.scripts.map((asset) => webview.asWebviewUri(asset).toString());
    const styleUris = assets.styles.map((asset) => webview.asWebviewUri(asset).toString());
    const discovery = JSON.stringify({ ...(readDiscovery() || {}), workspaceRoots: readWorkspaceRoots() });
    return `<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; connect-src ws://127.0.0.1:*; img-src ${webview.cspSource} data:; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${scriptNonce}' 'wasm-unsafe-eval';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  ${styleUris.map((uri) => `<link href="${uri}" rel="stylesheet">`).join("\n  ")}
  <title>PAI</title>
</head>
<body>
  <div id="app"></div>
  <script nonce="${scriptNonce}">window.__PAI_SIDEBAR_BRIDGE__ = ${discovery};</script>
  ${
    scriptUris.length > 0
      ? scriptUris.map((uri) => `<script nonce="${scriptNonce}" type="module" src="${uri}"></script>`).join("\n  ")
      : "<p>Sidebar assets are missing. Run the sidebar build first.</p>"
  }
</body>
</html>`;
  }
}

function activate(context) {
  const provider = new PaiSidebarProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider("paiSidebar.chatView", provider, {
      webviewOptions: { retainContextWhenHidden: true },
    }),
    vscode.commands.registerCommand("paiSidebar.refresh", () => provider.postDiscovery()),
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
