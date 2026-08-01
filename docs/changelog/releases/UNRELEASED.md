# 未发布

- 新增（android-workspace）：Android 工具页复用“PAI 助理空间”卡片提供可选沙盒工作区初始化，下载 Ubuntu Base 24.04.3 arm64 rootfs（约 28.5 MiB）、校验 SHA256、解压到 `runtime/linux` 并展示下载进度；未就绪时拦截文件、终端、配置、补丁、操作与 MCP 类工具，ready 后将读写、终端 cwd 和 MCP stdio cwd 收敛到应用私有沙盒。

- 新增（android-workspace）：Android 沙盒工作区在工具页提供独立文件管理器入口，可在独立界面浏览用户文件、导入到当前目录、选中文件导出或删除；文件管理器后端强制限制在沙盒用户文件区，屏蔽 `runtime/`、`tmp/`、`.pai`、`mcp/`、`skills/`、`private-organization/`、`avatars/`、`media/` 与应用数据文件等系统目录。
- 优化（android-workspace）：Android 沙盒文件管理器在移动端改为全屏安全区布局，工具栏在小屏使用两列按钮，并接管系统返回键，优先关闭删除确认、返回上级目录，再关闭文件管理器。
- 优化（android-workspace）：Android 沙盒工作区初始化下载增加连接与无进展超时，避免长期停留在“连接下载源”；超时会回写明确错误并输出日志，便于区分 DNS、握手和下载体卡点。
- 修复（android-workspace）：Android Linux rootfs 解压兼容 Ubuntu Base 中的硬链接条目，硬链接创建失败时按 RikkaHub 同类实现回退为复制源文件；解压改为 staging 目录，成功后再替换 `runtime/linux`，避免失败重试时被半截运行环境污染。
- 修复（android-workspace）：Android Linux rootfs 在线下载的独立 HTTP client 在 Android 端套用静态 WebPKI 根证书，避免 `rustls-platform-verifier` 未初始化导致连接阶段 panic；同时新增 Ubuntu Base 压缩包手动导入兜底，用户可自行下载 `ubuntu-base-24.04.3-base-arm64.tar.gz` 后在工具页导入，后端会校验大小与 SHA256 再解压置为 ready。
- 修复（android-log）：Android 后端文件日志在拿到应用数据目录后再解析并缓存 `backend.log` 路径，避免启动早期把日志路径永久缓存为空；Android 调试输出统一走运行日志通道，兼顾 `adb logcat` 与文件日志。

- 新增（android-workspace）：LLM 的 exec 终端命令后端新增 Android 沙盒 Linux 执行路径（`sandbox/backend_android.rs`）。工作区就绪时，通过 proot 将 Ubuntu rootfs 挂载为完整的 Linux 运行环境，沙盒根目录 bind 到 `/workspace`；LLM 可在 `/workspace` 内自由执行命令、安装软件包（如 apt install git）、管理 Git 仓库。启动执行前自动 patch rootfs 的 DNS、hosts、hostname、locale、group 等配置。

- 新增（android-workspace）：Android exec 的 LLM 提示层更新：终端环境块中明确说明当前是 Android 沙盒 Ubuntu Linux 运行环境，/workspace 是默认工作目录，可使用 apt 等标准工具；工具描述中 shell 标注为"Ubuntu 24.04 Linux (proot 沙盒)"。
- 修复（android-workspace）：Android proot 依赖库打包改为将 Termux `libtalloc.so.2` 重命名为 Android 可稳定打包的 `libtalloc.so`，并用 patchelf 同步修补 proot 的 `DT_NEEDED`；同时打包并运行前校验 `libandroid-shmem.so`，避免继续出现 `CANNOT LINK EXECUTABLE`。
- 修复（android-workspace）：Android proot 执行不再依赖 rootfs 内 `/usr/bin/env` 启动命令；参考 RikkaHub 的 rootfs 处理方式，将 rootfs 内绝对符号链接改写为宿主可用的相对链接，并在运行前按 `usr/bin/dash` 自愈 `/bin/sh`；proot 启动环境改为白名单注入，guest `TMPDIR` 固定为 `/tmp`，并隐藏 `/workspace/runtime` 与 `/workspace/tmp/proot`，避免宿主环境变量和内部 rootfs/proot 临时目录污染 exec。
- 修复（android-workspace）：Android 私有人格/私有部门读取根固定到应用私有 `llm-workspace`，reload 路径补齐配置归一化；proot 内同步映射 `/root/.pai`、`/root/.pai/skills`、`/root/.pai/private-organization` 与 `/root/.pai/mcp`，避免初始化 Linux 沙盒后组织与 Skill 读到空目录。
- 修复（runtime）：读取和写入 agents shard 时强制保留默认助理、副手、用户与系统内置人格；部门归一化不再因为 API 配置为空提前返回，运行组织快照与启动配置读取会补齐并写回默认部门，确保内置助理部门等默认部门不会被空配置状态连带隐藏，避免 Android 首启后无法新建或继续对话。
- 修复（runtime）：专家模型绑定 `assistantDepartmentApiConfigId` 无效或为空时自动回退到当前/首个文本聊天模型，前端部门人格选项与当前会话模型解析同步兜底，避免沙盒和供应商配置同时启用后默认部门存在但新建/继续对话判定人格、部门不可用。

- 优化（android-workspace）：Android 文件管理器导入冲突不再直接报错，改为自动生成 `name (1).ext` 风格的重命名文件路径；导入和导出均增加 64 MiB 单文件上限，超限时在前端和后端均返回明确错误。

>

> **Proot 来源：** CI 从 Termux 官方包仓库下载 proot 5.1.107.89 aarch64 及 libtalloc 依赖，用 patchelf 将 rpath 设为 `$ORIGIN` 确保同目录解决动态库依赖。
