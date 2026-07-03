# 未发布

## 功能

- 升级 genai 到 0.7.0-beta.9，并接入 ProviderConfig 模型列表刷新、adapter 绑定与 Responses response_id 元数据保存。

## 维护

- 升级 daisyui 到 5.6.10。
- 升级 Vue 到 3.5.39。
- 升级 shiki 到 4.3.0。
- 升级 @tauri-apps/api 到 2.11.1。
- 升级 tailwindcss 到 4.3.2。
- 升级 @tailwindcss/postcss 到 4.3.2。
- 升级 postcss 到 8.5.16。
- 升级 Vite 到 8.1.3。
- 升级 Vitest 到 4.1.9。
- 升级 vue-tsc 到 3.3.6。
- 升级 mermaid 到 11.16.0。
- 升级 vue-i18n 到 11.4.6。
- 升级 @intlify/devtools-types 到 11.4.6。
- 升级 @tanstack/vue-virtual 到 3.13.31。
- 升级 @tauri-apps/cli 到 2.11.4。
- 升级 @tailwindcss/typography 到 0.5.20。
- 升级 type-fest 到 5.7.0。
- 升级 @lucide/vue 到 1.23.0。
- 升级 Tauri global shortcut 插件到 2.3.2。
- 升级 serde_json 到 1.0.150。
- 升级 chrono 到 0.4.45。
- 升级 uuid 到 1.23.4。
- 升级 reqwest 到 0.13.4。
- 升级 time 到 0.3.53。
- 升级 socket2 到 0.6.4。
- 升级 xcap 到 0.9.6。
- 升级 tauri-build 到 2.6.3。

## 修复

- 修复 APP 强制切入被占用会话后，被接管窗口仍停留在原会话的问题；被接管方会自动退回系统会话，且系统会话不再参与占用锁定。
- 修复流式思维链计数器在增量输出时反复重新汇总已累计内容的问题，改为随 reasoning delta 增量更新计数。
