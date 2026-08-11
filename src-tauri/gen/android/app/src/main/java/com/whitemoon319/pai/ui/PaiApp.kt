package com.whitemoon319.pai.ui

import com.whitemoon319.pai.model.ActivityStep
import com.whitemoon319.pai.model.ChatMessage
import com.whitemoon319.pai.model.ConversationSummary
import com.whitemoon319.pai.model.buildActivityStepsFromMessage
import com.whitemoon319.pai.viewmodel.AppViewModel
import com.whitemoon319.pai.ws.ConnectionStatus
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
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
import androidx.compose.foundation.layout.size
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
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.filled.Mic
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
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.whitemoon319.pai.ui.richtext.MarkdownText
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaiApp(vm: AppViewModel) {
    DisposableEffect(Unit) {
        vm.start()
        onDispose { vm.stop() }
    }
    // 预载人设列表：消息气泡显示 agent 名
    LaunchedEffect(Unit) {
        vm.loadAgents()
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
                        ConversationRow(
                            conv = conv,
                            onClick = { onOpen(conv) },
                            onRename = { title ->
                                scope.launch { vm.renameConversation(conv.conversationId, title) }
                            },
                            onTogglePin = { pinned ->
                                scope.launch { vm.toggleConversationPin(conv.conversationId, pinned) }
                            },
                            onArchive = {
                                scope.launch { vm.archiveConversation(conv.conversationId) }
                            },
                            onDelete = {
                                scope.launch { vm.deleteConversation(conv.conversationId) }
                            },
                        )
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

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun ConversationRow(
    conv: ConversationSummary,
    onClick: () -> Unit,
    onRename: (String) -> Unit,
    onTogglePin: (Boolean) -> Unit,
    onArchive: () -> Unit,
    onDelete: () -> Unit,
) {
    var menuExpanded by remember { mutableStateOf(false) }
    var showRename by remember { mutableStateOf(false) }
    var showDelete by remember { mutableStateOf(false) }
    var renameText by remember { mutableStateOf(conv.title ?: "") }

    Column(
        Modifier
            .fillMaxWidth()
            .combinedClickable(
                onClick = onClick,
                onLongClick = { menuExpanded = true },
            )
            .padding(12.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                conv.title ?: "无标题",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = { menuExpanded = true }) {
                Icon(Icons.Default.MoreVert, contentDescription = "会话操作")
            }
        }
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

    DropdownMenu(expanded = menuExpanded, onDismissRequest = { menuExpanded = false }) {
        DropdownMenuItem(
            text = { Text("重命名") },
            onClick = {
                menuExpanded = false
                renameText = conv.title ?: ""
                showRename = true
            },
        )
        DropdownMenuItem(
            text = { Text(if (conv.isPinned == true) "取消固定" else "固定") },
            onClick = {
                menuExpanded = false
                onTogglePin(conv.isPinned != true)
            },
        )
        DropdownMenuItem(
            text = { Text("归档") },
            onClick = {
                menuExpanded = false
                onArchive()
            },
        )
        DropdownMenuItem(
            text = { Text("删除") },
            onClick = {
                menuExpanded = false
                showDelete = true
            },
        )
    }

    if (showRename) {
        AlertDialog(
            onDismissRequest = { showRename = false },
            title = { Text("重命名会话") },
            text = {
                OutlinedTextField(
                    value = renameText,
                    onValueChange = { renameText = it },
                    singleLine = true,
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    showRename = false
                    if (renameText.isNotBlank()) onRename(renameText)
                }) { Text("保存") }
            },
            dismissButton = { TextButton(onClick = { showRename = false }) { Text("取消") } },
        )
    }

    if (showDelete) {
        AlertDialog(
            onDismissRequest = { showDelete = false },
            title = { Text("删除会话") },
            text = { Text("确定删除「${conv.title ?: "无标题"}」吗？此操作不可恢复。") },
            confirmButton = {
                TextButton(onClick = {
                    showDelete = false
                    onDelete()
                }) { Text("删除") }
            },
            dismissButton = { TextButton(onClick = { showDelete = false }) { Text("取消") } },
        )
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
                    val agentName = vm.agents.collectAsState().value
                        ?.firstOrNull { it.id == msg.speakerAgentId }
                        ?.name
                    MessageBubble(msg, agentName = agentName)
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
            // 已选附件条：显示待发送附件，可移除
            val pendingAttachment by vm.pendingAttachment.collectAsState()
            if (pendingAttachment != null) {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 8.dp, vertical = 2.dp),
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    shape = MaterialTheme.shapes.small,
                ) {
                    Row(
                        Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.Default.AttachFile,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = MaterialTheme.colorScheme.onSecondaryContainer,
                        )
                        Spacer(Modifier.width(6.dp))
                        Text(
                            text = pendingAttachment!!.fileName,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSecondaryContainer,
                            modifier = Modifier.weight(1f),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        IconButton(
                            onClick = { vm.clearPendingAttachment() },
                            modifier = Modifier.size(24.dp),
                        ) {
                            Icon(
                                Icons.Default.Clear,
                                contentDescription = "移除附件",
                                modifier = Modifier.size(16.dp),
                                tint = MaterialTheme.colorScheme.onSecondaryContainer,
                            )
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
                // 模型切换（对齐 Vue：输入区左侧下拉）
                val apiConfigs = vm.appConfig.collectAsState().value?.apiConfigs ?: emptyList()
                var modelMenuExpanded by remember { mutableStateOf(false) }
                Box {
                    TextButton(onClick = { modelMenuExpanded = true }) {
                        Text("模型", style = MaterialTheme.typography.labelLarge)
                    }
                    DropdownMenu(expanded = modelMenuExpanded, onDismissRequest = { modelMenuExpanded = false }) {
                        if (apiConfigs.isEmpty()) {
                            DropdownMenuItem(
                                text = { Text("暂无供应商，请先到设置添加") },
                                onClick = { modelMenuExpanded = false },
                            )
                        } else {
                            apiConfigs.forEach { api ->
                                DropdownMenuItem(
                                    text = { Text("${api.name ?: api.id ?: "未命名"} · ${api.model ?: "—"}") },
                                    onClick = {
                                        modelMenuExpanded = false
                                        val convId = vm.currentConversationId.value
                                        if (convId != null) {
                                            scope.launch {
                                                vm.setConversationPreferredModel(convId, api.id)
                                            }
                                        }
                                    },
                                )
                            }
                        }
                    }
                }
                OutlinedTextField(
                    value = input,
                    onValueChange = { input = it },
                    modifier = Modifier.weight(1f),
                    placeholder = { Text("输入消息…") },
                    maxLines = 4,
                )
                Spacer(Modifier.width(4.dp))
                // 附件：文件选择 → 复制到沙盒 → 摄取 → 随消息发送
                val ctx = LocalContext.current
                val attaching by vm.attaching.collectAsState()
                val pendingAttachment by vm.pendingAttachment.collectAsState()
                val attachLauncher = androidx.activity.compose.rememberLauncherForActivityResult(
                    androidx.activity.result.contract.ActivityResultContracts.OpenDocument()
                ) { uri ->
                    if (uri != null) {
                        scope.launch {
                            try {
                                val sandboxDir = java.io.File(ctx.filesDir, "attachments").apply { mkdirs() }
                                val displayName = ctx.contentResolver.query(
                                    uri, null, null, null, null
                                )?.use { c ->
                                    val idx = c.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
                                    if (idx >= 0) c.getString(idx) else null
                                } ?: "attachment"
                                val mime = ctx.contentResolver.getType(uri) ?: ""
                                val dest = java.io.File(sandboxDir, displayName)
                                withContext(Dispatchers.IO) {
                                    ctx.contentResolver.openInputStream(uri)?.use { input ->
                                        dest.outputStream().use { output -> input.copyTo(output) }
                                    }
                                }
                                vm.attachLocalFile(dest.absolutePath, displayName, mime)
                            } catch (e: Exception) {
                                vm.error.value = "读取附件失败: ${e.message}"
                            }
                        }
                    }
                }
                IconButton(
                    onClick = { attachLauncher.launch(arrayOf("*/*")) },
                    enabled = !attaching && !isStreaming,
                ) {
                    Icon(
                        if (attaching) Icons.Default.Refresh else Icons.Default.AttachFile,
                        contentDescription = if (attaching) "添加附件中" else "添加附件",
                        tint = if (pendingAttachment != null) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.onSurface,
                    )
                }
                Spacer(Modifier.width(4.dp))
                // 语音输入：麦克风按钮（按住录音/点击切换），识别结果回填输入框
                val isRecording by vm.isRecording.collectAsState()
                val recognized by vm.recognizedText.collectAsState()
                LaunchedEffect(recognized) {
                    if (!recognized.isNullOrBlank()) {
                        input = input + recognized!!
                        vm.recognizedText.value = null
                    }
                }
                val audioPermissionLauncher = androidx.activity.compose.rememberLauncherForActivityResult(
                    androidx.activity.result.contract.ActivityResultContracts.RequestPermission()
                ) { granted ->
                    if (granted) vm.startRecording()
                    else vm.error.value = "需要录音权限才能使用语音输入"
                }
                IconButton(onClick = {
                    if (isRecording) {
                        vm.stopAndTranscribe()
                    } else {
                        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M &&
                            ctx.checkSelfPermission(android.Manifest.permission.RECORD_AUDIO)
                            != android.content.pm.PackageManager.PERMISSION_GRANTED
                        ) {
                            audioPermissionLauncher.launch(android.Manifest.permission.RECORD_AUDIO)
                        } else {
                            vm.startRecording()
                        }
                    }
                }) {
                    Icon(
                        if (isRecording) Icons.Default.Clear else Icons.Default.Mic,
                        contentDescription = if (isRecording) "停止录音" else "语音输入",
                        tint = if (isRecording) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface,
                    )
                }
                Spacer(Modifier.width(4.dp))
                if (isStreaming) {
                    IconButton(onClick = { scope.launch { vm.stopStreaming() } }) {
                        Icon(Icons.Default.Clear, contentDescription = "停止")
                    }
                } else {
                    IconButton(onClick = {
                        val text = input
                        val hasAttachment = pendingAttachment != null
                        if (text.isNotBlank() || hasAttachment) {
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
fun MessageBubble(message: ChatMessage, agentName: String? = null) {
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
                Column(Modifier.padding(10.dp)) {
                    Text(
                        "我",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(2.dp))
                    Text(text)
                }
            } else {
                Column(Modifier.padding(10.dp)) {
                    if (!agentName.isNullOrBlank()) {
                        Text(
                            agentName,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(2.dp))
                    }
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

/** 设置一级页条目（顺序对齐 Vue ConfigView 导航）。 */
private enum class SettingsEntry(
    val title: String,
    val subtitle: String,
    val group: String? = null,
) {
    Welcome("欢迎", "快速上手与常用入口", group = "通用"),
    Chat("聊天设置", "用户别名、回复风格、指令预设", group = "通用"),
    Notification("通知", "消息通知、声音、桌面操作提醒", group = "通用"),
    Network("网络访问", "远程连接开关、端口、访问密码", group = "通用"),
    Appearance("外观", "界面语言、字号", group = "通用"),
    Api("模型与供应商", "供应商增删改、连接测试、语音识别", group = "模型"),
    Tools("工具", "Android 沙盒工作区、工具状态", group = "模型"),
    Mcp("MCP", "MCP 服务器与技能", group = "模型"),
    Persona("人设", "人设与代理管理", group = "组织"),
    Department("部门", "部门结构", group = "组织"),
    DepartmentTree("部门树", "部门层级关系", group = "组织"),
    Memory("记忆", "记忆管理", group = "数据"),
    Task("任务", "定时任务与运行日志", group = "数据"),
    Logs("日志", "运行日志", group = "数据"),
    Storage("存储", "存储用量与清理", group = "数据"),
    RemoteIm("远程IM", "通道、联系人、转发设置", group = "连接"),
    Usage("用量", "Token 用量统计", group = "数据"),
    About("关于", "版本、检查更新、仓库", group = "通用"),
}

/** 设置页分组顺序。 */
private val settingsGroups = listOf("通用", "模型", "组织", "数据", "连接")

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
                // 一级：设置项列表（按组）
                Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                    settingsGroups.forEach { group ->
                        Text(
                            group,
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.padding(start = 16.dp, top = 12.dp, bottom = 4.dp),
                        )
                        SettingsEntry.entries.filter { it.group == group }.forEach { item ->
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
            }
            SettingsEntry.Welcome -> Text(
                "欢迎使用 P-AI\n\n常用入口：\n· 新建会话：会话列表右下角 + 按钮\n· 语音输入：聊天输入框麦克风\n· 添加附件：聊天输入框回形针\n· 远程连接：设置 → 网络访问",
                modifier = Modifier.padding(20.dp),
            )
            SettingsEntry.Chat -> ChatSettingsTab(settings = chatSettings, vm = vm)
            SettingsEntry.Notification -> NotificationSettingsTab(vm = vm)
            SettingsEntry.Network -> NetworkSettingsTab(vm = vm)
            SettingsEntry.Appearance -> AppearanceSettingsTab(vm = vm)
            SettingsEntry.Api -> ApiSettingsTab(appConfig = appConfig, vm = vm)
            SettingsEntry.Tools -> ToolsSettingsTab(vm = vm, toolStatus = toolStatus)
            SettingsEntry.Mcp -> McpSettingsTab(vm = vm)
            SettingsEntry.Persona -> PersonaSettingsTab(vm = vm)
            SettingsEntry.Department -> DepartmentSettingsTab(vm = vm)
            SettingsEntry.DepartmentTree -> DepartmentTreeSettingsTab(vm = vm)
            SettingsEntry.Memory -> MemorySettingsTab(vm = vm)
            SettingsEntry.Task -> TaskSettingsTab(vm = vm)
            SettingsEntry.Logs -> LogsSettingsTab(vm = vm)
            SettingsEntry.Storage -> StorageSettingsTab(vm = vm)
            SettingsEntry.RemoteIm -> RemoteImSettingsTab(vm = vm)
            SettingsEntry.Usage -> UsageSettingsTab(vm = vm)
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
        // 语音识别（STT）供应商选择
        Spacer(Modifier.height(16.dp))
        HorizontalDivider()
        Spacer(Modifier.height(12.dp))
        Text("语音识别供应商", style = MaterialTheme.typography.titleSmall)
        Text(
            "语音输入使用的识别服务；需先新增协议为 openai_stt 或 mimo_asr 的供应商。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(8.dp))
        val sttId = appConfig?.sttApiConfigId
        val sttCandidates = appConfig?.apiConfigs.orEmpty().filter { api ->
            val fmt = api.requestFormat.orEmpty()
            fmt == "openai_stt" || fmt == "mimo_asr" || fmt == "stt"
        }
        if (sttCandidates.isEmpty()) {
            Text(
                "暂无语音识别供应商，请在「新增」中把协议设为 openai_stt 或 mimo_asr。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        } else {
            sttCandidates.forEach { api ->
                val isSttCurrent = sttId == api.id
                Card(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                ) {
                    Row(
                        Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(api.name ?: "未命名", style = MaterialTheme.typography.bodyMedium)
                            Text(
                                "协议：${api.requestFormat} · 模型：${api.model ?: "—"}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            if (isSttCurrent) {
                                Text(
                                    "语音识别使用中",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                        }
                        if (!isSttCurrent) {
                            TextButton(
                                enabled = !saving,
                                onClick = {
                                    scope.launch { vm.saveSttApiConfig(api.id) }
                                },
                            ) { Text("设为语音识别") }
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
        "openai_stt", "mimo_asr",
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
    var pdfMode by remember { mutableStateOf(settings?.pdfReadMode ?: "text") }
    var presets by remember { mutableStateOf(settings?.instructionPresets ?: emptyList()) }
    var pdfMenuExpanded by remember { mutableStateOf(false) }
    // settings 异步到达后回填（首次进入时可能为 null）
    LaunchedEffect(settings) {
        if (settings != null) {
            alias = settings.userAlias ?: ""
            styleId = settings.responseStyleId ?: ""
            pdfMode = settings.pdfReadMode ?: "text"
            presets = settings.instructionPresets ?: emptyList()
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
        Spacer(Modifier.height(12.dp))
        // PDF 阅读模式
        Text("PDF 阅读模式", style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(4.dp))
        Box {
            OutlinedButton(
                onClick = { pdfMenuExpanded = true },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(if (pdfMode == "image") "图片模式（逐页截图）" else "文本模式（提取文字）")
            }
            DropdownMenu(expanded = pdfMenuExpanded, onDismissRequest = { pdfMenuExpanded = false }) {
                DropdownMenuItem(
                    text = { Text("文本模式（提取文字）") },
                    onClick = { pdfMode = "text"; pdfMenuExpanded = false },
                )
                DropdownMenuItem(
                    text = { Text("图片模式（逐页截图）") },
                    onClick = { pdfMode = "image"; pdfMenuExpanded = false },
                )
            }
        }
        Spacer(Modifier.height(12.dp))
        // 指令预设
        Text("指令预设", style = MaterialTheme.typography.titleSmall)
        Spacer(Modifier.height(4.dp))
        presets.forEachIndexed { index, preset ->
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                OutlinedTextField(
                    value = preset.name ?: "",
                    onValueChange = { name ->
                        presets = presets.toMutableList().also {
                            it[index] = it[index].copy(name = name)
                        }
                    },
                    label = { Text("名称") },
                    modifier = Modifier.weight(1f),
                )
                Spacer(Modifier.width(6.dp))
                OutlinedTextField(
                    value = preset.prompt ?: "",
                    onValueChange = { prompt ->
                        presets = presets.toMutableList().also {
                            it[index] = it[index].copy(prompt = prompt)
                        }
                    },
                    label = { Text("指令内容") },
                    modifier = Modifier.weight(2f),
                )
                TextButton(onClick = {
                    presets = presets.toMutableList().also { it.removeAt(index) }
                }) { Text("删除") }
            }
        }
        TextButton(onClick = {
            presets = presets + com.whitemoon319.pai.model.PromptCommandPreset(name = "", prompt = "")
        }) { Text("+ 添加指令预设") }
        Spacer(Modifier.height(16.dp))
        Button(
            enabled = !saving,
            onClick = {
                scope.launch {
                    vm.saveChatSettings(
                        alias = alias,
                        responseStyleId = styleId,
                        pdfReadMode = pdfMode,
                        instructionPresets = presets.ifEmpty { null },
                    )
                }
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text(if (saving) "保存中…" else "保存") }
    }
}

@OptIn(ExperimentalLayoutApi::class)
/** 网络访问（远程连接）设置：开关 + 端口 + 密码 + 状态展示。 */
@Composable
private fun NetworkSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val appConfig by vm.appConfig.collectAsState()
    val webAccessInfo by vm.webAccessInfo.collectAsState()
    val loading by vm.webAccessLoading.collectAsState()
    val saving by vm.settingsSaving.collectAsState()
    var enabled by remember { mutableStateOf(appConfig?.webAccessEnabled ?: true) }
    var port by remember { mutableStateOf((appConfig?.webAccessPort ?: 8429).toString()) }
    var password by remember { mutableStateOf(appConfig?.webAccessPassword ?: "") }
    var saved by remember { mutableStateOf(false) }

    LaunchedEffect(appConfig) {
        enabled = appConfig?.webAccessEnabled ?: true
        port = (appConfig?.webAccessPort ?: 8429).toString()
        password = appConfig?.webAccessPassword ?: ""
    }
    LaunchedEffect(Unit) {
        vm.refreshWebAccessInfo()
    }

    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
    ) {
        Text("远程连接", style = MaterialTheme.typography.titleMedium)
        Text(
            "允许局域网内其他设备（电脑浏览器 / VSCode 侧边栏）通过 WebSocket 连接本机的 PAI 服务。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))

        // 状态卡
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.surfaceVariant,
            shape = MaterialTheme.shapes.medium,
        ) {
            Column(Modifier.padding(12.dp)) {
                val running = webAccessInfo?.get("running") as? Boolean ?: false
                val enabledFlag = webAccessInfo?.get("enabled") as? Boolean ?: enabled
                Text(
                    buildString {
                        append("服务状态：")
                        append(
                            when {
                                !enabledFlag -> "已关闭"
                                running -> "运行中"
                                else -> "未启动"
                            }
                        )
                    },
                    style = MaterialTheme.typography.bodyMedium,
                )
                val localUrl = webAccessInfo?.get("localUrl") as? String
                if (!localUrl.isNullOrEmpty()) {
                    Text(
                        "本机地址：$localUrl",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                @Suppress("UNCHECKED_CAST")
                val remoteUrls = webAccessInfo?.get("remoteUrls") as? List<String> ?: emptyList()
                remoteUrls.take(3).forEach { url ->
                    Text(
                        "局域网：$url",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
                if (running) {
                    Text(
                        "端口：${webAccessInfo?.get("port")}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
        Spacer(Modifier.height(16.dp))

        // 开关
        Row(
            Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("启用网络访问", style = MaterialTheme.typography.bodyLarge)
                Text(
                    "关闭后其他设备无法远程连接",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(checked = enabled, onCheckedChange = { enabled = it })
        }
        Spacer(Modifier.height(12.dp))

        // 端口
        OutlinedTextField(
            value = port,
            onValueChange = { port = it.filter(Char::isDigit).take(5) },
            label = { Text("端口") },
            modifier = Modifier.fillMaxWidth(),
            enabled = enabled,
            singleLine = true,
        )
        Spacer(Modifier.height(12.dp))

        // 密码
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("访问密码") },
            modifier = Modifier.fillMaxWidth(),
            enabled = enabled,
            singleLine = true,
        )
        Spacer(Modifier.height(16.dp))

        OutlinedButton(
            onClick = {
                scope.launch {
                    val portValue = port.toIntOrNull() ?: 8429
                    saved = vm.saveWebAccess(enabled, portValue, password.trim())
                }
            },
            enabled = !saving,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (saving) "保存中…" else "保存设置")
        }
        if (saved) {
            Spacer(Modifier.height(8.dp))
            Text(
                "已保存，服务已按新配置重启",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
            )
        }
        if (loading) {
            Spacer(Modifier.height(8.dp))
            Text("刷新状态中…", style = MaterialTheme.typography.bodySmall)
        }
    }
}

/** 通知设置（对齐 Vue NotificationTab）。 */
@Composable
private fun NotificationSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val appConfig by vm.appConfig.collectAsState()
    val saving by vm.settingsSaving.collectAsState()
    var notifEnabled by remember { mutableStateOf(appConfig?.messageNotificationEnabled ?: true) }
    var soundEnabled by remember { mutableStateOf(appConfig?.messageNotificationSoundEnabled ?: true) }
    var desktopNotice by remember { mutableStateOf(appConfig?.desktopOperationNoticeEnabled ?: false) }
    var saved by remember { mutableStateOf(false) }

    LaunchedEffect(appConfig) {
        notifEnabled = appConfig?.messageNotificationEnabled ?: true
        soundEnabled = appConfig?.messageNotificationSoundEnabled ?: true
        desktopNotice = appConfig?.desktopOperationNoticeEnabled ?: false
    }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Text("通知设置", style = MaterialTheme.typography.titleMedium)
        Text(
            "控制收到新消息时的提醒方式。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))

        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("消息通知", style = MaterialTheme.typography.bodyLarge)
                Text("收到新消息时弹出提醒", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = notifEnabled, onCheckedChange = { notifEnabled = it })
        }
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("通知声音", style = MaterialTheme.typography.bodyLarge)
                Text("通知时播放提示音", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = soundEnabled, onCheckedChange = { soundEnabled = it })
        }
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("桌面操作提醒", style = MaterialTheme.typography.bodyLarge)
                Text("桌面操作完成时通知", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = desktopNotice, onCheckedChange = { desktopNotice = it })
        }
        Spacer(Modifier.height(16.dp))
        OutlinedButton(
            onClick = {
                scope.launch {
                    saved = vm.saveNotificationAndAppearance(
                        messageNotificationEnabled = notifEnabled,
                        messageNotificationSoundEnabled = soundEnabled,
                        desktopOperationNoticeEnabled = desktopNotice,
                    )
                }
            },
            enabled = !saving,
            modifier = Modifier.fillMaxWidth(),
        ) { Text(if (saving) "保存中…" else "保存设置") }
        if (saved) {
            Spacer(Modifier.height(8.dp))
            Text("已保存", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
        }
    }
}

/** 外观设置（对齐 Vue AppearanceTab：语言 + 字号）。 */
@Composable
private fun AppearanceSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val appConfig by vm.appConfig.collectAsState()
    val saving by vm.settingsSaving.collectAsState()
    var language by remember { mutableStateOf(appConfig?.uiLanguage ?: "zh-CN") }
    var sizeScale by remember { mutableStateOf(appConfig?.uiSizeScale ?: 100) }
    var saved by remember { mutableStateOf(false) }

    LaunchedEffect(appConfig) {
        language = appConfig?.uiLanguage ?: "zh-CN"
        sizeScale = appConfig?.uiSizeScale ?: 100
    }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Text("外观设置", style = MaterialTheme.typography.titleMedium)
        Text(
            "界面语言与显示字号。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))
        Text("界面语言", style = MaterialTheme.typography.bodyLarge)
        val languages = listOf("zh-CN" to "简体中文", "en-US" to "English", "zh-TW" to "繁體中文")
        languages.forEach { (code, label) ->
            Row(
                Modifier.fillMaxWidth().clickable { language = code }.padding(vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                if (language == code) Text("✓", color = MaterialTheme.colorScheme.primary)
            }
        }
        Spacer(Modifier.height(12.dp))
        Text("界面字号", style = MaterialTheme.typography.bodyLarge)
        Text(
            "当前 $sizeScale%",
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.padding(vertical = 4.dp),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("小", style = MaterialTheme.typography.bodySmall)
            Slider(
                value = sizeScale.toFloat(),
                onValueChange = { sizeScale = it.toInt() },
                valueRange = 80f..130f,
                steps = 4,
                modifier = Modifier.weight(1f).padding(horizontal = 8.dp),
            )
            Text("大", style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.height(16.dp))
        OutlinedButton(
            onClick = {
                scope.launch {
                    saved = vm.saveNotificationAndAppearance(uiLanguage = language, uiSizeScale = sizeScale)
                }
            },
            enabled = !saving,
            modifier = Modifier.fillMaxWidth(),
        ) { Text(if (saving) "保存中…" else "保存设置") }
        if (saved) {
            Spacer(Modifier.height(8.dp))
            Text("已保存", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
        }
    }
}

/** 部门设置（对齐 Vue DepartmentTab：只读展示部门信息）。 */
@Composable
private fun DepartmentSettingsTab(vm: AppViewModel) {
    val appConfig by vm.appConfig.collectAsState()
    val departments = appConfig?.departments.orEmpty()

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Text("部门管理", style = MaterialTheme.typography.titleMedium)
        Text(
            "当前版本支持查看部门结构；编辑请在桌面端配置。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))
        if (departments.isEmpty()) {
            Text(
                "暂无部门（使用默认部门）",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            departments.forEach { dept ->
                Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Column(Modifier.padding(12.dp)) {
                        Text(
                            buildString {
                                append(dept.name ?: "未命名")
                                if (dept.isBuiltInAssistant == true) append("（助手）")
                            },
                            style = MaterialTheme.typography.titleSmall,
                        )
                        if (!dept.summary.isNullOrBlank()) {
                            Text(
                                dept.summary,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        if (dept.agentIds.isNotEmpty()) {
                            Text(
                                "人设：${dept.agentIds.size} 个",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

/** 部门树设置（对齐 Vue DepartmentTreeTab：只读层级展示）。 */
@Composable
private fun DepartmentTreeSettingsTab(vm: AppViewModel) {
    val appConfig by vm.appConfig.collectAsState()
    val departments = appConfig?.departments.orEmpty()

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Text("部门树", style = MaterialTheme.typography.titleMedium)
        Text(
            "部门之间的层级关系（只读）。",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))
        if (departments.isEmpty()) {
            Text(
                "暂无部门",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        } else {
            // 只展示顶层部门（无父部门引用的）
            val topLevel = departments.filter { d ->
                !departments.any { it.childDepartmentIds.contains(d.id) }
            }
            if (topLevel.isEmpty()) {
                // 无明确根时全部平铺
                departments.forEach { dept -> DeptTreeNode(dept, departments, 0) }
            } else {
                topLevel.forEach { dept -> DeptTreeNode(dept, departments, 0) }
            }
        }
    }
}

@Composable
private fun DeptTreeNode(
    dept: com.whitemoon319.pai.model.DepartmentConfig,
    all: List<com.whitemoon319.pai.model.DepartmentConfig>,
    depth: Int,
) {
    val children = all.filter { dept.childDepartmentIds.contains(it.id) }
    Card(Modifier.fillMaxWidth().padding(start = (depth * 16).dp, top = 3.dp, bottom = 3.dp)) {
        Column(Modifier.padding(10.dp)) {
            Text(
                dept.name ?: "未命名",
                style = MaterialTheme.typography.titleSmall,
            )
            if (!dept.summary.isNullOrBlank()) {
                Text(
                    dept.summary,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                )
            }
        }
    }
    children.forEach { child ->
        DeptTreeNode(child, all, depth + 1)
    }
}

/** 人设设置（对齐 Vue PersonaTab：列表 + 编辑姓名/提示词）。 */
@Composable
private fun PersonaSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val agents by vm.agents.collectAsState()
    val loading by vm.agentsLoading.collectAsState()
    val saving by vm.settingsSaving.collectAsState()
    var selectedId by remember { mutableStateOf<String?>(null) }
    var editName by remember { mutableStateOf("") }
    var editPrompt by remember { mutableStateOf("") }

    LaunchedEffect(Unit) { vm.loadAgents() }

    val selected = agents?.firstOrNull { it.id == selectedId }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("人设管理", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadAgents() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (agents.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无可用人设",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            // 人设选择
            agents!!.forEach { agent ->
                val isSelected = agent.id == selectedId
                Card(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 3.dp)
                        .clickable { selectedId = agent.id },
                ) {
                    Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(
                                buildString {
                                    append(agent.name ?: "未命名")
                                    if (agent.isBuiltInUser == true) append("（用户）")
                                    if (agent.isBuiltInSystem == true) append("（系统）")
                                },
                                style = MaterialTheme.typography.titleSmall,
                            )
                            val scopeTag = agent.source ?: agent.scope ?: ""
                            if (scopeTag.isNotBlank()) {
                                Text(scopeTag, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                        if (isSelected) Text("✓", color = MaterialTheme.colorScheme.primary)
                    }
                }
            }
        }

        selected?.let { agent ->
            Spacer(Modifier.height(12.dp))
            HorizontalDivider()
            Spacer(Modifier.height(8.dp))
            LaunchedEffect(agent.id) {
                editName = agent.name ?: ""
                editPrompt = agent.systemPrompt ?: ""
            }
            OutlinedTextField(
                value = editName,
                onValueChange = { editName = it },
                label = { Text("名称") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )
            Spacer(Modifier.height(8.dp))
            OutlinedTextField(
                value = editPrompt,
                onValueChange = { editPrompt = it },
                label = { Text("系统提示词") },
                modifier = Modifier.fillMaxWidth().heightIn(min = 120.dp),
            )
            Spacer(Modifier.height(12.dp))
            OutlinedButton(
                onClick = {
                    scope.launch {
                        val updated = agents!!.map { a ->
                            if (a.id == agent.id) a.copy(name = editName, systemPrompt = editPrompt) else a
                        }
                        vm.saveAgents(updated)
                    }
                },
                enabled = !saving,
                modifier = Modifier.fillMaxWidth(),
            ) { Text(if (saving) "保存中…" else "保存修改") }
        }
    }
}

/** 记忆设置（对齐 Vue MemoryTab：列表 + 删除）。 */
@Composable
private fun MemorySettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val memories by vm.memories.collectAsState()
    val loading by vm.memoryLoading.collectAsState()
    var pendingDelete by remember { mutableStateOf<Map<String, Any?>?>(null) }

    LaunchedEffect(Unit) { vm.loadMemories() }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("记忆管理", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadMemories() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (memories.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无记忆",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            LazyColumn(Modifier.weight(1f)) {
                items(memories!!.size) { index ->
                    val item = memories!![index]
                    val content = item["content"] as? String ?: item["judgment"] as? String ?: ""
                    val id = (item["id"] as? String) ?: ""
                    Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                content.take(80),
                                style = MaterialTheme.typography.bodySmall,
                                modifier = Modifier.weight(1f),
                            )
                            TextButton(onClick = { pendingDelete = item }) { Text("删除") }
                        }
                    }
                }
            }
        }
    }
    pendingDelete?.let { item ->
        AlertDialog(
            onDismissRequest = { pendingDelete = null },
            title = { Text("删除记忆") },
            text = { Text("确定删除这条记忆吗？") },
            confirmButton = {
                TextButton(onClick = {
                    scope.launch {
                        val id = (item["id"] as? String) ?: ""
                        vm.deleteMemory(id)
                        pendingDelete = null
                    }
                }) { Text("删除") }
            },
            dismissButton = { TextButton(onClick = { pendingDelete = null }) { Text("取消") } },
        )
    }
}

/** 日志设置（对齐 Vue LogsTab：运行日志）。 */
@Composable
private fun LogsSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val logs by vm.runtimeLogs.collectAsState()
    val loading by vm.runtimeLogsLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadRuntimeLogs() }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("运行日志", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadRuntimeLogs() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (logs.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无日志",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            LazyColumn(Modifier.weight(1f)) {
                items(logs!!.size) { index ->
                    val log = logs!![index]
                    val level = log["level"] as? String ?: "info"
                    val message = log["message"] as? String ?: ""
                    val time = log["createdAt"] as? String ?: ""
                    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp)) {
                        Text(
                            level.take(5).uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            color = when (level) {
                                "error" -> MaterialTheme.colorScheme.error
                                "warn" -> MaterialTheme.colorScheme.tertiary
                                else -> MaterialTheme.colorScheme.onSurfaceVariant
                            },
                            modifier = Modifier.width(52.dp),
                        )
                        Text(
                            message,
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.weight(1f),
                        )
                        Text(
                            time.takeLast(12),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

/** 存储设置（对齐 Vue StorageTab）。 */
@Composable
private fun StorageSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val overview by vm.storageOverview.collectAsState()
    val loading by vm.storageLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadStorageOverview() }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("存储用量", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadStorageOverview(refresh = true) } }, enabled = !loading) {
                Text(if (loading) "刷新中…" else "刷新")
            }
        }
        if (overview == null) {
            Text(if (loading) "加载中…" else "暂无数据", style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(8.dp))
        } else {
            overview!!.forEach { (key, value) ->
                Row(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Text(key, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(value?.toString() ?: "—", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

/** 用量设置（对齐 Vue UsageTab）。 */
@Composable
private fun UsageSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val overview by vm.usageOverview.collectAsState()
    val loading by vm.usageLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadUsageOverview() }

    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("用量统计", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadUsageOverview() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (overview == null) {
            Text(if (loading) "加载中…" else "暂无数据", style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(8.dp))
        } else {
            overview!!.forEach { (key, value) ->
                Row(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Text(key, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(value?.toString() ?: "—", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

/** MCP 设置（对齐 Vue McpTab：服务器列表）。 */
@Composable
private fun McpSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val servers by vm.mcpServers.collectAsState()
    val loading by vm.mcpLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadMcpServers() }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("MCP 服务器", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadMcpServers() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (servers.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无 MCP 服务器",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            LazyColumn(Modifier.weight(1f)) {
                items(servers!!.size) { index ->
                    val server = servers!![index]
                    Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Column(Modifier.padding(12.dp)) {
                            Text(
                                server["name"] as? String ?: "未命名",
                                style = MaterialTheme.typography.titleSmall,
                            )
                            val command = server["command"] as? String ?: ""
                            val url = server["url"] as? String ?: ""
                            Text(
                                if (command.isNotBlank()) command else if (url.isNotBlank()) url else "—",
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

/** 任务设置（对齐 Vue TaskTab）。 */
@Composable
private fun TaskSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val tasks by vm.tasks.collectAsState()
    val loading by vm.tasksLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadTasks() }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("定时任务", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadTasks() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (tasks.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无任务",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            LazyColumn(Modifier.weight(1f)) {
                items(tasks!!.size) { index ->
                    val task = tasks!![index]
                    Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Column(Modifier.padding(12.dp)) {
                            Text(task["title"] as? String ?: "未命名", style = MaterialTheme.typography.titleSmall)
                            val next = task["nextRunAt"] as? String ?: ""
                            Text(
                                if (next.isNotBlank()) "下次：$next" else "—",
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

/** 远程 IM 设置（对齐 Vue RemoteImTab：通道列表）。 */
@Composable
private fun RemoteImSettingsTab(vm: AppViewModel) {
    val scope = rememberCoroutineScope()
    val channels by vm.remoteImChannels.collectAsState()
    val loading by vm.remoteImLoading.collectAsState()

    LaunchedEffect(Unit) { vm.loadRemoteImChannels() }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("远程 IM", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            TextButton(onClick = { scope.launch { vm.loadRemoteImChannels() } }, enabled = !loading) { Text(if (loading) "加载中…" else "刷新") }
        }
        if (channels.isNullOrEmpty()) {
            Text(
                if (loading) "加载中…" else "暂无远程 IM 通道",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(16.dp),
            )
        } else {
            LazyColumn(Modifier.weight(1f)) {
                items(channels!!.size) { index ->
                    val channel = channels!![index]
                    Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Column(Modifier.padding(12.dp)) {
                            Text(
                                channel["name"] as? String ?: channel["platform"] as? String ?: "未命名",
                                style = MaterialTheme.typography.titleSmall,
                            )
                            val platform = channel["platform"] as? String ?: ""
                            val enabled = channel["enabled"] as? Boolean ?: false
                            Text(
                                "$platform · ${if (enabled) "已启用" else "未启用"}",
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

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ToolsSettingsTab(
    vm: AppViewModel,
    toolStatus: List<com.whitemoon319.pai.model.ToolLoadStatus>,
) {    val scope = rememberCoroutineScope()
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