import { acknowledgeTransportWebviewHeartbeat, onTransportNotification } from "./services/tauri-api";

onTransportNotification("webview.ping", () => {
  acknowledgeTransportWebviewHeartbeat().catch(() => {});
});
