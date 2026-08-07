# 变更日志

> 此文件由 `pnpm changelog:build` 自动生成，展示最近版本的完整说明。

## 发布：v0.56.0

## 功能

- 足迹墙按凌晨 4 点分界：凌晨 0:00-3:59 的使用归属前一个分界日（适配通宵工作场景），今日视图、历史日历、年份统计与小时图全部按分界日口径计算；今日小时图 x 轴按分界日顺序排列（04:00 起头，0-3 点收尾）
- 简单页自定义供应商新增 API 协议下拉框：可手动选择协议（Auto / OpenAI Compatible / DeepSeek / OpenAI Responses / OpenAI Codex / Google Gemini / Anthropic / Ollama 等 26 项，与高级配置页一致），刷新模型列表与保存配置均按所选协议生效，替代原先固定的 auto。
- 模型卡新增显示名：模型卡标题默认显示模型名，点击即可原位修改显示名，回车或失焦保存、Esc 取消、清空恢复为模型名；简单页与高级页供应商配置均支持，显示名会随配置持久化，并在配置页下拉、模型选择树、聊天窗口模型选择、部门人格模型名等位置统一显示。
- 简单模式界面优化：供应商配置与 API Key 分区展示并加分割线，API Key 输入框与显示/隐藏按钮紧贴；模型卡说明（快速/专家/多模态用途）移至卡片外展示，标题旁增加编辑图标提示可修改显示名；简单模式保存的供应商统一以「简单供应商」命名，高级配置页中可辨识。
- PDF 阅读方式固定为图片模式：聊天设置页不再显示 PDF 阅读方式选项，读取 PDF 时始终按图片模式处理（仅受模型图片能力约束）。
- 配置页调整：「对话」页更名为「常用」，快捷操作（对话列表 / 请求预览 / 系统提示词预览）移入「日志」页的调试设置区域作为第三行；「日志」页更名为「调试」，调试设置区标题由「调用日志（内存）」改为「调试设置」。
- 常用页进一步调整：常用页图标改为星标并移到欢迎页下方；语音转文字开关文案由「完成后发送」改为「语音转文字后直接发送」；对话风格区域修复了标题重复显示的问题；指令预设输入框改为原生带边框样式，新增指令按钮移入卡片左下角与保存按钮同行。
- 执行终端设置从工具页移入常用页：工具页不再展示终端运行环境下拉，常用页新增「执行终端」分组，可选择终端运行环境（Windows 下），未检测到可用终端时提示安装 Git。
- 记忆页的聊天记录搜索与召回诊断标记为开发调试功能：仅开发模式下显示，生产构建中隐藏。
- 模型卡不允许空模型名保存：新增模型卡不再预填默认模型名，保存配置时校验模型名不能为空，未填写的模型卡会被拦截并提示。
- 会话目录权限弹窗优化：目录行在窄屏下自动换行不再重叠，弹窗改为头部/尾部固定、中间目录列表滚动并按视口高度自适应；新增按名称或路径搜索目录，目录超过 100 个时仅显示前 100 个并提示细化搜索。

## 修复

- 修复聊天窗口启动崩溃：ChatView 中监听右侧面板状态的 watch 在 `rightPaneOverlay` 初始化前执行触发 TDZ（ReferenceError），导致 setup 崩溃并连锁引发 `instance.update is not a function` 等报错，前端无法打开；调整声明顺序后恢复正常
- 修复模型显示名编辑不生效：模型卡内修改显示名后，脏检查（dirty）比对未感知该字段，保存/还原按钮保持禁用；同时保存载荷与启动加载路径也丢弃了显示名，导致即使保存成功重启后也会丢失。现已在脏检查、克隆、保存载荷、加载四条链路补齐 `displayName` 字段透传，并同步到聚合组内全部思维等级变体，简单页保存/草稿/加载链路同步携带
- 修复流式工具分段漏占位符：工具事件带 content/reasoning 时会新建 stream block，后续正文追加到新 block，跨 block 边界拼接仍用 `\n\n`，导致流式期间工具后正文无法按段展示；`assistantTextFromStreamBlocks` 改为与正式消息投影一致的占位符拼接规则（前段含工具标记且后段含正文时写占位符），流式与刷新恢复路径分段行为对齐
- 录音按钮改为仅桌面 APP 端显示：Web 端（含远程前端 / VS Code 侧边栏）不再显示按住说话按钮，改用宿主语义能力判断，不再依赖视口宽度与触摸检测，避免按钮随视口变化延迟出现/消失
- 模型按钮宽度自适应：桌面端按模型名长度自动伸缩并带上限，不再固定 176px 截断；窄屏仍按原上限收缩
- 输入面板手机模式适配：窄屏触摸设备下隐藏语音按钮（改用手机自带语音输入，减少麦克风权限依赖），模型按钮自动收缩宽度，避免计划模式按钮出现时发送按钮被挤出屏幕
- 远程访问密码认证：被 iframe 嵌入（远程前端模式）时优先向父窗口请求已保存密码，避免 Android WebView 拦截跨域 iframe 的 window.prompt 导致认证卡住（桌面独立窗口行为不变）
- 远程前端模式窗口栏叠加：被 iframe 嵌入且非 VSCode 宿主时隐藏页面自带窗口栏，避免与手机壳层 header 上下叠加（桌面独立窗口与 VSCode 侧边栏行为不变）
- 远程通知标题语言：广播 assistantDelta 附带的会话标题改为跟随用户 ui_language 配置（不再写死 zh-CN），与本地 live update 通知标题保持一致
- 远程桥接安全加固：与 iframe 父窗口（手机壳层）的 postMessage 双向通信统一校验约定 origin，转发 targetOrigin 不再使用通配 `*`，防恶意页面伪造密码/会话命令或窃听通知事件；密码消息接收侧额外校验 event.source 必须等于 window.parent，拒绝同 origin 下其他窗口伪造的密码注入
- 焦点恢复尾部对账改按会话自身 freshness（updatedAt + lastMessageId）指纹判定：不再依赖全局概览水位，避免列表先同步时吞掉视图尚未应用的正式消息收口；同时补充焦点恢复状态机关键路径日志（[焦点恢复] 前缀）便于排查
- 会话列表聚合位置修复：同一人格聚合块内的简化条目不再被统一排在列表末尾，改为紧跟其完整条目显示，避免与相邻目录的会话错位

## 发布：v0.55.0

## 功能

- 会话列表分组优化：同目录、同助手的会话自动聚合，最新一条完整展示，其余折叠为单行条目；打开会话不再自动展开所在分组，最近会话的文件夹胶囊可点击展开并定位到分组。
- 文件阅读器「打开目标」按钮组移到顶部常驻显示：目录树未打开时也能按目标打开当前目录，按钮样式与标签页统一。
- exec 工具新增高危命令确认机制：被本地规则拦截的危险命令会返回确认提示与承诺文案，用户了解风险并输入承诺后即可放行执行。
- 工具审查侧边栏文件改动按文件分组展示，每组内按时间顺序内联渲染，带动作标签与行数增减统计；补丁 diff 卡重构为表格样式，支持点击/右键折叠。
- 会话控制面板的 @ 人格列表不再显示被禁用的条目，只保留可用的候选。
- 输入框占位文案按状态动态切换：正常、忙碌中、有排队消息、有引导消息时提示各不相同。
- 输入框为空时发送按钮降级为灰色，点击可弹出快捷键切换菜单（Enter / Ctrl+Enter）。
- 计划模式入口改为可点击：输入命中「计划/方案/设计/架构」等关键词时，输入框旁出现「计划」按钮，点击开启；激活状态的高亮徽标也可点击取消。
- 气泡分段改为占位符方案：分段模式与背景开关彻底解耦，无背景时用分割线区分段落，计划卡与正文段样式统一；「思考与工具」面板不再显示只有工具标记、没有正文的空条目。
- operate 截图工具完善：截图总是保存并返回路径（默认按会话建目录，会话压缩/归档/删除/撤回时自动清空对应目录），驱动模型不支持图片时也能截图并返回路径。
- 桌面操作提醒改为按轮次发送：同一轮调度内首次调用 operate 才弹系统通知，不再频繁打扰。
- 对话设置页「多模态分析模型」改用模型树选择器，与其他模型选择器统一。
- 最高指令新增「程序讲解」一节：讲解调用链时先摸清函数调用链，以函数名/变量名为主位，不清楚时先调查再讲。
- 「简洁」对话风格新增约束：同一要点只讲一个面向，不提供正反两面讲解；移除「抽象」对话风格。
- 设置窗口打开默认落在欢迎页；「热键」文案统一改回「快捷键」。

## 修复

- 追问会话流式期间停止按钮可用了：之前流式时停止按钮被禁用，无法中断，现与主窗口行为一致。
- 修复输入法组合输入后发送快捷键被误吞的问题：Ctrl+Enter 发送模式下，中文输入法组合确认后的 Enter 不再被误判。
- Web 端（VS Code 侧边栏 / 浏览器直连）自己发起对话不流式的问题已修复，正文不再一次性全量刷出。
- 压缩/归档摘要请求不再因历史图片未按模型能力处理而失败（不支持图片的模型上历史图片自动降级处理）。
- 追问视图发送消息后不再丢失平滑滚动到底部的行为，与主会话一致。
- 停止回复或切换到其他窗口时，不再多余地全量重拉消息历史，只应用后端返回的结果。
- 工具审查侧边栏新建文件的补丁行号不再错位。
- 登录启动时同步 shell 环境不再可能卡住应用：超时自动跳过，Linux/macOS 上的编译问题已修复，三平台构建恢复通过。
- 应用读取文件、处理大表情贴纸、截图时不再卡顿其他会话的回复与操作（耗时操作移出主线程）。
- 表情贴纸 GIF 入库保留原始动图（不再转成静态 WebP），超过 5MB 的 GIF 拒绝入库并提示。
- 移除不再使用的 reload / organize_context / screenshot 工具（能力已并入 config / operate），工具列表更干净。
- 副手部门（deputy/explorer）的工具权限不再有硬编码限制，与其他部门一样完全由权限卡控制。
- 发布构建接入缓存，后续三平台构建显著提速。

## 依赖

## 发布：v0.52.0

## 功能

- Linux 应用内自动更新（AppImage）：支持自动检查、下载并替换安装新版本，发布流程同步生成 Linux 更新清单与签名；macOS 仍不启用。

### 会话权限管理

- 新建会话卡片与权限管理卡片新增「本会话始终批准一切请求」开关：勾选后本会话一切请求自动批准，终端与补丁工具可访问任意目录，跳过目录权限、智能评估与人工审批；勾选后所有目录权限选择下拉框隐藏。
- 工作模式文案与顺序调整：在此目录工作 / 在此目录的会话工作树工作（独立工作树）/ 在此目录的所有工作树工作，中英文同步更新。

### 发送快捷键

- 发送按钮右键菜单可切换发送方式：按 Enter 发送 / 按 Ctrl + Enter 发送（Ctrl+Enter 模式下 Enter 换行），选择持久化保存，快捷键页「发送快捷键」下拉同步联动。
- 快捷键页重构：组名改为「热键」，内置快捷键合并入组并补充说明描述，不可修改项以禁用态展示，滚轮等键名按当前语言显示；移除「呼叫 AI」入口。

### 足迹墙：欢迎页用量砖块墙

- 欢迎页顶部新增「足迹墙」模块：1 天 / 7 天 / 30 天 / 所有 四窗口切换，按会话 / 人格 / 部门分组，砖块色阶展示 token 用量，会话数 / 总 token 统计卡一目了然。
- 用量改为写入时记账：新增全局 SQLite 台账表 `usage_trail`，按「小时桶 × 会话 × 模型」累加每次 LLM 调用，provider 显示名写时快照，不再受供应商改名影响。
- 用量页与足迹墙同源：`get_usage_overview` 改为从台账聚合，旧 `cumulative_usage` 聚合链退役。
- 历史数据迁移：v3 消息迁移后自动把旧账本迁入台账 epoch 桶（标记为历史累计），迁移幂等（事务 + 互斥，失败可安全重跑）。

### 欢迎页精简为一行仪表盘

- 移除欢迎主卡片与 13 张配置检查卡，改为单行仪表盘：缺什么才提示什么。
- 运行时依赖 git / node / rg 缺失时显示「未安装」+ 一键安装（winget 自动安装，失败自动打开官方下载页）；已装好的依赖不再占位。
- 快速 / 专家模型未设置时显示提示按钮，点击直达对话设置页完成模型分工。
- 右侧保留「开始对话」直达按钮；清理 welcome 块约 40 个死 i18n 键（title/subtitle/badge/各卡片 summary 等）。
- 欢迎页卡片间距与卡片内边距统一为 16px，模块衔接更自然。

## 重构

## 修复

- Linux / macOS 下关闭窗口直接优雅退出应用，不再出现「窗口隐藏后无法恢复、托盘不可见时无法退出」的死锁。
- Linux Wayland 会话不再启动录音热键监听（该能力依赖 X11，Wayland 下无效且每次启动报错），改为明确跳过并记录日志。
- 修复 Linux / macOS 下大小写不同但真实不同的工作区路径被误判为重复而丢弃的问题。
- Linux 终端工具新增 zsh 识别（bash 优先、zsh 次之、sh 兜底）。
- 全局呼出热键与录音键录制不再接受 Win/⌘ 键组合（按 Win/⌘ 时提示不支持），避免录制成功但实际不生效的误导。
- 修复默认录音键 CapsLock 无法触发录音的问题（按键映射缺失导致按下无响应）。
- 保存设置时若全局热键已被其他软件占用，会明确提示「热键已被占用」，不再静默保存后无任何反馈。
- 录音后台唤醒默认改为禁用；该功能仅 Windows 支持，macOS / Linux 下设置页直接隐藏开关，避免误导用户开启后不生效。
- 用量页四张表（模型用量/人格/类型/会话）统一精简为五列：名称（供应商·模型/人格/类型/会话）+ 总量 + 输出 + 思考 + 缓存命中率；移除综合总量、缓存读、缓存写、会话数/消息数列，思考列新增排序支持。
- 用量概览精简为四项：总量（total_tokens 口径）、输出、思考、缓存命中率；移除综合加权总量、缓存读取、缓存写入与参考说明，卡片不再带辅助描述；概览卡片改为 md 断点横排，设置窗口宽度下不再竖排。
- 设置页各页工具栏按钮（刷新 / 打开目录 / 新增 / 保存等）统一为实色 base100 样式并配同规格图标，供应商能力标签页未选中态同步改为 base100，视觉不再混搭 outline / 主题色。
- 快捷键页「后台语音唤醒」行标题不再写死为「开」，改为「后台语音唤醒（按住录音键）」标题 + 右侧真实状态开关（切换走后端命令并回写，失败回滚）。
- 设置页导航侧边栏顶部增加留白，首个导航按钮不再紧贴分界线。
- 更新弹窗改为三段式布局：标题与进度条固定在顶部、更新日志中间独立滚动、操作按钮始终可见，不再需要滚动到底才能点按钮。
- 足迹墙「所有」窗口会话数按跨桶并集去重，同一会话在小时桶与历史桶只计一次；委托会话归档后仍归 delegate 分组，口径与用量页一致。
- 足迹墙分组占位 label（已删除会话/未绑定人格/未绑定部门）改为按当前语言展示，英文界面不再混入中文。
- 用量页「会话类型」分组表 label 按 kind key 映射到当前语言，英文界面不再显示硬编码中文类型名。
- 用量页已删除会话标题、未绑定人格、未识别供应商占位统一映射到当前语言，英文界面不再混入中文。
- 聊天流式期间消息重叠与高卡顿：测高完全改走官方 TanStack Virtual `measureElement`（内部 ResizeObserver + itemSizeCache 缓存短路），移除全部自定义测高实现与项目测量缓存，重复 ref 回调零 DOM 访问，弹性尾部高度与布局同源读取。
- 流式正文与思维链文本改为 100ms 节流批量应用（约 10fps），工具状态与事件、权威快照即时处理，流式结束/停止时冲刷缓冲，最后一段内容不丢失。
- 正在输入后的耗时计时器独立为模块级状态，不再整体重建消息数组，虚拟列表不再被计时器空转重算。
- 思维链字数显示改为 1 秒线性爬升动画（双重缓冲：目标值随节流更新，显示值平滑过渡），数字不再逐帧闪烁。
- 修复空消息仍进请求体的问题：临时消息块（计划/goal 提示词渲染为空）和纯空消息不再补空格掩盖，历史 user 空消息在源头直接跳过、latest user 全空不追加，空 assistant 正常走过滤；之前只在日志预览路径生效，生产请求体组装漏同步。
- 工具执行空结果占位由空格改为 `(no output)`：Anthropic / Bedrock / GLM / DeepSeek-GCP 等严格供应商拒绝空 tool content，语义化占位让模型区分「执行成功但无输出」而非看到空白。

## 依赖

## 发布：v0.51.0

## 计划模式：切换更顺畅

- 计划模式切换改为内存态处理，不再写入会话记录与磁盘、不再经过会话互斥锁；忙碌（流式输出 / 整理上下文）期间也可以自由切换计划模式，不需要等当前轮次结束。
- 子对话（side chat）不再继承主对话的计划模式，各会话独立记忆自己的模式，避免相互干扰。

## 聊天：流式中也能加载更早历史

- 修复流式输出期间滚动到顶部无法加载更多历史的问题：流式期间靠近顶部时会临时切换列表锚定方式，让「加载更早消息」的视口补偿生效，往上翻历史时内容不再跳动错位。

## 附件：PDF 更轻量

- PDF 附件不再随请求整包发送二进制，只保留路径提示；模型需要内容时通过内置的 read_file 工具按需读取，请求更快、更省流量。

## 界面细节

- 思维链与工具展开区左移对齐头像，折叠头保持原位，视觉更整齐。
- 工具卡弹窗宽度以中间对话区域 92% 为上限并自动换行，不再撑出屏幕。

## 关于页与更新

- 更新日志接口缓存 1 小时，避免频繁请求 GitHub。
- 仓库入口与检查更新移入版本卡片右上角，更新日志卡片高度随窗口自适应。

## 部门与人格

- 部门树节点与直属下级多选显示人格头像，标题栏增加人格页跳转入口，一眼认出是谁。

## 修复

- 修复 codex 端点 400 报错：移除伪造请求头，升级依赖至官方修复版本。

- 新增（android-remote）：远程前端模式连接表单新增「访问密码」输入框，密码随远程目标存入 localStorage；连接后手机壳层监听 iframe 内电脑 PAI 页面的密码请求，用已保存密码自动回复完成认证，规避 Android WebView 对跨域 iframe `window.prompt` 的静默拦截；电脑 PAI 页面在 iframe 嵌入时优先向父窗口请求密码，桌面独立窗口仍走原密码弹窗，行为不变。
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

## 发布：v0.50.1

## 修复：聊天更稳

- 修复流式输出期间消息行重叠：上一版「消除流式卡顿」的优化误伤了消息行高度测量，回复增长时用户消息会和助理消息叠在同一行；已恢复可靠的高度测量，流式消息与普通消息不再重叠。
- 修复切换会话时滚动位置卡住：切到新会话后列表有时停在半路，看不到最后一条消息；现在会等新会话消息排版完成后自动定位到最后一条，切走切回都不再错位。
- 修复流式消息串会话：后台会话正在流式时切换会话，偶尔会把别的会话的整条回复刷到当前会话；已加强会话隔离，各会话的流式内容互不串扰。

## 操作：忙碌中不再误禁

- 忙碌（流式输出 / 整理上下文）期间，从消息创建分支、转发、分享、委托、代码审查、创建任务、切换工作目录、浏览器打开等操作不再被禁用——这些是子代理或读取类操作，不影响当前轮次；撤回与多选模式仍保持忙碌禁用。
- 修复右键「从消息创建分支」在忙碌时报「完成后再撤回」的误导提示：创建分支不再受主轮次忙碌状态拦截，可直接确认创建。

## 发布：v0.50.0

## 性能：对话更快更省

- 上下文压缩请求与正常对话完全对齐，供应商缓存命中率从 0 提升到 98.65%，长会话压缩明显变快，也不再出现压缩后旧账残留。
- 流式输出期间不再逐字下发大体积快照，正文与思维链逐字累积，长回复滚动更跟手，聊天列表不再卡顿。
- 工具执行期间标题栏上下文用量圆环实时更新，压缩完成后正确归零，压缩卡片与标题栏百分比始终一致。

## 修复：侧边栏与 Web 端

- Web 端（VS Code 侧边栏 / 远程）消息图片恢复显示。
- 修复 Web 端调用本机窗口命令未被明确拒绝的边界问题。
- 流式输出期间不再无限刷错误日志导致前端卡死（窗口重载后残留连接会自动清理）。

## 修复：会话体验

- 侧边追问会话支持消息撤回、重新生成，以及右键「从消息创建分支」，操作与主会话一致。
- 粘贴文本跟随输入框焦点，不再固定写入主会话。
- 无头像的消息不再渲染头像占位，布局更干净。

## 设置：简单模式与首次启动

- 设置窗口新增「简单 / 高级」模式切换：全新用户默认简单模式，老用户默认高级模式；简单模式为单页紧凑表单，支持草稿保存。
- 首次启动无可用模型时强制进入简单设置，保存后自动打开对话窗口。
- 首次启动主题跟随系统明暗：亮色系统使用「秋日」主题，暗色系统使用「森林」主题。
- 移除独立「快速设置」窗口，入口整合进设置窗口；左上角更新日志移入「关于 → 版本更新」。
- 模型选择下拉改为浮层显示，不再被卡片裁剪，超出屏幕时自动向上展开。
- 未设置头像的人格显示内置品牌图标，默认人格名称改为「Pai」。

## 修复：窗口与细节

- 新建「隔离工作树」会话不再弹出多余控制台窗口，后台子进程控制台窗口全量补漏。
- MCP 工具权限兼容旧格式命名，`server-id::search` 等旧候选名不再丢失。
- 内置快捷键列表补充 `Shift + Tab`（切换计划模式）与 `Alt + Z`（代码预览自动换行）。
- 简单设置「保存并开始对话」后自动打开对话窗并关闭设置窗，状态提示不再显示原始 key。

## 发布：v0.44.0

## 修复：VSCode 侧边栏恢复稳定

- 修复（ide-bridge-discovery）：同时运行安装版与调试构建时，后启动的实例不再清除对方发布的连接信息，侧边栏不再偶发失联。
- 修复（sidebar-open-settings）：侧边栏点击设置不再提示「无法识别外部链接」，改为直接通知后端打开本机设置窗口。
- 修复（sidebar-theme-follow）：侧边栏主题恢复跟随 VSCode 编辑器，明暗切换与强调色不再各说各话。

## 修复：会话压缩后不留旧账

- 修复（compaction-clear-todos）：压缩完成后自动清空该会话的待办列表，后续轮次不再挂载压缩前的过时任务。

## 修复：组织与工具细节

- 修复（department-multi-parent）：预设部门（explorer、reviewer、saddler）可以被多个上级部门同时引用，不再被限制为仅能归属助理部门。
- 修复（mcp-tool-name-normalize）：MCP 组成员名含空格或分隔符时不再导致整轮请求失败，保存与装配时统一规范化，仅在重名时添加成员前缀。
- 修复（changelog-markdown-render）：更新弹窗的更新说明支持 Markdown 展示（标题、列表、代码块），错误信息仍保持纯文本。

## 新增：独立工作树模式

- 新增（independent-worktree）：会话可固定使用 `.pai/.worktree/{会话 ID 前 8 位}` 作为项目修改位置；Shell 与文件编辑在既有权限通过后，工作树模式仅允许写入根 `.pai/**`，独立模式额外拒绝其他会话工作树，最大权限仍保持绕过。

## 依赖升级

- 升级 @tanstack/vue-virtual 至 3.13.35（virtual-core 3.17.7）：修复上方行尺寸变化时的一帧视口跳动、流式消息跨折叠线增长不再拖拽滚动位置、短内容时滚动偏移负值等问题。
- 升级 daisyui 至 5.7.9：修复 join 嵌套泄漏、按钮激活态、modal RTL、进度条动画等组件细节问题。
- 升级 genai 至 0.7.0-beta.15：Anthropic 工具调用 JSON 解析错误不再中断流式响应、OpenAI 显式提示缓存与消息结构优化提升缓存利用率、推理档位 none 兼容 zero 命名。

## 发布：v0.43.0

## 修复：加载更多历史消息时画面保持稳定

- 修复（prepend-anchor-stability）：聊天窗口向上加载更多历史消息时，新消息刷出不再引起画面跳动；加载完成后必须脱离顶部一次才可继续加载更多，避免连续误触。

## 体验优化：聊天「思考与工具」明细

- 优化（activity-detail-layout）：思考与工具明细的活动区域支持全宽展示；摘要按首行显示并保留原始换行，展开后仅显示剩余内容；带状态箭头，不可展开项点击不再误收起。
- 优化（activity-detail-colors）：思维链固定使用橙色、工具使用绿色，浅色与深色主题各配一套明度，不再依赖主题色；思考标题去除粗体、保留斜体。
- 优化（activity-detail-alignment）：思考、正文、工具三类标记符号与文本按基线对齐，统一视觉垂直位置。
- 优化（tool-json-format）：工具参数展开区若为 JSON 原文，自动格式化缩进显示，便于阅读。

## 体验优化：模型与更新状态展示

- 优化（model-refresh-status-inline）：模型刷新状态文本框移入按钮同一行左侧，始终显示当前结果，而不是仅在成功或失败后展示。
- 优化（provider-model-refresh-fix）：修复供应商页新增模型后刷新无反应的问题；切换供应商时自动清除刷新错误和加载残留。
- 优化（log-timeline-final-append）：LLM 轮次日志时间线补充最终回复写入阶段的翻译显示。

## 更新：自动更新链路更清晰更可靠

- 优化（updater-route-status）：自动更新检查会逐条展示中转与直连线路的连接状态；单条线路超时或失败后立即切换下一条，全部失败才结束更新。
- 优化（updater-progress-log）：更新检查和下载过程补充阶段日志、线路提示与超时出口；界面不再展示端点、重试次数和下载游标等技术信息，面向普通用户更易懂。

## 修复：发送消息时的首帧跳动

- 修复（elastic-tail-first-frame）：发送新消息时，弹性尾部空间不再在未测量首帧清零后重新扩张，避免内容被上推的跳动。

## 发布：v0.42.0

## 功能：MCP 一卡一组，组内多服务器与工具名前缀

- 功能（mcp-group-card）：一张 MCP 卡片即一组服务器，definitionJson 可整体保存多个服务器并整组启停；部署时组内每个服务器独立连接、工具合并，已有单卡单服务器数据自然兼容。
- 功能（mcp-multi-format）：兼容 mcpServers 对象/数组、根级平铺对象、根级数组与单服务器直接字段五种嵌套格式；`headers` 作为 `httpHeaders` 别名，env 支持 `{value, secret}` 对象形态，`transport: "sse"` 与 `type` 别名识别。
- 功能（mcp-tool-prefix）：MCP 工具名统一带 `{成员名}_{工具名}` 前缀暴露给 LLM，按最后一个下划线还原路由到对应成员；组内歧义前缀与跨卡片成员重名在部署/校验阶段报可读错误。
- 功能（mcp-structured-errors）：MCP 校验错误改为结构化错误码 + 参数，前端按 i18n 渲染为可读文案（中/英/繁）。
- 功能（mcp-ai-fix）：校验失败时可通过专家模型一键修复 MCP 配置格式，敏感字段值脱敏占位、修复后还原，结果回填编辑框由用户确认保存。

## 功能：MCP 支持 SSE transport

- 功能（mcp-sse-transport）：MCP 客户端新增 legacy SSE（HTTP+SSE）传输支持，连接 SSE 端点、经 endpoint 事件获取 message 地址后 POST JSON-RPC，响应经 SSE 通道异步返回；鉴权头在连接与 message 请求中均携带，不再将 `transport: "sse"` 静默降级为 streamable HTTP。
- 依赖（rmcp-3）：rmcp 升级 2.1.0 → 3.0.1，适配 get_stream 签名变更与 sse-stream 0.2.5 的 API 更名。

## 功能：APP 文件路径右键打开所在文件夹

- 功能（app-file-context-open-containing-folder）：APP 端文件阅读器中，文件路径右键菜单新增“打开所在文件夹”，复用现有本地文件管理器能力；Web 与 VS Code 宿主不显示该菜单项。

## 修复：摘要标题原子账本一致性

- 修复（summary-title-shard-consistency）：自动生成摘要标题后，会话概览与消息正文保持一致。统一 replace / batch replace 的派生标题规则：替换消息若改变摘要标题状态，按替换后消息集合重算 `latest_summary_title`；`update_unarchived_conversation_by_id` 提交 v3 替换时使用统一派生元数据并同步内存缓存，不再用旧派生字段覆盖新标题。
- 修复（summary-title-batch-consistency）：批量替换 `provider_meta` 与单条替换路径共享同一派生规则，最终元数据与最终消息集合一致；替换非最新摘要、删除标题、多消息替换均按最终摘要范围取值，普通消息替换不改变既有正文长度、预览等派生字段行为。

## 功能：更新下载代理游标切换

- 功能（updater-download-proxy-cursor）：自动与中转更新下载每次仅使用当前代理游标；请求失败、HTTP 非成功、响应流中断、下载总时限 10 分钟或内容长度不完整时，持久化推进至下一个 HTTPS 下载代理并结束本次更新；完整下载后保持当前游标，直连模式不受影响。

## 修复：更新页面链接使用直连地址

- 修复（updater-release-page-direct-url）：更新窗口的“打开 Releases”始终使用 GitHub 原始发布页地址，不再把网页请求发送到仅支持资源下载的代理；更新检查、清单与安装包下载仍按所选更新方式走原有代理链路。

## 清理：移除未使用代码与警告

- 清理（dead-code-removal）：删除模型命名空间剥离、远程唤醒失败上报、入站联系人匹配等 6 个无调用函数，以及 MCP 校验错误结构中的未读字段与未使用方法；群聊长度门改写（默认禁用）整套保留并标注 `#[allow(dead_code)]`，便于后续重新启用。

## 发布：v0.41.0

## 修复：前台会话流式状态恢复

- 重构（chat-runtime-single-path）：主对话的前台恢复、水位与终态收口从录音模块收敛到唯一聊天运行时；APP 与 Web/VS Code 继续复用同一 `main-chat` 入口和传输门面。
- 修复（chat-foreground-tail-watermark）：前台恢复以会话变更水位决定是否读取正式尾消息；水位推进时强制以单条正式消息收口同 ID 的流式半成品，并在读取失败时回退轻量快照。
- 修复（chat-stop-guided-history）：停止回复后，引导消息出队产生的正式历史事件仍会立即合并到当前会话；停止保护仅抑制已停止轮次的等待/流式投影，不再吞掉正式消息。
- 修复（chat-foreground-stream-recovery）：压缩中尚未产生正式 assistant 消息时仅显示会话忙态；收到 `roundStarted` 的正式消息 ID 后才显示对应气泡，避免暴露内部占位身份。
- 修复（chat-stop-preserves-message-activity）：中止调度时直接以正式消息的内容块保存思考与工具活动，不再依赖废弃的前端流式镜像。
- 修复（chat-virtual-measurement）：聊天时间线改为真实行高驱动的虚拟布局，不再以预估高度定位富文本消息，避免首次加载或切换会话时消息短暂重叠。

## 调整：统一设置页章节模板

- 调整（settings-template-sections）：以图像生成配置页为标准，将 API 供应商、Codex、外观、通知、关于、网络访问、聊天设置、快捷键、人格、使用统计和存储迁移页的一级章节标题与说明移到内容卡片外；普通 Codex 认证字段接入 `ConfigTemplate`，保留动态列表、统计表格和专用交互逻辑；开发用 Demo 页保持不变。
- 调整（settings-template-rows）：按“组—行—项”重新整理快捷键、聊天设置、外观、通知、人格和日志页；拆分默认模型、STT、录音参数、记忆设置、日志容量等独立设置项，取消 API/Codex 动态模型编辑器中的多字段并排布局，保留原有事件、保存和动态管理逻辑。

## 修复：计划确认提示词仅发送一次

- 修复（plan-confirm-one-shot-prompt）：用户确认执行计划后，计划路径提示词仅随该次继续执行请求发送；后续普通消息与上下文压缩不再重复回灌活动计划。

## 优化：聊天模型名称紧凑显示

- 优化（chat-model-label-separator）：聊天输入栏的模型名称以中点分隔供应商、模型和思维强度；供应商在紧凑栏内最多显示两个字符，完整名称仍可通过悬浮提示和下拉菜单查看。
- 修复（chat-status-banner-info-tone）：补齐聊天状态横幅的 `info` 类型，并让上下文压缩状态显式使用该样式。

## 调整：协调与远程客服预设权限白名单

- 调整（leader-and-remote-customer-service-whitelists）：leader 与远程客服部门的默认权限改为白名单；leader 保留协调、核验和联网检索所需的最小工具，远程客服保留新闻检索、媒体阅读、表情包和图片创作能力。所有内置预设部门的默认 Skill 白名单均包含 `memory-generation`，远程客服额外启用 `news-analyst`。

## 修复：删除部门时同步断开树关系

- 修复（department-delete-prunes-tree-edges）：部门设置页删除自定义部门时，会同步移除所有指向该部门的直属关系；不再因保存时还原旧树边而触发无效子部门校验、导致删除无法保存。

## 调整：预设部门可配置且可还原

- 调整（preset-department-defaults-not-locks）：explorer、reviewer 与 saddler 的名称、说明、模型、模型兜底和权限从强制策略改为可编辑的默认值；加载和保存不再覆盖用户配置。点击“还原”时，前端只传部门 ID，并经统一传输接口从后端获取唯一的完整默认草稿（桌面与 Web 共用），恢复对应的模型与权限白名单。

## 更新：固定预设部门与能力资产约束

- 功能（fixed-preset-departments）：新增固定的 explorer、reviewer 与 saddler 部门，将其固定为主助理的直属下级，并预置各自独立的模型、工具和 Skill 白名单；配置加载会自动补齐或修正这些内置部门与关联。
- 修复（saddler-pai-scope）：为 saddler 部门的文件与终端工具增加运行时范围校验，仅允许在当前项目 `.pai/` 目录内写入或更新能力资产。

## 发布：v0.40.0

- 修复：发送消息时不再在附件提示阶段检查文件是否存在，减少 prompt 构建阶段的磁盘访问和额外耗时。
- 修复：群聊远程应答默认不再对超长回复发起二次快速模型改写，模型回复完成后直接进入落库与外发流程。
- 修复：文件传输入口收口为路径/分块统一方案，粘贴文件不再走整文件 base64 主路径，并补上附件超时清理与提交竞态保护。
- 修复：会话草稿会保留已落盘但尚未生成 base64 预览的图片附件，切换会话后不再丢失。
- 新增：加入独立文生图功能域与“图像生成”工具，支持本地 ComfyUI、OpenAI GPT Image、xAI Grok Imagine、火山方舟 Seedream 和 Gemini Nano Banana 2；配置页可管理独立供应商/模型与 ComfyUI 工作流，生成结果安全保存并显示自 PAI 助理空间。
- 新增：加入独立 `image_edit` 图像编辑工具，支持对话附件驱动的局部修改、局部重绘、扩图和多图融合，并按供应商能力复用现有生图模型与认证链路。
- 调整：生图模型默认宽高比统一为 `1:1`，默认分辨率统一为 `512x512`；供应商模板和设置页测试预设同步更新。
- 调整：生图供应商默认超时改为 600 秒，配置上限收敛为 10 分钟，移除原先 300 秒默认超时。
- 调整：生图工具参数收敛为“提示词 → 分辨率”，移除无独立意义的宽高比参数，并向 LLM 明确默认分辨率 `512x512`。
- 优化：默认生图模型与多模态分析模型选择“不配置”时，不再向模型提供 image_generate / read_media 工具。
- 修复：清理 DaisyUI v4 残留类 form-control / label-text（v5 已移除），修复配置页、委派面板、工作目录选择等多处标签与输入框挤在同一行的布局错乱。

## 发布：v0.37.3

## 图片预览更好用了

- 点击图片预览外的空白区域，或者按键盘的 Esc 键，预览会直接关闭，不用再找关闭按钮。

## 发送消息更快了

- 发送带附件的消息时，去掉了发送前的文件二次核对环节，整体发送速度更快，特别是发送大型文件或批量消息时感受更明显。

## 群聊远程回复更直接

- 群聊中收到超长回复时，默认不会再额外发起二次改写，大段内容也会直接落库发出来，不再被额外打断。

## 群聊入场更稳定

- 修复了群聊中被唤醒时，入场摘要偶尔丢失的问题。
