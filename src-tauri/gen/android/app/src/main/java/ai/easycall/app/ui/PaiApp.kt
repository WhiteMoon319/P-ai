package ai.easycall.app.ui

import ai.easycall.app.model.ChatMessage
import ai.easycall.app.model.ConversationSummary
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
    val reasoning by vm.reasoningText.collectAsState()
    val toolEvents by vm.toolEvents.collectAsState()
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
                if (reasoning.isNotEmpty()) {
                    item(key = "reasoning") {
                        ReasoningBlock(reasoning, isStreaming)
                    }
                }
                toolEvents.forEachIndexed { index, tool ->
                    item(key = "tool_$index") {
                        ToolCallPill(tool, isStreaming)
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

@Composable
private fun ReasoningBlock(text: String, streaming: Boolean) {
    // 默认展开：流式思考实时滚动可见；结束后用户可点击折叠
    var expanded by remember { mutableStateOf(streaming) }
    LaunchedEffect(streaming) {
        if (streaming) expanded = true
    }
    val color = MaterialTheme.colorScheme.outline
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
        shape = MaterialTheme.shapes.medium,
        modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp).widthIn(max = 320.dp),
    ) {
        Column(Modifier.padding(8.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth().clickable { expanded = !expanded },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    if (streaming) "思考中…" else if (expanded) "思考（点击折叠）" else "已思考（点击展开）",
                    style = MaterialTheme.typography.labelMedium,
                    color = color,
                )
            }
            if (expanded) {
                Text(
                    text,
                    Modifier.padding(top = 6.dp),
                    style = MaterialTheme.typography.bodySmall,
                    color = color,
                )
            }
        }
    }
}

@Composable
private fun ToolCallPill(text: String, streaming: Boolean) {
    // 工具调用默认折叠为标题条；展开显示工具名/参数
    var expanded by remember { mutableStateOf(false) }
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
        shape = MaterialTheme.shapes.small,
        modifier = Modifier.padding(horizontal = 12.dp, vertical = 3.dp).widthIn(max = 320.dp),
    ) {
        Column(Modifier.padding(horizontal = 10.dp, vertical = 6.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth().clickable { expanded = !expanded },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    if (streaming) "🛠 调用中…" else if (expanded) "工具（点击收起）" else "工具（点击展开）",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSecondaryContainer,
                )
            }
            if (expanded) {
                Text(
                    text,
                    Modifier.padding(top = 6.dp),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSecondaryContainer,
                )
            }
        }
    }
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
                    val reasoning = message.parts
                        .mapNotNull { it.reasoningContent?.takeIf { r -> r.isNotBlank() } }
                        .joinToString("\n")
                    if (reasoning.isNotBlank()) {
                        ReasoningBlock(reasoning, streaming = false)
                    }
                    if (text.isNotBlank()) {
                        MarkdownText(content = text)
                    }
                }
            }
        }
    }
}