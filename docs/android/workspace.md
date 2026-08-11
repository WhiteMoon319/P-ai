# Android 工作区（workspace / rootfs / proot）

## 结构

- 工作区根：`<dataDir>/llm-workspace`
- Linux 运行环境：`<dataDir>/runtime/android-workspace/default`（rootfs 解压目标）
- 状态：`android_workspace_status`（state / download bytes / stage）

## 初始化流程

1. `get_android_workspace_status` → 检查就绪（usr/bin/dash 存在）
2. 未就绪 → `init_android_workspace`：
   - 下载 rootfs（`ubuntu-base-24.04.3-base-arm64.tar.gz`，约 29.8MB，见
     `third_party/android/rootfs/manifest.json`）
   - SHA256 校验（`7b2dced6...dabb048`）
   - 解压到 staging 目录（拒绝绝对路径/`..`/非法 symlink/hardlink）
   - 校验 usr/bin/dash 后 rename 到 runtime 根
3. `repair_android_workspace_runtime`：补 proot 依赖目录、patch rootfs、写 marker
4. `reset_android_workspace_runtime/state`：重置运行环境或整个工作区

## proot

- 二进制：`libproot_exec.so`（Termux proot，patchelf 修正 soname/rpath=$ORIGIN）
- loader：`libproot_loader.so`
- 依赖：`libtalloc.so`、`libandroid-shmem.so`
- 来源与版本：`third_party/android/proot/manifest.json`

## 安全约束

- rootfs 解压路径解析：拒绝 `RootDir`/`ParentDir`/`Prefix`（绝对路径与 `..`）
- symlink 目标必须解析后仍在 root 内（`android_workspace_rootfs_resolve_symlink_target`）
- 文件导入导出：64MiB 上限，decode/read 前预检大小
- 路径访问经沙盒与 symlink 防护

## 文件管理 RPC

`android_workspace.list / readText / writeText / move / delete / import / export / glob / grep`
（详见 `contracts/native-rpc/methods.json`）。

## 长任务

workspace init/repair/reset/rootfs 导入为长任务（callLong 600s 或任务状态机），
进度经 `get_android_workspace_status` 轮询；失败带可读错误，不静默。
