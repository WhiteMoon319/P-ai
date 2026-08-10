include!("sandbox/types.rs");
include!("sandbox/policy.rs");
include!("sandbox/backend_common.rs");
include!("sandbox/backend_process.rs");
#[cfg(target_os = "android")]
mod android_rootfs_runner {
    include!("sandbox/android_rootfs/runner.rs");
}
#[cfg(target_os = "android")]
mod android_rootfs_patcher {
    include!("sandbox/android_rootfs/patcher.rs");
}
#[cfg(target_os = "android")]
use android_rootfs_runner::*;
#[cfg(target_os = "android")]
use android_rootfs_patcher::*;
include!("sandbox/manager.rs");
