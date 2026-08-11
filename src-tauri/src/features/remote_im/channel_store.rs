// channel_store 域已整体迁入 pai-android-platform（远程 IM 渠道私有状态存储）。
// 本文件仅作 src-tauri 桥接 re-export + AppState→ctx 适配。

pub(crate) use pai_android_platform::remote_im::channel_store::*;

use super::*;

pub(crate) fn remote_im_channel_store_ctx_from_state(
    state: &AppState,
) -> pai_android_platform::remote_im::channel_store::RemoteImChannelStoreCtx<'_> {
    pai_android_platform::remote_im::channel_store::RemoteImChannelStoreCtx {
        data_path: &state.data_path,
        write_locks: &state.remote_im_channel_state_write_locks,
    }
}
