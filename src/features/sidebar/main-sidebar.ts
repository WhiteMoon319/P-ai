// 侧栏复用 APP 的主聊天入口。连接与文件选择器等边界能力由 tauri-api
// 统一处理，不能在这里复制聊天状态机。
import "./assets/sidebar-theme.css";
import "../../main-chat";

// 宿主标记必须在主入口挂载前设置，sidebar-theme.css 按 data-host 分流：
// vscode 走宿主 CSS 变量跟随 VSCode 主题，web 走应用主题。
const bridgeWindow = window as Window & { acquireVsCodeApi?: unknown };
if (
  typeof bridgeWindow.acquireVsCodeApi === "function"
  || window.location.protocol === "vscode-webview:"
) {
  document.documentElement.setAttribute("data-host", "vscode");
} else {
  document.documentElement.setAttribute("data-host", "web");
}

