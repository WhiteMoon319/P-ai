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
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaiApp(vm: AppViewModel) {
    DisposableEffect(Unit) {
        vm.start()
        onDispose { vm.stop() }
    }

    var inChat by remember { mutableStateOf(false) }
    var title by remember { mutableStateOf("会话") }

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
                title = conv.title ?: conv.conversationId
                inChat = true
            },
            onNew = {
                title = "新会话"
                inChat = true
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ConnectionBanner(vm: AppViewModel) {
    val connection by vm.connectionState.collectAsState()
    val statusText = when (connection) {
        ConnectionStatus.Connecting -> "连接中…"
        ConnectionStatus.Connected -> "已连接"
        ConnectionStatus.Disconnected -> "未连接"
    }
    Surface(color = when (connection) {
        ConnectionStatus.Connected -> MaterialTheme.colorScheme.primaryContainer
        else -> MaterialTheme.colorScheme.surfaceVariant
    }) {
        Text(
            text = statusText,
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
                if (streaming.isNotEmpty()) {
                    item(key = "streaming") { Text(streaming, Modifier.padding(12.dp)) }
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
            Text(text, Modifier.padding(10.dp))
        }
    }
}