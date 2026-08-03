# 未发布

## 修复

- 整体删除旧布局（app_data.json 单文件 / app_data/ 子目录）的读取与迁移链：`read_legacy_app_data` / `read_legacy_split_app_data` / `legacy_app_data_split_*` 路径辅助全部删除，`read_app_data` 退化为纯分片聚合（等价 `read_layout_app_data`）；V1 baseline 迁移 7 步（内置 agents 补全、会话元数据补全、头像路径迁移、归档合并、内联媒体外置、主会话标记归一、工具评审遗留清理）及 `migrate_legacy_app_data_if_needed` 显式迁移入口删除，启动门闩与 bootstrap 快照不再执行旧布局迁移；`DATA_MIGRATION_VERSION_V1_BASELINE` 常量删除，最低迁移版本从 V2 起算；`AppData.archived_conversations` 字段删除（仅旧归档合并使用）。产品不再支持旧布局直接升级，代码中不存在任何读取旧数据文件的路径。
- 废弃 `user_alias` 旧字段：旧数据迁移时直接抛弃 `user_alias`，不再拷贝进运行时状态（`RuntimeStateFile` 删除该字段及 `build_runtime_state_file` / `apply_runtime_state_to_app_data` / `read_layout_app_data` 中的拷贝逻辑）；业务消费方一律改从 agents 用户人格（user-persona）的 name 取昵称——历史记忆索引签名/渲染、微信入站联系人显示名、IDE persona、压缩兜底、聊天准备快照全部切换，`AppData.user_alias` 仅保留用于旧文件反序列化兼容，不再流向运行时。
- 剥离旧数据（app_data.json）的非迁移读取：旧布局数据只允许在迁移流程中被读取——新增显式迁移入口 `migrate_legacy_app_data_if_needed`（幂等，检测旧布局存在时读入 app_data.json 并触发 V1 迁移写回分片），启动门闩与 bootstrap 快照在业务读取前执行；删除 `read_agents_shard` / `read_runtime_state_shard` / `read_conversation_meta_shard` / `read_conversation_shard_raw` / `collect_chat_index_items_from_storage` 六处 legacy 兜底分支，业务读取一律只走分片；MCP/Skills 刷新不再整读 `read_app_data`，改用 `state_read_agents_cached`。迁移完成后 AppData 数据不再进入内存缓存。
- exec 工具的系统提示词补充 rg 使用约定：明确 `rg -r` 是 `--replace`（必须带参数），`-rn` 会被解析成 `-r n` 把匹配替换成字符 `n` 输出产生假结果；统一使用 `rg -n` 搜索（rg 默认递归），避免短选项拼接陷阱。
- 修复压缩/归档请求体预览与实际执行不一致的问题：`get_prompt_preview` 的 compaction/archive 预览此前仍走独立的 `SummaryContext` 模式构建（独立 system 模板、清空历史媒体），与已切换为 Chat 模式的实际压缩执行产生差异，预览展示的请求体与实际发出不一致；现预览分支改为与实际压缩完全同构（Chat 模式 + `LatestUserPayloadIntent::SummaryContext` 注入、data_path/工具参数一致），并清理 `PromptBuildMode::SummaryContext` 全部死分支（枚举变体、构建分支、模式解析、system 快照与 preamble 中的模式判断）。
- 修复压缩/归档请求与正常对话请求结构不一致导致供应商缓存命中率低的问题：压缩路径原来自建 `SummaryContext` 独立模式，system 区块、user_alias、user_intro、response_style_id、tools、记忆注入全部与正常对话不一致，缓存前缀必然 miss；现压缩改为与正常对话完全一致的 `Chat` 模式，仅最后一条 user 消息注入压缩指令——user_alias/user_intro 改从 agents 用户人格读取（废弃 `runtime.user_alias` 残留）、response_style_id 改读运行态、tool_session_id 改用 `inflight_chat_key(agent.id, conversation_id)` 格式（plan/task 工具不再被 policy 过滤，工具数从 24 恢复到 26）、data_path 传 `Some(&state.data_path)` 修复记忆召回、压缩路径同步透传工具定义；实测缓存命中从 0 提升到 98.65%（system/tools/messages 静态部分完全一致）。
- 修复空 user 消息被错误补空格的问题：临时消息块（计划/goal 提示词渲染为空）和纯空消息不该生成 `user ' '`，补空格只在「有图片/音频但无文本」时才有意义；历史消息补位、latest_user 补位、genai 序列化层三处条件统一改为「文本空且带媒体才补空格」，并删除 latest user 文本块全空兜底补 `' '` 的逻辑，避免请求里出现脏空消息。
- 修复压缩请求 system 尾部多一个换行的问题：`build_genai_chat_request` 判断空用 trim、发送用原值，导致压缩与正常对话 system 字符数不一致，改为 trim 后发送。
- 修复上下文压缩/归档输入被错误精简导致历史消息丢失的问题：07-10 的 remote-im 提交把压缩/归档输入从「最后 block 完整消息」换成固定 10K 字符的保留对话读取器并过滤旧压缩消息，导致长会话压缩时前面的消息全部丢失、后面的消息被裁剪、上一轮摘要消失；已恢复 `read_archive_pipeline_last_block_conversation`（读取最后 block 完整消息、不过滤压缩消息），删除远程唤醒精简读取器（远程唤醒早已移除 LLM 压缩，该读取器为纯残留），压缩与归档输入恢复完整历史。
- 修复 SummaryContext 归档压缩 JSON 解析失败无法定位的问题：模型将 `openLoops` 输出为 `[{"loop": "..."}]` 对象形态，与契约要求的字符串数组不符，`MemoryCurationDraft` 反序列化报 `invalid type: map, expected a string` 整段失败；解析器新增 `deserialize_open_loops` 兼容两种形态（纯字符串直接收、对象形态提取 `loop` 字段），无效元素跳过不中断；json_contract 明确 `openLoops` 元素必须是纯字符串；失败日志不再截断 raw（去掉 `chars().take(240)`），完整保留模型原始输出便于定位。
- 消除流式输出期间高频 channel 事件的大 payload IPC 阻塞：普通正文 delta 与思维链 delta（逐 token 高频事件）不再随事件下发 `stream_cache` 快照——前端只要看到 streamCache 就走 `reduceStreamSnapshot` 全量覆盖路径，轻量快照的空正文会把流式内容覆盖成空白，因此高频事件直接不带快照，前端走增量渲染逐字累积，每个事件 payload 降到只有 delta 文本本身；完整快照仍保留给低频关键事件（工具调用/结果/状态）做权威校正，后端缓存照常全量更新、恢复查询不受影响。事件远低于 Tauri 8KB 阈值，改走 eval 快路径而非 fetch 路径，消除 `sendIpcMessage` 高频同步阻塞。
- 消除聊天虚拟滚动在流式输出期间的 Layout Thrashing（强制同步布局）：`measureElement` 不再每次无条件 `getBoundingClientRect` 读几何属性——tanstack 内部 ResizeObserver 触发时用 `borderBoxSize` 异步测量，主动测量优先读 `measuredVirtualItemHeights` 缓存、仅首次挂载才读 DOM；`:ref` 回调 `measureVirtualRow` 对「同一元素 + 已有缓存高度」直接短路，跳过 DOM 读取与 `measureElement`，尺寸变化由 ResizeObserver 异步兜底；`handleVirtualItemResize` 调整为先更新缓存再测量，避免缓存分支读到旧值导致 virtualizer 布局不更新。
- 修复 Web 端消息图片无法显示的问题：移除「Web 端禁止读取本地图片路径」的多余权限限制（应用本身具备文件浏览器与宿主文件读写能力，此限制与产品定位矛盾），`read_local_chat_image_thumbnail` / `read_local_chat_image_original` 从 Web native-only 名单移除并接入 Web dispatcher 转发，前端读取聊天图片不再因 Web 环境返回空；图片附件路径保持真实落盘路径。
- 修复 Web dispatcher native-only 名单遗漏 `clear_window_chat_view_stream_bindings_command`：该窗口流绑定清理命令此前既无 Web 分支也未显式拒绝，与 bind/unbind 同类归入 native-only。
- 修复 Web 端（VS Code 侧边栏 / 远程 bridge）调用 `show_quick_setup_window`、`complete_quick_setup_and_open_chat` 未被明确拒绝的问题：Web dispatcher 的 native-only 命令清单补齐这两个本机窗口命令，与前端传输适配器边界一致。
- 修复新建“隔离工作树”会话时 Git 根目录二次校验会弹出控制台窗口的问题：Windows 下以 `CREATE_NO_WINDOW` 执行校验进程。
- 修复后台子进程弹出控制台窗口的遗漏点：Git 幽灵快照、VSCode 桥接网络探测、winget 安装、WSL/Shell 终端启动器、默认程序打开文件，均以 `CREATE_NO_WINDOW` 抑制多余控制台窗口。
- 聊天消息无头像时不再渲染头像占位（含首字母兜底），用户消息靠右、助理名称靠左布局不变。
- 修复 MCP 工具权限兼容名回归：规范化组成员工具命名后丢失了旧格式候选名（`server-id::search` 等 `server_id::provider_tool_name` 组合），已补回。
- 修复简单设置面板双滚动条：外层容器不再叠加 `overflow-y-auto`，滚动统一由面板内通用模板接管。
- 「保存并开始对话」不再自动创建普通会话：默认只保留系统通知会话，由用户在会话列表手动新建（新建后自动切换）。
- 修复简单设置保存成功后状态提示显示原始 key `status.saved`：三语言 `status` 区块补充 `saved` 文案。
- 修复简单设置保存后助理部门挂载多个模型：部门 `apiConfigIds` 只保留专家模型（之前会额外挂载快速/多模态模型），工具评审模型仍指向快速模型。
- 修复助理人格无默认头像：未设置头像的非用户人格，后端 `read_avatar_data_url` 直接返回内置品牌图标（与历史消息图片生成共用同一资源），前端不再做头像降级渲染。
- 内置助理人格（default-agent）默认名称改为「Pai」：仅影响新建默认人格，不迁移已有数据。
- 简单设置外观区块调整：主题切换改为头部左上角太阳/月亮图标按钮（swap 翻转动画 + 「切换到亮色/暗色」文字，与标题栏模式切换同款样式），外观卡内只保留语言选择。
- 修复「保存并开始对话」后设置窗未关闭：保存成功后打开对话窗并隐藏设置窗。
- 修复模型选择下拉被卡片裁剪、超出屏幕无法选择的问题：`ApiConfigTreeSelect` 下拉面板改为 Teleport 到 body + popover 顶层显示 + fixed 定位（下方空间不足自动向上展开），并监听滚动与窗口大小变化保持面板跟随触发按钮，设置窗「模型分工」、对话页批量归档、部门模型分配等使用该组件的下拉一并修复。
- 修复粘贴文本永远进主会话输入框的问题：全局粘贴处理在焦点位于会话输入框（含侧边追问会话）时放行文本，由浏览器原生行为落到焦点所在的输入框，不再被拦截后固定写入主会话；图片文件粘贴仍统一进入附件队列。
- 修复侧边追问会话无法撤回消息的问题：消息气泡底部撤回/重新生成按钮与右键菜单撤回在侧边会话中事件断链（未绑定 `recallTurn`/`regenerateTurn`），已接入与主会话一致的撤回链路，确认弹窗按焦点会话查询撤回预览。
- 修复侧边追问会话右键「从消息创建分支」无效的问题：侧边会话未绑定分支创建事件，右键菜单分支点击无反应；已接通无弹窗一键创建链路，创建后自动刷新侧边会话列表并切换到新分支。
- 修复 `ApiConfigTreeSelect` 多根节点（div + Teleport）导致非 prop 属性无法自动继承的问题：外部传入的 `id`（如批量归档模型选择的 label 关联目标）不再触发 Vue extraneous attrs 警告，`inheritAttrs: false` + 根节点 `v-bind="$attrs"` 手动承接。
- 修复流式输出期间日志无限刷 `TAURI Couldn't find callback id` 导致前端卡死：前端页面重载（HMR / 手动刷新 / 崩溃重建）后旧 bindingId 的流式 channel 注册残留在 Rust 侧，且 `Channel::send` 在 JS callback 失效时仍返回 Ok，僵尸注册无法自动清理；现于窗口启动/重载时（`appBootstrapMount`）先清理本窗口残留流式绑定，再重新绑定新 channel。
- 降低折叠思维链时的流式渲染开销：思维链面板折叠时跳过全文签名计算（`activityPanelMemoKey` 不再对思维链全文做哈希与全量转换），避免每次 delta 都做全文哈希与全量转换；`activityItemsSignature` 保留内容哈希，确保工具结果等长内容替换仍可感知。
- 修复上下文压缩完成后标题栏用量圆环不归零的问题：压缩消息是追加在消息末尾的最新一条（无 usage 字段），用量计算从尾部往前找 assistant 消息时跳过它、回退到压缩前旧消息的高占用率；现识别压缩消息（`context_compaction` / `summary_context_seed`）后直接按 20k tokens 折算占用率，不再回退到压缩前旧值。
- 修复压缩卡片右上角用量百分比与标题栏不一致的问题：压缩卡片此前在 blockPage 消息里独立重算占用率（同样会跳过压缩消息回退旧值），现改为直接复用标题栏同一数据源 `chatUsagePercent`，两处始终一致。
- 工具执行期间标题栏用量圆环动态更新：工具结果落盘时把上下文用量写入流式缓存（`stream_cache`），随后续每个 delta 事件下发，前端实时更新占用率；切屏恢复时也直接来自流式缓存，无需旁路广播 `context_usage_update` 事件。
- 设置窗标题栏「切换到简单」按钮改用主题色（`btn-primary`），不再使用 ghost 样式。
- 移除设置窗标题栏「开始对话」按钮（原入口过时），按钮迁移至欢迎页主内容区右上角（原「打开快速设置」位置），点击直接打开对话窗。
- 设置页「快捷键」内置快捷键列表补充两条遗漏：`Shift + Tab` 切换计划模式、`Alt + Z` 切换代码预览自动换行。

## 依赖

## 功能

- 设置窗口新增「简单 / 高级」模式切换：全新用户默认简单模式，老用户默认高级模式；简单模式为单页紧凑表单（供应商 + 快速/专家/多模态三模型卡 + 对话风格 + 语言 + 明暗主题 + Shell + 热键 + 硅基流动折叠区），支持 localStorage 草稿态，保存成功后清除。
- 移除独立「快速设置」窗口：无可用 LLM 时启动改开设置窗口；左上角更新日志入口移入「关于 → 版本更新」区块并自动加载。
- 简单设置面板优化：切换 DeepSeek / OpenCode 时自动填充对应默认模型，自定义供应商清空模型并提供「刷新模型」拉取候选列表（复用 `refresh_models`）；多模态卡不显示能力开关（默认支持工具调用，图片/音频/视频能力默认全开），DeepSeek 下不提供多模态输入；对话风格固定为「无要求」不再提供选项；外观区块整体移至表单最前；移除 Shell 选项（保持默认 auto，保存不覆盖已有设置）；修复面板上下边距；供应商入口暂只保留 DeepSeek 与自定义（OpenCode 定义保留备用）；改用设置页通用模板，保存按钮与状态提示固定头部，并提示保存将覆盖原有快速 / 专家模型设置。
- 左上角「简单 / 高级」切换按钮改版：双按钮分段控件改为单按钮（swap 图标翻转动画 + 「切换到高级 / 简单」文字提示），点击即切换。
- 首次启动引导流程：无可用 LLM 时强制进入简单设置模式（不再显示复杂设置卡）；简单设置保存按钮改为「保存并开始对话」，保存成功后自动打开对话窗口；设置窗口标题栏新增「开始对话」按钮，可随时跳转对话窗。
- 修复简单设置模式开关不生效：前端加载配置时未回填 `simpleSetupMode` 字段、保存时也未随配置提交，导致首屏永远显示复杂模式；已补齐加载回填与保存提交链路。
- 首次启动默认主题跟随系统明暗：亮色系统使用「秋日」主题、暗色系统使用「森林」主题；简单设置里的明暗主题按钮同步改为秋日 / 森林。
- 抽取共用模型卡片组件 `ApiModelCard`：供应商页与简单设置面板共用同一模型卡（模型输入 + 候选下拉 + 能力开关 + 思考等级），供应商页原卡片逻辑保持不变，清理被组件接管后的前端死代码。
