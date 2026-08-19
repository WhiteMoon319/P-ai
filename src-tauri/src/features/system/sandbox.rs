// types 与 backend_common 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::sandbox::backend_common::*;
pub(crate) use pai_android_platform::sandbox::types::*;
#[path = "sandbox/policy.rs"]
mod sandbox_policy;
pub(crate) use sandbox_policy::*;
#[path = "sandbox/backend_process.rs"]
mod sandbox_backend_process;
pub(crate) use sandbox_backend_process::*;
#[cfg(target_os = "android")]
#[path = "sandbox/android_rootfs/runner.rs"]
mod android_rootfs_runner;
#[cfg(target_os = "android")]
pub(crate) use android_rootfs_runner::*;
// patcher 已迁至 crates/pai-android-platform（阶段 5）。
#[cfg(target_os = "android")]
pub(crate) use pai_android_platform::sandbox::android_rootfs::patcher::*;

use std::collections::{HashSet};
use std::path::{PathBuf, Path};
use super::*;
#[path = "sandbox/manager.rs"]
mod sandbox_manager;
pub(crate) use sandbox_manager::*;
