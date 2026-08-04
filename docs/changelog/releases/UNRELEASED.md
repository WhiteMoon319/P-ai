# 未发布

- 修复（android-model）：Android 端模型列表刷新的 4 条获取路径（OpenAI/Gemini/Anthropic/genai）统一注入静态 WebPKI 根证书，修复 HTTPS 模型列表请求因 Android 无系统根证书校验失败导致模型列表加载不出来的问题（与 Linux rootfs 下载同因）。
- 重构（android-ci）：Android 构建 CI 改为 debug 包构建，触发分支改为 `main`/`dev`；APK 版本号与命名改为 git 派生（`versionCode` = 提交总数，`versionName` = `git describe` 派生），新增 `scripts/patch-android-version.sh` 注入版本逻辑；只上传签名后的 debug APK，重命名为 `P-ai-${GITHUB_REF_NAME}-aarch64.apk` 到 artifact。
- 调整（android-ci）：debug 构建 workflow 仅监听 `dev` 分支（`main` 只由 release tag 流程覆盖）。
- 修复（android-ci）：修复 `patch-android-version.sh` 注入的 Kotlin `Regex` 语法错误（`r"""` 前缀非法，Kotlin 三引号本身即 raw string），导致 Android release 构建失败。
- 新增（android-ci）：新增 Android release CI workflow，`v*` tag 推送或手动触发，release 构建后使用仓库 secrets（`KEYSTORE_BASE64`/`KEYSTORE_PASSWORD`/`KEY_ALIAS`/`KEY_PASSWORD`）签名并 verify，发布签名 APK 到 GitHub Release。
- 文档（android）：README 改写为 Android 移植版说明，突出桌面版原仓库 `kawayiYokami/P-ai`。
- 新增（android-workspace）：Android 工具页复用“PAI 助理空间”卡片提供可选 Linux 运行环境初始化，下载 Ubuntu Base 24.04.3 arm64 rootfs（约 28.5 MiB）、校验 SHA256、解压到 `runtime/android-workspace/default/linux` 并展示下载进度；未就绪时仅拦截终端与 MCP 类工具，文件、配置、补丁与操作类工具仍可维护助理空间，ready 后将终端 cwd 和 MCP stdio cwd 收敛到 `/workspace`。
- 重构（android-workspace）：参考 RikkaHub workspace 思路，Android 端将应用私有 `llm-workspace` 整根同时 bind 到 Linux 内的 `/workspace` 与 `/root/.pai`，rootfs、下载缓存、staging 与 proot 临时目录迁出 `llm-workspace`，放入应用私有 runtime，避免工作区 bind 自引用和运行时文件污染用户空间。

- 新增（android-workspace）：Android 沙盒工作区在工具页提供独立文件管理器入口，可在独立界面浏览用户文件、导入到当前目录、选中文件导出或删除；文件管理器后端强制限制在沙盒用户文件区，屏蔽 `runtime/`、`tmp/`、`.pai`、`mcp/`、`skills/`、`private-organization/`、`avatars/`、`media/` 与应用数据文件等系统目录。
- 新增（android-workspace）：后端补齐 RikkaHub 式工作区文件 API 骨架，提供文本 read/write、move、glob 与 grep 命令，并复用 Android 工作区路径逃逸与系统目录屏蔽规则，便于后续前端或 Skill 直接维护助理空间文件。
- 优化（android-workspace）：Android 沙盒文件管理器在移动端改为全屏安全区布局，工具栏在小屏使用两列按钮，并接管系统返回键，优先关闭删除确认、返回上级目录，再关闭文件管理器。
- 优化（android-workspace）：Android 沙盒工作区初始化下载增加连接与无进展超时，避免长期停留在“连接下载源”；超时会回写明确错误并输出日志，便于区分 DNS、握手和下载体卡点。
- 修复（android-workspace）：Android Linux rootfs 解压兼容 Ubuntu Base 中的硬链接条目，硬链接创建失败时按 RikkaHub 同类实现回退为复制源文件；解压改为 runtime staging 目录，成功后再替换 `runtime/android-workspace/default/linux`，避免失败重试时被半截运行环境污染。
- 修复（android-workspace）：Android Linux rootfs 在线下载的独立 HTTP client 在 Android 端套用静态 WebPKI 根证书，避免 `rustls-platform-verifier` 未初始化导致连接阶段 panic；同时新增 Ubuntu Base 压缩包手动导入兜底，用户可自行下载 `ubuntu-base-24.04.3-base-arm64.tar.gz` 后在工具页导入，后端会校验大小与 SHA256 再解压置为 ready。
- 修复（android-log）：Android 后端文件日志在拿到应用数据目录后再解析并缓存 `backend.log` 路径，避免启动早期把日志路径永久缓存为空；Android 调试输出统一走运行日志通道，兼顾 `adb logcat` 与文件日志。

- 新增（android-workspace）：LLM 的 exec 终端命令后端新增 Android 沙盒 Linux 执行路径（`sandbox/backend_android.rs`）。工作区就绪时，通过 proot 将 Ubuntu rootfs 挂载为完整的 Linux 运行环境，沙盒根目录 bind 到 `/workspace`；LLM 可在 `/workspace` 内自由执行命令、安装软件包（如 apt install git）、管理 Git 仓库。启动执行前自动 patch rootfs 的 DNS、hosts、hostname、locale、group 等配置。

- 新增（android-workspace）：Android exec 的 LLM 提示层更新：终端环境块中明确说明当前是 Android 沙盒 Ubuntu Linux 运行环境，/workspace 是默认工作目录，可使用 apt 等标准工具；工具描述中 shell 标注为"Ubuntu 24.04 Linux (proot 沙盒)"。
- 修复（android-workspace）：Android proot 依赖库打包改为将 Termux `libtalloc.so.2` 重命名为 Android 可稳定打包的 `libtalloc.so`，并用 patchelf 同步修补 proot 的 `DT_NEEDED`；同时打包并运行前校验 `libandroid-shmem.so`，避免继续出现 `CANNOT LINK EXECUTABLE`。
- 修复（android-workspace）：Android proot 执行不再依赖 rootfs 内 `/usr/bin/env` 启动命令；参考 RikkaHub 的 rootfs 处理方式，将 rootfs 内绝对符号链接改写为宿主可用的相对链接，并在运行前按 `usr/bin/dash` 自愈 `/bin/sh`；proot 启动环境改为继承 Android 必需环境并显式移除 loader 污染变量，guest `TMPDIR` 固定为 `/tmp`，宿主 `PROOT_TMP_DIR` 固定到应用私有 runtime，避免宿主环境变量和内部 rootfs/proot 临时目录污染 exec。
- 修复（android-workspace）：Android proot 执行包装统一使用 `/bin/sh` 启动，不再因 rootfs 中 Bash 条目异常触发 `/usr/bin/bash` 缺失；`PROOT_TMP_DIR` 移出 `/workspace` 绑定源目录，改用应用私有运行目录，避免 proot 临时 glue rootfs 与工作区 bind 自引用导致 `tmp/proot/proot-XXX` 创建或 chmod 失败；重置沙盒时同步清理新旧 proot 临时目录。
- 修复（android-workspace）：Android 文件工具新增 proot guest 路径别名映射，`/workspace/...`、`/root/.pai/...` 会在 read/read_media/apply_patch/write/delete/move 前映射到应用私有 `llm-workspace` 真实路径，解决终端、Skill 与文件工具看到的助理空间不一致问题。
- 修复（android-workspace）：Android proot exec 不再套用桌面端只读命令白名单、绝对路径授权和写入风险审查，`/proc`、`/etc`、`apt`、`python` 等标准 Linux 命令可直接在独立沙盒内执行；提示层同步说明 apt/pip 安装内容会随 rootfs 持久保留，直到手动重置沙盒。
- 修复（android-workspace）：Android 通用 read/read_media/apply_patch/write/delete/move 改用独立文件工具边界规则，不再依赖 Linux rootfs 就绪状态；继续禁止直接访问或通过符号链接绕入 `runtime/`、`tmp/`、`.pai`、`mcp/`、`private-organization/`、`avatars/`、`media/` 与应用数据文件，同时恢复 `skills/`、`.gitignore` 等 Skill 和正常项目文件访问，确保沙盒损坏或重置后仍可维护助理空间文件。
- 新增（android-workspace）：Android 工具页新增“修复沙盒”和“重置沙盒”；修复复用 exec 启动前的 rootfs 入口、符号链接、配置与 proot 依赖自愈，不删除已安装软件；重置需显式确认，只删除 Ubuntu rootfs、下载缓存和 proot 临时目录，保留工作区用户文件、Skill、MCP、私有组织与应用配置。
- 修复（android-workspace）：Android 私有人格/私有部门读取根固定到应用私有 `llm-workspace`，reload 路径补齐配置归一化；proot 内将 `llm-workspace` 整根同时映射为 `/workspace` 与 `/root/.pai`，避免初始化 Linux 沙盒后组织与 Skill 读到空目录。
- 修复（runtime）：读取和写入 agents shard 时强制保留默认助理、副手、用户与系统内置人格；部门归一化不再因为 API 配置为空提前返回，运行组织快照与启动配置读取会补齐并写回默认部门，确保内置助理部门等默认部门不会被空配置状态连带隐藏，避免 Android 首启后无法新建或继续对话。
- 修复（runtime）：专家模型绑定 `assistantDepartmentApiConfigId` 无效或为空时自动回退到当前/首个文本聊天模型，前端部门人格选项与当前会话模型解析同步兜底，避免沙盒和供应商配置同时启用后默认部门存在但新建/继续对话判定人格、部门不可用。

- 优化（android-workspace）：Android 文件管理器导入冲突不再直接报错，改为自动生成 `name (1).ext` 风格的重命名文件路径；导入和导出均增加 64 MiB 单文件上限，超限时在前端和后端均返回明确错误。

- 重构（android-workspace）：Android 工作区代码按 RikkaHub 职责拆分为 `types`（状态/DTO/限额）、`manager`（resolver/sandbox 边界）、`file_system`（文件 API helper）、`rootfs_installer`（下载/校验/安全解压/staging 原子安装）、`rootfs_paths`（tar 逃逸检查与符号链接相对化纯函数）、`rootfs_patcher`（DNS/hosts/locale/group/入口自愈）与 `proot_runner`（proot 命令构建与执行）；`android_workspace.rs` 从 2000+ 行收口至约 1000 行，旧 `sandbox/backend_android.rs` 删除，由新 `sandbox/android_rootfs/{runner,patcher}.rs` 取代。
- 新增（android-workspace）：Android 工作区状态升级 v2，状态文件新增 `llmWorkspaceRoot` 与 `runtimeRoot` 两个路径字段；旧 v1 状态读取时自动回填路径、版本号与下载总量并写回，前端类型同步扩展。
- 新增（sandbox）：沙盒执行请求支持可选 stdin 与取消令牌；新增统一输出收集层，先启动 stdout/stderr 读取再写入 stdin 避免大输出死锁，stdin 写入遇 EPIPE（子进程提前退出）降级为警告不再误报失败，超时与取消双路径可中断；Windows 后端 stdin 写入改为独立线程避免不消费输入时挂死。
- 测试（android-workspace）：新增 `android_workspace_rootfs_paths`（tar 逃逸/绝对符号链接相对化/跨目录相对化）、`android_workspace_rootfs_patcher`（patch 幂等/dash 自愈/group 保留/symlink 修复）、`android_workspace_status_v2`（v1 JSON 兼容回填/序列化往返）三组集成测试，与既有路径契约测试合计 22 个用例通过；删除内嵌重复测试。
- 新增（android-workspace）：Android 文件管理器补齐查看文本、编辑文本、移动/重命名与 glob/grep 搜索入口，直接复用后端 read/write/move/glob/grep 命令与既有沙盒路径边界。
- 修复（android-workspace）：Android proot 打包补上 `libproot_loader.so`（此前 CI 只打包 `libproot_exec.so`，guest 进程缺少 ELF loader 导致 `execve("/usr/bin/sh")` 直接失败、rootfs 表现为未挂载）；runner 在 loader 缺失时给出明确错误而非静默继续。
- 修复（android-workspace）：Android proot `PROOT_TMP_DIR` 统一改为基于 canonical root 计算，与 `-r` rootfs 路径同源，修复 `/data/data` 与 `/data/user/0` 前缀不一致导致 proot 找不到 glue 临时目录、rootfs 未挂载的问题；`android_workspace_proot_temp_root` 与 repair 命令同步 canonicalize。
- 修复（android-workspace）：Android proot exec 改为宿主侧 `TMPDIR` 指向应用私有 runtime（此前误设 guest 内 `/tmp`，导致 `--link2symlink` glue rootfs 在 Android 宿主 chmod 失败、rootfs 未挂载）；启动前显式探测 `PROOT_TMP_DIR` 可写性并校验 `/usr/bin/sh`/`/bin/sh` 入口，失败时给出可诊断错误而非裸 `execve` 失败。
- 修复（android-workspace）：Android rootfs 入口自愈同时覆盖 `/usr/bin/sh` 与 `/bin/sh`（Ubuntu 中 `/bin -> /usr/bin` symlink，proot 实际 execve `/usr/bin/sh`），并放宽就绪判定为 dash + `/usr/bin/sh` 同时可用，避免 rootfs 就绪但 exec 必挂。
- 修复（android-memory）：Android 端嵌入（OpenAI/Gemini）与重排（vLLM）模型调用改用静态 WebPKI 根证书构建 HTTP 客户端，避免 reqwest 默认 `rustls-platform-verifier` 在 Android 上未初始化 panic 导致记忆检索调用异常。
- 修复（android）：合并上游 0.43 后补注册 `list_transport_conversations`、`list_conversation_create_options`、`get_conversation_workspace_permission`、`select_conversation_workspace_permission`、`save_conversation_workspace_layout`、`list_conversation_workspaces` 六个 Tauri 命令，修复前端 `workspace.list`/`workspace.layout.save`/`workspace.permission` 调用找不到命令导致工作区与沙盒状态加载失败的问题。
- 修复（android）：Android WebView 同样注入 `__TAURI_INTERNALS__`，窗口控制按钮显隐改为在传输适配层按平台能力判定，移动端不再显示最小化/最大化/关闭按钮。
- 修复（android）：Android 上打开设置不再调用 `show_main_window`（仅配置了 chat 窗口会报 "Window 'main' not found"），改为 WebView 内导航 `settings.html` 并注入 `platform=android`，恢复设置页打开与工具页 Android 沙盒管理入口。

> **Proot 来源：** CI 从 Termux 官方包仓库下载 proot 5.1.107.89 aarch64 及 libtalloc 依赖，用 patchelf 将 rpath 设为 `$ORIGIN` 确保同目录解决动态库依赖。

## 修复

- 修复（android-notification）：Android 端轮次完成/失败通知恢复“前台跳过”语义。Android 为单 WebView，wry 端窗口可见性/焦点均无实现（`set_visible`/`focus` 为 Unsupported，`is_focused`/`is_visible` 无对应消息处理），桌面式窗口焦点判定不可用；现改由前端 `useChatForegroundActivity` 上报聊天视图前台激活状态（`visibilitychange` + focus + 聊天视图），后端 `set_chat_window_active` 全平台存储该状态，Android 下会话有活跃 binding 且聊天视图前台激活时才跳过通知，前台不打扰、切后台正常提醒，桌面端行为不变。
- 修复新建“隔离工作树”会话时 Git 根目录二次校验会弹出控制台窗口的问题：Windows 下以 `CREATE_NO_WINDOW` 执行校验进程。
- 修复后台子进程弹出控制台窗口的遗漏点：Git 幽灵快照、VSCode 桥接网络探测、winget 安装、WSL/Shell 终端启动器、默认程序打开文件，均以 `CREATE_NO_WINDOW` 抑制多余控制台窗口。

## 依赖

## 功能

- 新增（android-live-update）：live update 通知在 TODO 更新后自动刷新。`update_conversation_todos_and_emit` 更新 todo 成功后新增 `live_update_todos_changed` 刷新入口，同 id 重发 ongoing 通知，让岛/通知栏实时展示最新步骤文本；并复用「有 todo 显示步骤、无 todo 显示对话标题」的正文逻辑。桌面端为空实现，行为不变。
- 新增（android-live-update）：live update 通知新增短文本（Android 13+ `shortText`）。Rust builder 新增 `short_text()`，Kotlin 端在 API 33+ 调用 `setShortCriticalText`，岛/锁屏胶囊优先显示 todo 当前步骤，无 todo 时显示对话标题，展开通知仍显示完整标题与正文。
- 新增（android-notification）：通知设置页新增「通知测试」，可分别发送普通通知与实时通知测试：普通通知立即弹出一条；实时通知在 Android 上模拟 live update 完整生命周期（ongoing 第 1/2 步 → 2 秒后刷新第 2/2 步 → 2 秒后转终态），桌面端降级为普通通知。
- 新增（android-live-update）：Android 端新增消息输出与目标 live update 通知。消息轮次开始/进行中显示常驻 ongoing 通知（“正在回复…”），完成或失败后同 id 更新为可手动划掉的终态通知，不再重复打扰；目标创建/更新显示进行中通知并展示目标摘要，目标结束后更新为终态。参考 MAA-Meow TaskExecutionService 的 live update 模式，采用“同通知 id 更新 + ongoing 切换”实现，多会话并发时用 owner 记录保证只有当前显示归属会话结束才更新终态，避免覆盖仍在进行中的其他会话通知。桌面端为空实现，行为不变。
- 新增（android-live-update）：Android 15 (API 35) 官方 live updates（promoted ongoing）。将 tauri-plugin-notification 2.3.3 vendor 进 `src-tauri/vendor/`（path 依赖），扩展 Android 端通知能力：Rust builder 新增 `request_promoted_ongoing()` 与 `progress()`；Kotlin 端 `Notification` 数据类新增 promoted/进度字段，`buildNotification` 在 ongoing + promoted 时调用 `setRequestPromotedOngoing`，并在提供进度时设置进度条；插件 manifest 声明 `POST_PROMOTED_NOTIFICATIONS` 权限（随 manifest merge 进应用），插件依赖 `androidx.core:core-ktx` 升至 1.17.0（`setRequestPromotedOngoing` 自该版本加入）。消息输出中/目标进行中的 ongoing 通知现以标准样式（BigTextStyle + 不确定进度）请求系统提升，API 35+ 默认展开、锁屏可见，API 35 以下静默降级为普通 ongoing；终态通知不请求提升。桌面端行为不变。
- 修复（android-welcome）：设置页欢迎界面的 Git/Node.js/ripgrep 运行时检测在 Android 上改走沙盒 Linux 环境：宿主 PATH 检测在 Android 恒判未安装（git/node/rg 实际运行在 proot Ubuntu 沙盒内），现改为沙盒就绪后在沙盒内执行 `command -v` 检测，未就绪视为未安装；同时欢迎界面接收 Android 平台标记，Android 上 Git/Node 卡片隐藏不可用的“一键安装/手动安装”按钮（winget 仅桌面端），并提示在沙盒终端用 `apt install` 安装，桌面端行为不变。
- 新增（android-live-update）：live update 通知小图标改为 PAI 原图标：通知插件全局配置与 live 通知 builder 均指定 `ic_stat_pai`，`scripts/patch-android-project.sh` 将 `src-tauri/icons/android/drawable/ic_stat_pai.png` 复制为 Android drawable 资源，随构建自动生效，桌面端不变。
- 新增（android-live-update）：live update 通知正文显示 TODO 当前步骤：目标进行中 / 消息输出中的 ongoing 通知优先展示会话当前 todo 的进行中步骤（`第 X/N 步：内容`，英文 `Step X/N: ...`），无 todo 时回退原正文（目标摘要 / “正在回复…”）。
- 新增（android-live-update）：Android 后台任务保活前台服务：保活由任务生命周期驱动（回复轮次开始 / 目标激活时启动，全部结束 / 目标结束时停止），Rust 侧维护活跃任务集合并通过通知插件新增的 `keep_alive_start` / `keep_alive_stop` 命令启停 `specialUse` 前台服务（引用计数，通知插件 Kotlin 端新增 `startKeepAlive` / `stopKeepAlive` 命令），独立于通知权限与 live 通知发送结果——通知权限被拒时任务仍在后台运行，进程照样保活；服务自带低打扰保活通知，任务结束后自动移除。插件 manifest 新增 `FOREGROUND_SERVICE` / `FOREGROUND_SERVICE_SPECIAL_USE` 权限与 service 声明。
- 新增（android-remote-frontend）：开放 Android 设置页原「网络连接」入口（去掉 desktopOnly 过滤），Android 端替换为「远程前端」连接表单：输入电脑 PAI 的 IP 与端口（默认 8429），连接后手机 PAI 应用壳保留、内容区 iframe 加载电脑 PAI 的 `http://ip:port/sidebar` 远程 Web UI，识别本地/远程模式；远程目标存 localStorage，应用重启默认本地模式。
- 新增（android-remote-frontend）：远程模式下右上角设置图标旁新增「退出远程」按钮；点设置时 iframe 切换到电脑 PAI 的 `/settings` 设置页，header 提供「返回远程聊天」按钮；退出远程后回到手机 PAI 本地界面并清理远程通知。
- 新增（android-remote-frontend）：远程模式通知：iframe 内远程页面（web 宿主分支）把电脑 PAI 的 `chat.assistantDelta` 事件经 `window.parent.postMessage` 转发给手机壳层，壳层调本地 `remote_live_update_notify` 命令构建 Android 通知（回复中 ongoing → 完成/失败同 id 更新终态，delta 节流刷新，退出远程时 `clear` 移除）；复用 live update 通知基建，桌面端行为不变。
