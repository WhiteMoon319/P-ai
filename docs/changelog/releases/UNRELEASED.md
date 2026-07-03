# 未发布

## 功能

- 升级 genai 到 0.7.0-beta.9，并接入 ProviderConfig 模型列表刷新、adapter 绑定与 Responses response_id 元数据保存。

## 维护

- 升级 daisyui 到 5.6.10。
- 升级 Vue 到 3.5.39。
- 升级 shiki 到 4.3.0。
- 升级 @tauri-apps/api 到 2.11.1。

## 修复

- 修复 APP 强制切入被占用会话后，被接管窗口仍停留在原会话的问题；被接管方会自动退回系统会话，且系统会话不再参与占用锁定。
- 修复流式思维链计数器在增量输出时反复重新汇总已累计内容的问题，改为随 reasoning delta 增量更新计数。
