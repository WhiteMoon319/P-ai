// ==================== 独立图像生成模块导图 ====================
// 1) config: 供应商/模型配置归一化与端点解析
// 2) types: 独立请求、结果与运行时中间类型
// 3) storage: 图片校验、下载与 Assistant Space 落盘
// 4) providers/comfyui: 云端供应商与本地工作流适配
// 5) edit: 图像编辑输入解析与编辑 payload 适配
// 6) service/commands: 稳定服务入口与 Tauri 命令
include!("image_generation/config.rs");
include!("image_generation/types.rs");
include!("image_generation/storage.rs");
include!("image_generation/providers.rs");
include!("image_generation/edit.rs");
include!("image_generation/comfyui.rs");
include!("image_generation/codex.rs");
include!("image_generation/service.rs");
