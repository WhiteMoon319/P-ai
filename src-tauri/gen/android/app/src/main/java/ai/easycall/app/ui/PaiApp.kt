package ai.easycall.app.ui

import ai.easycall.app.model.ActivityStep
import ai.easycall.app.model.ChatMessage
import ai.easycall.app.model.ConversationSummary
import ai.easycall.app.model.buildActivityStepsFromMessage
import ai.easycall.app.viewmodel.AppViewModel
import ai.easycall.app.ws.ConnectionStatus
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Send
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
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
import ai.easycall.app.ui.richtext.MarkdownText
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
        val scope = rememberCoroutineScope()

        if (inChat) {
            ChatScreen(
                vm = vm,
                title = title,
                onBack = {
                    inChat = false
                    title = "会话"
                },
            )
        } else {
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
            )
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
) {
    ConversationListScreenImpl(vm = vm, onOpen = onOpen, onNew = onNew, onCreated = onCreated)
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
) {
    val scope = rememberCoroutineScope()
    val conversations by vm.conversations.collectAsState()
    val loading by vm.loading.collectAsState()
    var showNewDialog by remember { mutableStateOf(false) }
    var fullOptions by remember { mutableStateOf<ai.easycall.app.model.CreateConversationOptions>(ai.easycall.app.model.CreateConversationOptions()) }
    var optionsLoading by remember { mutableStateOf(false) }
    var selectedOption by remember { mutableStateOf<ai.easycall.app.model.CreateConversationOptionItem?>(null) }

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
) {
    val scope = rememberCoroutineScope()
    val messages by vm.messages.collectAsState()
    val streaming by vm.streamingText.collectAsState()
    val activitySteps by vm.activitySteps.collectAsState()
    val isStreaming by vm.isStreaming.collectAsState()
    val loading by vm.loading.collectAsState()
    var input by remember { mutableStateOf("") }
    val listState = rememberLazyListState()

    Column(Modifier.fillMaxSize()) {
        ConnectionBanner(vm)
        TopAppBar(
            title = { Text(title) },
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
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
                        ThinkingBlock(steps = activitySteps, streaming = isStreaming)
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
private fun ThinkingBlock(steps: List<ActivityStep>, streaming: Boolean) {
    // 大类默认展开：流式中思考实时可见；结束后用户可整体折叠。
    val hasLoadingReasoning = streaming && steps.any { it is ActivityStep.Reasoning }
    var groupExpanded by remember { mutableStateOf(true) }
    LaunchedEffect(streaming, steps) {
        if (hasLoadingReasoning) groupExpanded = true
    }
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
            val title = when {
                streaming && hasLoadingReasoning -> "思考与工具 · 进行中"
                else -> "思考与工具 · ${reasoningCount}思考 ${toolCount}工具"
            }
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
            val isOpen = expanded ?: (step.text.length < 400)
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
            val isOpen = expanded ?: step.name.isNullOrBlank() // 无工具名时默认展开看上下文
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
    ThinkingBlock(steps = steps, streaming = false)
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