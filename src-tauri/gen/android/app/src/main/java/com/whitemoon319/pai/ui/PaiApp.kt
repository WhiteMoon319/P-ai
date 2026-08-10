package com.whitemoon319.pai.ui

import com.whitemoon319.pai.model.ActivityStep
import com.whitemoon319.pai.model.ChatMessage
import com.whitemoon319.pai.model.ConversationSummary
import com.whitemoon319.pai.model.buildActivityStepsFromMessage
import com.whitemoon319.pai.viewmodel.AppViewModel
import com.whitemoon319.pai.ws.ConnectionStatus
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.whitemoon319.pai.ui.richtext.MarkdownText
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaiApp(vm: AppViewModel) {
    DisposableEffect(Unit) {
        vm.start()
        onDispose { vm.stop() }
    }

    // 错误反馈：一次性 Toast 提示（新建失败/刷新失败等），避免静默无反应
    val errorMsg by vm.error.collectAsState()
    val context = androidx.compose.ui.platform.LocalContext.current
    LaunchedEffect(errorMsg) {
        val msg = errorMsg ?: return@LaunchedEffect
        android.widget.Toast.makeText(context, msg, android.widget.Toast.LENGTH_SHORT).show()
        vm.consumeError()
    }

    // 不透明背景盖住底层 Rust WebView，避免透出可交互残留
    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.surface,
    ) {
        var inChat by remember { mutableStateOf(false) }
        var title by remember { mutableStateOf("会话") }
        var showSettings by remember { mutableStateOf(false) }
        val scope = rememberCoroutineScope()

        when {
            showSettings -> {
                SettingsScreen(
                    vm = vm,
                    onBack = { showSettings = false },
                )
            }
            inChat -> {
                ChatScreen(
                    vm = vm,
                    title = title,
                    onBack = {
                        inChat = false
                        title = "会话"
                    },
                    onSettings = { showSettings = true },
                )
            }
            else -> {
                ConversationListScreen(
                    vm = vm,
                    onOpen = { conv ->
                        scope.launch {
                            vm.openConversation(conv.conversationId)
                        }
                        title = conv.title ?: conv.conversationId
                        inChat = true
                    },
                    onNew = {
                        scope.launch {
                            val id = vm.createConversation(title = null)
                            if (id != null) {
                                title = "新会话"
                                inChat = true
                            }
                        }
                    },
                    onCreated = { id ->
                        title = "新会话"
                        inChat = true
                    },
                    onSettings = { showSettings = true },
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ConnectionBanner(vm: AppViewModel) {
    val connection by vm.connectionState.collectAsState()
    // 本地模式下 ws 稳定连接是常态，无需横幅；仅在出现异常时提示
    if (connection != ConnectionStatus.Disconnected) return
    Surface(color = MaterialTheme.colorScheme.errorContainer) {
        Text(
            text = "后端未连接，会话可能不可用",
            modifier = Modifier.fillMaxWidth().padding(8.dp),
            textAlign = TextAlign.Center,
            style = MaterialTheme.typography.labelMedium,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConversationListScreen(
    vm: AppViewModel,
    onOpen: (ConversationSummary) -> Unit,
    onNew: () -> Unit,
    onCreated: (String) -> Unit = {},
    onSettings: () -> Unit = {},
) {
    ConversationListScreenImpl(vm = vm, onOpen = onOpen, onNew = onNew, onCreated = onCreated, onSettings = onSettings)
}

/**
 * 列表 + 新建选择对话框：点「新建」先拉 createOptions 可选项，用户自选部门/人格后创建。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ConversationListScreenImpl(
    vm: AppViewModel,
    onOpen: (ConversationSummary) -> Unit,
    onNew: () -> Unit,
    onCreated: (String) -> Unit,
    onSettings: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val conversations by vm.conversations.collectAsState()
    val loading by vm.loading.collectAsState()
    var showNewDialog by remember { mutableStateOf(false) }
    var fullOptions by remember { mutableStateOf<com.whitemoon319.pai.model.CreateConversationOptions>(com.whitemoon319.pai.model.CreateConversationOptions()) }
    var optionsLoading by remember { mutableStateOf(false) }
    var selectedOption by remember { mutableStateOf<com.whitemoon319.pai.model.CreateConversationOptionItem?>(null) }

    LaunchedEffect(Unit) {
        vm.refreshConversations()
    }

    // 点新建：拉选项并打开选择对话框
    fun openNewDialog() {
        scope.launch {
            optionsLoading = true
            fullOptions = vm.fetchCreateOptionsFull()
            optionsLoading = false
            val list = fullOptions.departments
            // 有可选项才弹选择框；为空时退回默认创建（保留原行为）
            if (list.isEmpty()) {
                onNew()
            } else {
                // 预选默认部门/人格（若在列表中）
                selectedOption = list.firstOrNull {
                    it.departmentId == fullOptions.defaultDepartmentId &&
                        it.agentId == fullOptions.defaultAgentId
                } ?: list.first()
                showNewDialog = true
            }
        }
    }

    Column(Modifier.fillMaxSize()) {
        ConnectionBanner(vm)
        TopAppBar(
            title = { Text("P-AI 会话") },
            navigationIcon = {
                IconButton(onClick = { scope.launch { vm.refreshConversations() } }) {
                    Icon(Icons.Default.Refresh, contentDescription = "刷新")
                }
            },
            actions = {
                IconButton(onClick = { onSettings() }) {
                    Icon(Icons.Default.Settings, contentDescription = "设置")
                }
                IconButton(onClick = { openNewDialog() }) {
                    Icon(Icons.Default.Add, contentDescription = "新建")
                }
            },
        )
        when {
            loading && conversations.isEmpty() -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }
            conversations.isEmpty() -> {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("暂无会话，点右上角 + 新建")
                }
            }
            else -> {
                LazyColumn(Modifier.fillMaxSize()) {
                    items(conversations, key = { it.conversationId }) { conv ->
                        ConversationRow(conv, onClick = { onOpen(conv) })
                        HorizontalDivider()
                    }
                }
            }
        }
    }

    if (showNewDialog) {
        val list = fullOptions.departments
        // 按部门分组，保持后端顺序
        val grouped = list.groupBy { it.departmentName ?: it.departmentId ?: "其他" }
        AlertDialog(
            onDismissRequest = { showNewDialog = false },
            title = { Text("新建会话") },
            text = {
                when {
                    optionsLoading -> Box(
                        Modifier.fillMaxWidth().height(160.dp),
                        contentAlignment = Alignment.Center,
                    ) { CircularProgressIndicator() }
                    list.isEmpty() -> Text("没有可用的部门/人格，请先在电脑端配置。")
                    else -> LazyColumn(
                        Modifier.fillMaxWidth().heightIn(max = 380.dp),
                    ) {
                        grouped.forEach { (department, items) ->
                            item(key = "hdr_$department") {
                                Text(
                                    department,
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.padding(vertical = 6.dp),
                                )
                            }
                            items.forEach { opt ->
                                item(key = opt.id ?: opt.agentId ?: opt.departmentId ?: "opt") {
                                    val selected = selectedOption?.let { s ->
                                        s.agentId == opt.agentId && s.departmentId == opt.departmentId
                                    } == true
                                    Row(
                                        Modifier
                                            .fillMaxWidth()
                                            .clickable {
                                                selectedOption = opt
                                            }
                                            .padding(vertical = 6.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        RadioButton(
                                            selected = selected,
                                            onClick = { selectedOption = opt },
                                        )
                                        Spacer(Modifier.width(8.dp))
                                        Column {
                                            Text(
                                                opt.agentName ?: opt.agentId ?: "人格",
                                                style = MaterialTheme.typography.bodyMedium,
                                            )
                                            if (selected) {
                                                Spacer(Modifier.height(2.dp))
                                                Text(
                                                    "默认方案",
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = MaterialTheme.colorScheme.primary,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !optionsLoading && selectedOption != null,
                    onClick = {
                        val opt = selectedOption
                        showNewDialog = false
                        if (opt != null) {
                            scope.launch {
                                val id = vm.createConversation(
                                    title = null,
                                    departmentId = opt.departmentId,
                                    agentId = opt.agentId,
                                )
                                if (id != null) onCreated(id)
                            }
                        }
                    },
                ) { Text("创建") }
            },
            dismissButton = {
                TextButton(onClick = { showNewDialog = false }) { Text("取消") }
            },
        )
    }
}

@Composable
fun ConversationRow(conv: ConversationSummary, onClick: () -> Unit) {
    Column(
        Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(12.dp)
    ) {
        Text(conv.title ?: "无标题", style = MaterialTheme.typography.titleMedium)
        val preview = conv.previewMessages?.lastOrNull()?.textPreview
        if (!preview.isNullOrBlank()) {
            Spacer(Modifier.height(2.dp))
            Text(
                preview,
                maxLines = 2,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (conv.unreadCount > 0) {
            Text(
                "未读 ${conv.unreadCount}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(
    vm: AppViewModel,
    title: String,
    onBack: () -> Unit,
    onSettings: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    val messages by vm.messages.collectAsState()
    val streaming by vm.streamingText.collectAsState()
    val activitySteps by vm.activitySteps.collectAsState()
    val isStreaming by vm.isStreaming.collectAsState()
    val loading by vm.loading.collectAsState()
    var input by remember { mutableStateOf("") }
    val listState = rememberLazyListState(
        initialFirstVisibleItemIndex = Int.MAX_VALUE // 首帧即锚定列表末尾，避免顶部闪现/滑动
    )

    // 消息/流式输出/活动步骤变化时自动滚动到最底部
    LaunchedEffect(messages.size, streaming.length, activitySteps.size) {
        if (!loading && messages.isNotEmpty()) {
            val last = listState.layoutInfo.totalItemsCount - 1
            if (last >= 0) {
                // 流式输出中用平滑动画，其余（进入/加载完成/追加消息）瞬移到最底部，避免滑动过程
                if (isStreaming) {
                    listState.animateScrollToItem(last)
                } else {
                    listState.scrollToItem(last)
                }
            }
        }
    }

    Column(Modifier.fillMaxSize()) {
        ConnectionBanner(vm)
        TopAppBar(
            title = { Text(title) },
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                }
            },
            actions = {
                IconButton(onClick = onSettings) {
                    Icon(Icons.Default.Settings, contentDescription = "设置")
                }
            },
        )
        if (loading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxWidth(),
            ) {
                items(messages, key = { it.id }) { msg ->
                    MessageBubble(msg)
                }
                if (activitySteps.isNotEmpty()) {
                    item(key = "thinking") {
                        ThinkingBlock(steps = activitySteps)
                    }
                }
                if (streaming.isNotEmpty()) {
                    item(key = "streaming") {
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant,
                            shape = MaterialTheme.shapes.medium,
                            modifier = Modifier.padding(12.dp),
                        ) {
                            MarkdownText(content = streaming, modifier = Modifier.padding(10.dp))
                        }
                    }
                }
            }
            Row(
                Modifier
                    .fillMaxWidth()
                    .imePadding()
                    .padding(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = input,
                    onValueChange = { input = it },
                    modifier = Modifier.weight(1f),
                    placeholder = { Text("输入消息…") },
                    maxLines = 4,
                )
                Spacer(Modifier.width(8.dp))
                if (isStreaming) {
                    IconButton(onClick = { scope.launch { vm.stopStreaming() } }) {
                        Icon(Icons.Default.Clear, contentDescription = "停止")
                    }
                } else {
                    IconButton(onClick = {
                        val text = input
                        if (text.isNotBlank()) {
                            input = ""
                            scope.launch { vm.sendMessage(text) }
                        }
                    }) {
                        Icon(Icons.Default.Send, contentDescription = "发送")
                    }
                }
            }
        }
    }
}

/**
 * 「思维活动」大类容器：承载一段回合内交错出现的思考与工具调用。
 * 两层折叠——大类可整体折叠/展开；大类内每个 step 又各自展开/折叠。
 * 语义参考 rikkahub 的 groupMessageParts（thinking 大类 → steps[]）。
 */
@Composable
private fun ThinkingBlock(steps: List<ActivityStep>) {
    // 大类默认折叠：思考/工具不占屏幕，需要时点开查看
    var groupExpanded by remember { mutableStateOf(false) }
    val color = MaterialTheme.colorScheme.outline
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        shape = MaterialTheme.shapes.medium,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
    ) {
        Column(Modifier.padding(8.dp)) {
            // 大类头部：显示思考/工具计数，点击整体折叠
            val reasoningCount = steps.count { it is ActivityStep.Reasoning }
            val toolCount = steps.count { it is ActivityStep.Tool }
            val title = "思考与工具 · ${reasoningCount}思考 ${toolCount}工具"
            Row(
                modifier = Modifier.fillMaxWidth().clickable { groupExpanded = !groupExpanded },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    if (groupExpanded) "▾ $title（点击收起）" else "▸ $title（点击展开）",
                    style = MaterialTheme.typography.labelMedium,
                    color = color,
                )
            }
            if (groupExpanded) {
                steps.forEachIndexed { index, step ->
                    ActivityStepRow(step)
                    if (index != steps.lastIndex) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f))
                    }
                }
            }
        }
    }
}

/** 大类内单个步骤：思考或工具，各自可展开/折叠。 */
@Composable
private fun ActivityStepRow(step: ActivityStep) {
    var expanded by remember { mutableStateOf<Boolean?>(null) } // null=跟随默认
    val color = MaterialTheme.colorScheme.outline
    when (step) {
        is ActivityStep.Reasoning -> {
            val isOpen = expanded ?: false
            Column(Modifier.fillMaxWidth().padding(top = 6.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth().clickable { expanded = !isOpen },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        if (isOpen) "▾ 思考" else "▸ 思考",
                        style = MaterialTheme.typography.labelSmall,
                        color = color,
                    )
                }
                if (isOpen) {
                    Text(
                        step.text,
                        Modifier.padding(start = 12.dp, top = 4.dp),
                        style = MaterialTheme.typography.bodySmall,
                        color = color,
                    )
                }
            }
        }
        is ActivityStep.Tool -> {
            val title = "${step.name ?: "工具"}" +
                (if (step.argsText.isNullOrBlank()) "" else "\n参数：${step.argsText}")
            val isOpen = expanded ?: false // 工具步骤默认折叠
            val statusText = when (step.status) {
                "done" -> "✓ 完成"
                "failed" -> "✗ 失败"
                "running" -> "⚙ 执行中"
                else -> "工具"
            }
            Column(Modifier.fillMaxWidth().padding(top = 6.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth().clickable { expanded = !isOpen },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "${if (isOpen) "▾" else "▸"} ${statusText} ${step.name ?: "工具"}",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSecondaryContainer,
                    )
                }
                if (isOpen) {
                    if (!step.reasoning.isNullOrBlank()) {
                        Text(
                            step.reasoning,
                            Modifier.padding(start = 12.dp, top = 4.dp),
                            style = MaterialTheme.typography.bodySmall,
                            color = color,
                        )
                    }
                    if (!step.argsText.isNullOrBlank()) {
                        Text(
                            step.argsText,
                            Modifier.padding(start = 12.dp, top = 4.dp),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSecondaryContainer,
                        )
                    }
                    if (!step.resultText.isNullOrBlank()) {
                        Text(
                            step.resultText,
                            Modifier.padding(start = 12.dp, top = 4.dp),
                            style = MaterialTheme.typography.bodySmall,
                            color = color,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ThinkingSectionMessage(steps: List<ActivityStep>) {
    if (steps.isEmpty()) return
    ThinkingBlock(steps = steps)
}

@Composable
fun MessageBubble(message: ChatMessage) {
    val isUser = message.role == "user"
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 4.dp),
        horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
    ) {
        Surface(
            color = if (isUser) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
            shape = MaterialTheme.shapes.medium,
            modifier = Modifier.widthIn(max = 320.dp),
        ) {
            val text = message.parts.joinToString("\n") { it.displayText }
            if (isUser) {
                Text(text, Modifier.padding(10.dp))
            } else {
                Column(Modifier.padding(10.dp)) {
                    // 落盘消息的思考+工具统一聚合为 thinking 大类（两级折叠），与流式一致
                    val steps = buildActivityStepsFromMessage(message)
                    if (steps.isNotEmpty()) {
                        ThinkingSectionMessage(steps)
                    }
                    if (text.isNotBlank()) {
                        MarkdownText(content = text)
                    }
                }
            }
        }
    }
}

// ==================== 设置 ====================

/** 设置一级页条目。 */
private enum class SettingsEntry(
    val title: String,
    val subtitle: String,
) {
    Api("模型与供应商", "供应商增删改、连接测试、启用切换"),
    Chat("聊天设置", "用户别名、回复风格、默认配置"),
    Tools("工具", "Android 沙盒工作区、工具状态"),
    About("关于", "版本、检查更新、仓库"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    vm: AppViewModel,
    onBack: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var current by remember { mutableStateOf<SettingsEntry?>(null) }
    val settingsLoading by vm.settingsLoading.collectAsState()
    val appConfig by vm.appConfig.collectAsState()
    val chatSettings by vm.chatSettings.collectAsState()
    val toolStatus by vm.toolStatus.collectAsState()
    val bootstrap by vm.bootstrap.collectAsState()
    // 工具状态需要 agent_id：优先当前会话的，没有则让后端用默认
    val agentId = remember {
        vm.currentConversationId.value?.let { id ->
            vm.conversations.value.firstOrNull { it.conversationId == id }?.agentId
        }
    }

    LaunchedEffect(Unit) {
        // 冷启动时 ws 可能尚未连接，一次性拉取会失败；改为监听连接建立后自动重试，
        // 同时保持进入即拉一次（已连接时立即生效）
        vm.loadSettings(agentId)
        vm.loadAboutInfo()
        vm.connectionState.collect { status ->
            if (status == ConnectionStatus.Connected) {
                vm.loadSettings(agentId)
                vm.refreshWorkspaceStatus()
            }
        }
    }

    val entry = current
    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text(entry?.title ?: "设置") },
            navigationIcon = {
                IconButton(onClick = { if (entry == null) onBack() else current = null }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                }
            },
        )
        when (entry) {
            null -> {
                // 一级：设置项列表
                Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                    SettingsEntry.entries.forEach { item ->
                        Card(
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp)
                                .clickable { current = item },
                        ) {
                            Row(
                                Modifier.padding(16.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Column(Modifier.weight(1f)) {
                                    Text(item.title, style = MaterialTheme.typography.titleSmall)
                                    Text(
                                        item.subtitle,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                                Text("›", style = MaterialTheme.typography.titleMedium)
                            }
                        }
                    }
                }
            }
            SettingsEntry.Api -> ApiSettingsTab(appConfig = appConfig, vm = vm)
            SettingsEntry.Chat -> ChatSettingsTab(settings = chatSettings, vm = vm)
            SettingsEntry.Tools -> ToolsSettingsTab(vm = vm, toolStatus = toolStatus)
            SettingsEntry.About -> AboutSettingsTab(bootstrap = bootstrap, vm = vm)
        }
    }
}


@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ApiSettingsTab(appConfig: com.whitemoon319.pai.model.AppConfig?, vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val saving by vm.settingsSaving.collectAsState()
    var editing by remember { mutableStateOf<com.whitemoon319.pai.model.ApiConfig?>(null) }
    var showEditor by remember { mutableStateOf(false) }
    var testingId by remember { mutableStateOf<String?>(null) }
    var testResult by remember { mutableStateOf<String?>(null) }
    var pendingDelete by remember { mutableStateOf<com.whitemoon319.pai.model.ApiConfig?>(null) }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("模型与供应商", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            Button(onClick = {
                editing = com.whitemoon319.pai.model.ApiConfig(
                    id = "api-config-${System.currentTimeMillis()}",
                    name = "新供应商",
                    requestFormat = "auto",
                    baseUrl = "https://api.openai.com/v1",
                    model = "gpt-4o-mini",
                )
                showEditor = true
            }) { Text("新增") }
        }
        Spacer(Modifier.height(8.dp))
        if (appConfig == null || appConfig.apiConfigs.isEmpty()) {
            Text("暂无供应商配置，点击右上角新增。", style = MaterialTheme.typography.bodySmall)
        } else {
            val currentId = appConfig.assistantDepartmentApiConfigId
                ?: appConfig.selectedApiConfigId
            appConfig.apiConfigs.forEach { api ->
                val isCurrent = currentId != null && currentId == api.id
                Card(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                ) {
                    Column(Modifier.padding(12.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f)) {
                                Text(api.name ?: "未命名", style = MaterialTheme.typography.titleSmall)
                                Text(
                                    "模型：${api.model ?: "—"} · 协议：${api.requestFormat ?: "auto"}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                if (!api.baseUrl.isNullOrBlank()) {
                                    Text(
                                        api.baseUrl,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                                if (isCurrent) {
                                    Text(
                                        "生效中",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                }
                            }
                            Row {
                                TextButton(
                                    enabled = !saving && testingId != api.id,
                                    onClick = {
                                        scope.launch {
                                            testingId = api.id
                                            testResult = vm.testApiConfigConnection(api)
                                            testingId = null
                                        }
                                    },
                                ) { Text(if (testingId == api.id) "测试中…" else "测试") }
                                if (!isCurrent) {
                                    TextButton(
                                        enabled = !saving,
                                        onClick = {
                                            scope.launch { vm.switchPrimaryApiConfig(api.id ?: return@launch) }
                                        },
                                    ) { Text("启用") }
                                }
                                TextButton(onClick = {
                                    editing = api
                                    showEditor = true
                                }) { Text("编辑") }
                                TextButton(onClick = { pendingDelete = api }) { Text("删除") }
                            }
                        }
                        testResult?.let { result ->
                            Spacer(Modifier.height(4.dp))
                            Text(
                                "测试结果：${result.take(120)}",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }
            }
        }
    }

    // 新增/编辑表单对话框
    if (showEditor && editing != null) {
        ApiConfigEditorDialog(
            config = editing!!,
            onDismiss = { showEditor = false },
            onSave = { updated ->
                scope.launch {
                    val isNew = appConfig?.apiConfigs?.none { it.id == updated.id } != false
                    val ok = if (isNew) vm.createApiConfig(updated) else vm.updateApiConfig(updated)
                    if (ok) showEditor = false
                }
            },
        )
    }

    // 删除确认
    pendingDelete?.let { target ->
        AlertDialog(
            onDismissRequest = { pendingDelete = null },
            title = { Text("删除供应商") },
            text = { Text("确定删除「${target.name ?: target.id}」吗？删除后关联引用会被清理。") },
            confirmButton = {
                TextButton(onClick = {
                    scope.launch {
                        val ok = vm.deleteApiConfig(target.id ?: "")
                        if (ok) pendingDelete = null
                    }
                }) { Text("删除") }
            },
            dismissButton = {
                TextButton(onClick = { pendingDelete = null }) { Text("取消") }
            },
        )
    }
}

/** 供应商新增/编辑表单。 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ApiConfigEditorDialog(
    config: com.whitemoon319.pai.model.ApiConfig,
    onDismiss: () -> Unit,
    onSave: (com.whitemoon319.pai.model.ApiConfig) -> Unit,
) {
    var name by remember { mutableStateOf(config.name ?: "") }
    var requestFormat by remember { mutableStateOf(config.requestFormat ?: "auto") }
    var baseUrl by remember { mutableStateOf(config.baseUrl ?: "") }
    var apiKey by remember { mutableStateOf(config.apiKey ?: "") }
    var model by remember { mutableStateOf(config.model ?: "") }
    var temperature by remember { mutableStateOf(config.temperature.toString()) }
    var contextWindow by remember { mutableStateOf(config.contextWindowTokens.toString()) }
    var maxOutput by remember { mutableStateOf(config.maxOutputTokens.toString()) }

    val requestFormats = listOf(
        "auto", "openai", "deepseek", "openai_responses", "codex", "gemini", "anthropic",
        "fireworks", "together", "groq", "mimo", "minimax", "moonshot", "nebius", "xai",
        "zai", "bigmodel", "aliyun", "baidu", "cohere", "ollama", "ollama_cloud", "vertex",
        "github_copilot", "opencode_go", "bedrock_api",
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (config.name.isNullOrBlank()) "新增供应商" else "编辑供应商") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()).heightIn(max = 420.dp)) {
                OutlinedTextField(
                    value = name, onValueChange = { name = it },
                    label = { Text("名称") }, modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(
                    value = model, onValueChange = { model = it },
                    label = { Text("模型") }, modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(
                    value = baseUrl, onValueChange = { baseUrl = it },
                    label = { Text("Base URL") }, modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(
                    value = apiKey, onValueChange = { apiKey = it },
                    label = { Text("API Key") }, modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(6.dp))
                OutlinedTextField(
                    value = requestFormat, onValueChange = { requestFormat = it },
                    label = { Text("协议格式") }, modifier = Modifier.fillMaxWidth(),
                    supportingText = {
                        Text(
                            requestFormats.joinToString(" / "),
                            style = MaterialTheme.typography.labelSmall,
                        )
                    },
                )
                Spacer(Modifier.height(6.dp))
                Row {
                    OutlinedTextField(
                        value = temperature, onValueChange = { temperature = it },
                        label = { Text("温度") }, modifier = Modifier.weight(1f).padding(end = 4.dp),
                    )
                    OutlinedTextField(
                        value = contextWindow, onValueChange = { contextWindow = it },
                        label = { Text("上下文窗口") }, modifier = Modifier.weight(1f).padding(horizontal = 4.dp),
                    )
                    OutlinedTextField(
                        value = maxOutput, onValueChange = { maxOutput = it },
                        label = { Text("最大输出") }, modifier = Modifier.weight(1f).padding(start = 4.dp),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = {
                onSave(
                    config.copy(
                        name = name.ifBlank { "未命名" },
                        requestFormat = requestFormat.ifBlank { "auto" },
                        baseUrl = baseUrl,
                        apiKey = apiKey,
                        model = model,
                        temperature = temperature.toDoubleOrNull() ?: 0.7,
                        contextWindowTokens = contextWindow.toIntOrNull() ?: 128000,
                        maxOutputTokens = maxOutput.toIntOrNull() ?: 8192,
                    )
                )
            }) { Text("保存") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消") }
        },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ChatSettingsTab(settings: com.whitemoon319.pai.model.ChatSettings?, vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val saving by vm.settingsSaving.collectAsState()
    var alias by remember { mutableStateOf(settings?.userAlias ?: "") }
    var styleId by remember { mutableStateOf(settings?.responseStyleId ?: "") }
    // settings 异步到达后回填（首次进入时可能为 null）
    LaunchedEffect(settings) {
        if (settings != null) {
            alias = settings.userAlias ?: ""
            styleId = settings.responseStyleId ?: ""
        }
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        Text("聊天设置", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = alias,
            onValueChange = { alias = it },
            label = { Text("用户别名") },
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            value = styleId,
            onValueChange = { styleId = it },
            label = { Text("回复风格 ID") },
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(16.dp))
        Button(
            enabled = !saving,
            onClick = {
                scope.launch {
                    vm.saveChatSettings(alias = alias, responseStyleId = styleId)
                }
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text(if (saving) "保存中…" else "保存") }
        Spacer(Modifier.height(8.dp))
        Text(
            "指令预设等高级设置在电脑端编辑。",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ToolsSettingsTab(
    vm: AppViewModel,
    toolStatus: List<com.whitemoon319.pai.model.ToolLoadStatus>,
) {
    val scope = rememberCoroutineScope()
    val workspace by vm.workspaceStatus.collectAsState()
    val busy by vm.workspaceBusy.collectAsState()
    var showFiles by remember { mutableStateOf(false) }

    if (showFiles) {
        WorkspaceFileManagerScreen(
            vm = vm,
            onBack = { showFiles = false },
        )
        return
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        // Android 沙盒工作区
        Text("Android 沙盒工作区", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(6.dp))
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp)) {
                val stateText = when (workspace?.state) {
                    "ready", "Ready" -> "✓ 就绪"
                    "downloading", "Downloading" -> "⚙ 下载/导入中"
                    "not_downloaded", "NotDownloaded" -> "未初始化"
                    else -> workspace?.state ?: "未知"
                }
                Text(stateText, style = MaterialTheme.typography.titleSmall)
                workspace?.runtimeVersion?.let {
                    Spacer(Modifier.height(2.dp))
                    Text("运行环境：$it", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                workspace?.lastError?.let {
                    Spacer(Modifier.height(4.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
                Spacer(Modifier.height(8.dp))
                // 按钮较多，用 FlowRow 自动换行避免挤爆单行
                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Button(
                        enabled = !busy && workspace?.isReady != true,
                        onClick = { scope.launch { vm.initWorkspace() } },
                    ) { Text("初始化") }
                    Button(
                        enabled = !busy && workspace?.isReady == true,
                        onClick = { scope.launch { vm.repairWorkspace() } },
                    ) { Text("修复运行时") }
                    OutlinedButton(
                        enabled = !busy && workspace?.isReady == true,
                        onClick = { scope.launch { vm.resetWorkspaceRuntime() } },
                    ) { Text("重置运行时") }
                    OutlinedButton(
                        enabled = !busy,
                        onClick = { scope.launch { vm.resetWorkspaceState() } },
                    ) { Text("重置状态") }
                    OutlinedButton(
                        enabled = !busy && workspace?.isReady == true,
                        onClick = {
                            scope.launch { vm.listWorkspaceDir(null) }
                            showFiles = true
                        },
                    ) { Text("文件管理") }
                }
                Spacer(Modifier.height(4.dp))
                Text(
                    "初始化会下载 Ubuntu rootfs 到沙盒；重置运行时保留用户工作区与 Skill 数据。",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Spacer(Modifier.height(16.dp))
        // 工具状态目录
        Text("工具状态", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        if (toolStatus.isEmpty()) {
            Text("暂无工具状态数据。", style = MaterialTheme.typography.bodySmall)
        } else {
            toolStatus.forEach { tool ->
                Card(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                ) {
                    Row(
                        Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        val ok = tool.status == "ready" || tool.status == "ok"
                        Text(
                            if (ok) "✓" else "✗",
                            color = if (ok) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.titleSmall,
                        )
                        Spacer(Modifier.width(8.dp))
                        Column {
                            Text(tool.id ?: "工具", style = MaterialTheme.typography.titleSmall)
                            if (!tool.detail.isNullOrBlank()) {
                                Text(
                                    tool.detail,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AboutSettingsTab(
    bootstrap: com.whitemoon319.pai.model.BootstrapSnapshot?,
    vm: AppViewModel,
) {
    val scope = rememberCoroutineScope()
    val version by vm.appVersion.collectAsState()
    val repoUrl by vm.repoUrl.collectAsState()
    val updateResult by vm.updateResult.collectAsState()
    var updateChecking by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp)) {
        Text("关于", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(12.dp))
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp)) {
                Text("P-AI Android", style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.height(4.dp))
                Text("版本：${version ?: "—"}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                val config = bootstrap?.config
                Text(
                    "当前模型：${config?.assistantDepartmentApiConfigId ?: "—"}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    "供应商数：${config?.apiConfigs?.size ?: 0}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(8.dp))
                Button(
                    enabled = !updateChecking,
                    onClick = {
                        scope.launch {
                            updateChecking = true
                            vm.checkUpdate()
                            updateChecking = false
                        }
                    },
                ) { Text(if (updateChecking) "检查中…" else "检查更新") }
                updateResult?.let {
                    Spacer(Modifier.height(4.dp))
                    Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                }
                repoUrl?.let {
                    Spacer(Modifier.height(4.dp))
                    Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

/** Android 工作区文件浏览器：目录导航、查看/编辑、新建、重命名/移动、删除、导入、导出、搜索。 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun WorkspaceFileManagerScreen(
    vm: AppViewModel,
    onBack: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val context = androidx.compose.ui.platform.LocalContext.current
    val files by vm.workspaceFiles.collectAsState()
    val dir by vm.workspaceDir.collectAsState()
    var editing by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }
    var newFileName by remember { mutableStateOf<String?>(null) }
    var newDirName by remember { mutableStateOf<String?>(null) }
    var renameTarget by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }
    var moveTarget by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }
    var pendingDelete by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }
    var exporting by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }
    var searchMode by remember { mutableStateOf(false) }
    var searchQuery by remember { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<com.whitemoon319.pai.model.WorkspaceSearchMatch>>(emptyList()) }
    var searching by remember { mutableStateOf(false) }
    var menuFor by remember { mutableStateOf<com.whitemoon319.pai.model.WorkspaceFileEntry?>(null) }

    // 导入：SAF 文件选择
    val filePicker = androidx.activity.compose.rememberLauncherForActivityResult(
        androidx.activity.result.contract.ActivityResultContracts.OpenDocument()
    ) { uri ->
        if (uri != null) {
            scope.launch {
                try {
                    val name = androidx.documentfile.provider.DocumentFile.fromSingleUri(context, uri)?.name
                        ?: "import.bin"
                    val bytes = context.contentResolver.openInputStream(uri)?.readBytes()
                    if (bytes != null) {
                        val b64 = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
                        val target = dir?.takeIf { it.isNotBlank() }?.let { "$it/" } ?: ""
                        val ok = vm.importWorkspaceFile(name, b64, target)
                        if (ok) vm.listWorkspaceDir(dir)
                    }
                } catch (e: Exception) {
                    vm.error.value = "导入失败: ${e.message}"
                }
            }
        }
    }

    fun refresh() {
        scope.launch { vm.listWorkspaceDir(dir) }
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text(if (searchMode) "搜索文件" else "工作区文件") },
            navigationIcon = {
                IconButton(onClick = { if (searchMode) { searchMode = false } else onBack() }) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                }
            },
            actions = {
                if (searchMode) {
                    IconButton(onClick = {
                        scope.launch {
                            searching = true
                            val base = dir?.takeIf { it.isNotBlank() }
                            searchResults = vm.serviceGrep(searchQuery, base)
                            searching = false
                        }
                    }) {
                        Icon(Icons.Default.Search, contentDescription = "搜索")
                    }
                } else {
                    IconButton(onClick = { searchMode = true }) {
                        Icon(Icons.Default.Search, contentDescription = "搜索")
                    }
                    IconButton(onClick = { refresh() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "刷新")
                    }
                    IconButton(onClick = { filePicker.launch(arrayOf("*/*")) }) {
                        Icon(Icons.Default.Add, contentDescription = "导入文件")
                    }
                    IconButton(onClick = { newFileName = "" }) {
                        Icon(Icons.Default.Create, contentDescription = "新建文件")
                    }
                }
            },
        )
        if (searchMode) {
            // 搜索栏（grep）
            Column(Modifier.fillMaxWidth().padding(8.dp)) {
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = { searchQuery = it },
                    label = { Text("搜索内容") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(Modifier.height(6.dp))
                Button(
                    enabled = searchQuery.isNotBlank() && !searching,
                    onClick = {
                        scope.launch {
                            searching = true
                            val base = dir?.takeIf { it.isNotBlank() }
                            searchResults = vm.serviceGrep(searchQuery, base)
                            searching = false
                        }
                    },
                ) { Text(if (searching) "搜索中…" else "搜索") }
                Spacer(Modifier.height(6.dp))
                if (searchResults.isEmpty() && !searching) {
                    Text("无结果", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                LazyColumn(Modifier.fillMaxSize()) {
                    items(searchResults, key = { "${it.path}:${it.line}" }) { m ->
                        Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp)) {
                            Text("${m.path ?: "—"}:${m.line}", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                            m.text?.let {
                                Text(it, style = MaterialTheme.typography.bodySmall, maxLines = 3)
                            }
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))
                    }
                }
            }
            return@Column
        }

        // 路径导航
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                if (dir.isNullOrEmpty()) "/（根）" else "/${dir}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
            if (!dir.isNullOrEmpty()) {
                TextButton(onClick = { scope.launch { vm.listWorkspaceDir(null) } }) { Text("根目录") }
            }
        }
        HorizontalDivider()
        LazyColumn(Modifier.fillMaxSize()) {
            files?.parentPath?.let { parent ->
                item(key = "parent") {
                    TextButton(onClick = { scope.launch { vm.listWorkspaceDir(parent) } }) {
                        Text("↑ 上一级")
                    }
                }
            }
            files?.entries.orEmpty().forEach { entry ->
                item(key = entry.path ?: entry.name ?: "") {
                    Row(
                        Modifier.fillMaxWidth().clickable {
                            if (entry.isDirectory) {
                                scope.launch { vm.listWorkspaceDir(entry.path) }
                            } else {
                                editing = entry
                            }
                        }.padding(horizontal = 12.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            if (entry.isDirectory) "📁" else "📄",
                            style = MaterialTheme.typography.titleSmall,
                        )
                        Spacer(Modifier.width(8.dp))
                        Column(Modifier.weight(1f)) {
                            Text(entry.name ?: "—", style = MaterialTheme.typography.bodyMedium)
                            if (!entry.isDirectory && entry.bytes != null) {
                                Text(
                                    "${entry.bytes} bytes",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        // 更多操作菜单
                        Box {
                            IconButton(onClick = { menuFor = entry }) {
                                Icon(Icons.Default.MoreVert, contentDescription = "更多")
                            }
                            DropdownMenu(
                                expanded = menuFor?.path == entry.path,
                                onDismissRequest = { menuFor = null },
                            ) {
                                if (!entry.isDirectory) {
                                    DropdownMenuItem(
                                        text = { Text("查看/编辑") },
                                        onClick = { menuFor = null; editing = entry },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("导出") },
                                        onClick = { menuFor = null; exporting = entry },
                                    )
                                }
                                DropdownMenuItem(
                                    text = { Text("重命名") },
                                    onClick = { menuFor = null; renameTarget = entry },
                                )
                                if (!entry.isDirectory) {
                                    DropdownMenuItem(
                                        text = { Text("移动") },
                                        onClick = { menuFor = null; moveTarget = entry },
                                    )
                                }
                                DropdownMenuItem(
                                    text = { Text("删除") },
                                    onClick = { menuFor = null; pendingDelete = entry },
                                )
                            }
                        }
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))
                }
            }
        }
    }

    // 查看/编辑文本文件
    editing?.let { entry ->
        WorkspaceTextEditorDialog(
            entry = entry,
            vm = vm,
            onDismiss = { editing = null },
        )
    }

    // 新建文件
    newFileName?.let { initial ->
        var name by remember { mutableStateOf(initial) }
        AlertDialog(
            onDismissRequest = { newFileName = null },
            title = { Text("新建文件") },
            text = {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("文件名") },
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    val base = dir?.takeIf { it.isNotBlank() }?.let { "$it/" } ?: ""
                    val path = "$base${name.trim()}"
                    newFileName = null
                    scope.launch {
                        val ok = vm.writeWorkspaceFile(path, "", overwrite = false)
                        if (ok) vm.listWorkspaceDir(dir)
                    }
                }) { Text("创建") }
            },
            dismissButton = {
                TextButton(onClick = { newFileName = null }) { Text("取消") }
            },
        )
    }

    // 新建目录
    newDirName?.let { initial ->
        var name by remember { mutableStateOf(initial) }
        AlertDialog(
            onDismissRequest = { newDirName = null },
            title = { Text("新建目录") },
            text = {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("目录名") },
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    val base = dir?.takeIf { it.isNotBlank() }?.let { "$it/" } ?: ""
                    val path = "$base${name.trim()}/.keep"
                    newDirName = null
                    scope.launch {
                        val ok = vm.writeWorkspaceFile(path, "", overwrite = false)
                        if (ok) vm.listWorkspaceDir(dir)
                    }
                }) { Text("创建") }
            },
            dismissButton = {
                TextButton(onClick = { newDirName = null }) { Text("取消") }
            },
        )
    }

    // 重命名
    renameTarget?.let { entry ->
        var name by remember { mutableStateOf(entry.name ?: "") }
        AlertDialog(
            onDismissRequest = { renameTarget = null },
            title = { Text("重命名") },
            text = {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("新名称") },
                    modifier = Modifier.fillMaxWidth(),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    val base = entry.path?.let { p ->
                        p.substringBeforeLast("/", "")
                    } ?: ""
                    val newPath = if (base.isEmpty()) name.trim() else "$base/${name.trim()}"
                    renameTarget = null
                    scope.launch {
                        val ok = vm.moveWorkspaceFile(entry.path ?: "", newPath, overwrite = false)
                        if (ok) vm.listWorkspaceDir(dir)
                    }
                }) { Text("确定") }
            },
            dismissButton = {
                TextButton(onClick = { renameTarget = null }) { Text("取消") }
            },
        )
    }

    // 移动（输入目标路径）
    moveTarget?.let { entry ->
        var targetPath by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { moveTarget = null },
            title = { Text("移动文件") },
            text = {
                Column {
                    Text("移动「${entry.name}」到（相对路径，如 sub/dir/name.txt）：", style = MaterialTheme.typography.bodySmall)
                    Spacer(Modifier.height(6.dp))
                    OutlinedTextField(
                        value = targetPath,
                        onValueChange = { targetPath = it },
                        label = { Text("目标路径") },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    moveTarget = null
                    scope.launch {
                        val ok = vm.moveWorkspaceFile(entry.path ?: "", targetPath.trim(), overwrite = false)
                        if (ok) vm.listWorkspaceDir(dir)
                    }
                }) { Text("移动") }
            },
            dismissButton = {
                TextButton(onClick = { moveTarget = null }) { Text("取消") }
            },
        )
    }

    // 删除确认
    pendingDelete?.let { entry ->
        AlertDialog(
            onDismissRequest = { pendingDelete = null },
            title = { Text("删除文件") },
            text = { Text("确定删除「${entry.name}」吗？") },
            confirmButton = {
                TextButton(onClick = {
                    val path = entry.path
                    pendingDelete = null
                    if (path != null) {
                        scope.launch {
                            vm.deleteWorkspaceFile(path)
                            vm.listWorkspaceDir(dir)
                        }
                    }
                }) { Text("删除") }
            },
            dismissButton = {
                TextButton(onClick = { pendingDelete = null }) { Text("取消") }
            },
        )
    }

    // 导出：base64 → 分享/保存
    exporting?.let { entry ->
        var exportingState by remember { mutableStateOf(false) }
        AlertDialog(
            onDismissRequest = { exporting = null },
            title = { Text("导出") },
            text = {
                if (exportingState) {
                    Text("导出中…", style = MaterialTheme.typography.bodySmall)
                } else {
                    Text("导出「${entry.name}」？", style = MaterialTheme.typography.bodySmall)
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !exportingState,
                    onClick = {
                        scope.launch {
                            exportingState = true
                            val result = vm.exportWorkspaceFile(entry.path ?: "")
                            exportingState = false
                            if (result?.dataBase64 != null) {
                                try {
                                    val bytes = android.util.Base64.decode(result.dataBase64, android.util.Base64.NO_WRAP)
                                    val name = result.fileName ?: entry.name ?: "export.bin"
                                    // 写入 app 导出目录
                                    val outDir = java.io.File(context.filesDir, "exports")
                                    outDir.mkdirs()
                                    val out = java.io.File(outDir, name)
                                    out.writeBytes(bytes)
                                    android.widget.Toast.makeText(context, "已导出到 ${out.absolutePath}", android.widget.Toast.LENGTH_LONG).show()
                                } catch (e: Exception) {
                                    vm.error.value = "导出失败: ${e.message}"
                                }
                            }
                            exporting = null
                        }
                    },
                ) { Text(if (exportingState) "导出中…" else "导出") }
            },
            dismissButton = {
                TextButton(onClick = { exporting = null }) { Text("取消") }
            },
        )
    }
}

/** 文本文件查看/编辑对话框。 */
@Composable
private fun WorkspaceTextEditorDialog(
    entry: com.whitemoon319.pai.model.WorkspaceFileEntry,
    vm: AppViewModel,
    onDismiss: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var content by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }

    LaunchedEffect(entry.path) {
        content = vm.readWorkspaceFile(entry.path ?: return@LaunchedEffect)?.text
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(entry.name ?: "文件") },
        text = {
            when (val c = content) {
                null -> Box(Modifier.fillMaxWidth().height(120.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
                else -> OutlinedTextField(
                    value = c,
                    onValueChange = { content = it },
                    modifier = Modifier.fillMaxWidth().height(300.dp),
                    textStyle = MaterialTheme.typography.bodySmall,
                )
            }
        },
        confirmButton = {
            TextButton(
                enabled = content != null && !saving,
                onClick = {
                    val c = content ?: return@TextButton
                    val path = entry.path ?: return@TextButton
                    scope.launch {
                        saving = true
                        val ok = vm.writeWorkspaceFile(path, c, overwrite = true)
                        saving = false
                        if (ok) onDismiss()
                    }
                },
            ) { Text(if (saving) "保存中…" else "保存") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消") }
        },
    )
}