package ai.easycall.app

import ai.easycall.app.viewmodel.AppViewModel
import android.app.Application
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * 全局 Application：持有应用级协程作用域与 AppViewModel。
 * 注意：此原生前端不依赖 Tauri IPC，只通过 ws://127.0.0.1:8429 与 Rust 后端通信。
 */
class PaiApplication : Application() {
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    val viewModel by lazy { AppViewModel(appScope) }

    override fun onCreate() {
        super.onCreate()
    }
}