# 未发布

- 新增（android-workspace）：Android 工具页复用“PAI 助理空间”卡片提供可选沙盒工作区初始化，下载 Ubuntu Base 24.04.3 arm64 rootfs（约 28.5 MiB）、校验 SHA256、解压到 `runtime/linux` 并展示下载进度；未就绪时拦截文件、终端、配置、补丁、操作与 MCP 类工具，ready 后将读写、终端 cwd 和 MCP stdio cwd 收敛到应用私有沙盒。

- 新增（android-workspace）：Android 沙盒工作区在工具页提供独立文件管理器入口，可在独立界面浏览用户文件、导入到当前目录、选中文件导出或删除；文件管理器后端强制限制在沙盒用户文件区，屏蔽 `runtime/`、`tmp/`、`.pai`、`mcp/`、`skills/`、`private-organization/`、`avatars/`、`media/` 与应用数据文件等系统目录。
- 优化（android-workspace）：Android 沙盒文件管理器在移动端改为全屏安全区布局，工具栏在小屏使用两列按钮，并接管系统返回键，优先关闭删除确认、返回上级目录，再关闭文件管理器。
- 优化（android-workspace）：Android 沙盒工作区初始化下载增加连接与无进展超时，避免长期停留在“连接下载源”；超时会回写明确错误并输出日志，便于区分 DNS、握手和下载体卡点。
- 修复（android-workspace）：Android Linux rootfs 解压兼容 Ubuntu Base 中的硬链接条目，硬链接创建失败时按 RikkaHub 同类实现回退为复制源文件；解压改为 staging 目录，成功后再替换 `runtime/linux`，避免失败重试时被半截运行环境污染。
- 修复（android-workspace）：Android Linux rootfs 在线下载的独立 HTTP client 在 Android 端套用静态 WebPKI 根证书，避免 `rustls-platform-verifier` 未初始化导致连接阶段 panic；同时新增 Ubuntu Base 压缩包手动导入兜底，用户可自行下载 `ubuntu-base-24.04.3-base-arm64.tar.gz` 后在工具页导入，后端会校验大小与 SHA256 再解压置为 ready。
- 修复（android-log）：Android 后端文件日志在拿到应用数据目录后再解析并缓存 `backend.log` 路径，避免启动早期把日志路径永久缓存为空；Android 调试输出统一走运行日志通道，兼顾 `adb logcat` 与文件日志。
