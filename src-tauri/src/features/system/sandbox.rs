// types 与 backend_common 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::sandbox::backend_common::*;
pub(crate) use pai_android_platform::sandbox::types::*;
include!("sandbox/policy.rs");
include!("sandbox/backend_process.rs");
#[cfg(target_os = "android")]
pub(crate) mod android_rootfs_runner {
    include!("sandbox/android_rootfs/runner.rs");
}
// patcher 已迁至 crates/pai-android-platform（阶段 5）。
#[cfg(target_os = "android")]
pub(crate) use pai_android_platform::sandbox::android_rootfs::patcher::*;
#[cfg(target_os = "android")]
use android_rootfs_runner::*;

use std::collections::{HashSet};
use std::path::{PathBuf, Path};
use super::*;
include!("sandbox/manager.rs");
