//! P-AI Android 平台能力（阶段 5）。
//!
//! 职责：Android 工作区 / rootfs / proot / TLS / 沙盒等平台相关逻辑，
//! 不依赖 tauri / jni；由 pai-android-bridge（阶段 6）与 Android 宿主接入。
//!
//! 迁移策略：从 src-tauri 逐步搬入零 tauri 依赖的平台模块。

pub mod android_workspace;
pub mod remote_im;
pub mod local_port_service;
pub mod remote_im_sdk;
pub mod sandbox;
pub mod tls;
pub mod chat;
pub mod delegate;
