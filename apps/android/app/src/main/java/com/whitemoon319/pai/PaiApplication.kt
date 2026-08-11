package com.whitemoon319.pai

import com.whitemoon319.pai.viewmodel.AppViewModel
import android.app.Application
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * 全局 Application：持有应用级协程作用域与 AppViewModel。
 * 原生前端不依赖 WebView/WS：Rust 后端由 PaiNative.init 在 MainActivity 中以
 * JNI 原生桥方式加载（JSON-RPC over JNI）。
 */
class PaiApplication : Application() {
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    val viewModel by lazy { AppViewModel(this, appScope) }

    override fun onCreate() {
        super.onCreate()
    }
}