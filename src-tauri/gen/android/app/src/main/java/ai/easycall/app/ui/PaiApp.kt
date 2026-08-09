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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
) {
    val scope = rememberCoroutineScope()
    val conversations by vm.conversations.collectAsState()
    val loading by vm.loading.collectAsState()

    LaunchedEffect(Unit) {
        vm.refreshConversations()
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
                IconButton(onClick = onNew) {
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