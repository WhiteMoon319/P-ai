package com.whitemoon319.pai

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.whitemoon319.pai.native.PaiNative
import com.whitemoon319.pai.ui.PaiApp
import com.whitemoon319.pai.viewmodel.AppViewModel
import android.widget.Toast

/**
 * P-AI Android 原生入口（Compose，无 WebView）。
 *
 * 彻底拔掉 Tauri 运行时：本 Activity 不再继承 TauriActivity，也不创建任何 WebView。
 * Rust 后端以纯原生 .so 加载（[PaiNative.init] 自建 Tokio runtime + AppState），
 * 前后端通过 JNI 直接调用（JSON-RPC over JNI），见 PaiNative / native_bridge.rs。
 */
class MainActivity : ComponentActivity() {

    private lateinit var viewModel: AppViewModel

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        // Android 13+ 通知运行时权限（消息通知功能依赖）
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 1001)
        }

        viewModel = (application as PaiApplication).viewModel
        // 用应用数据目录初始化原生后端（等价旧 tauri 的 app_data_dir）。
        // dataDir = /data/user/0/<pkg>，与旧版一致，可复用现有配置/工作区/rootfs。
        val initResult = runCatching { PaiNative.init(dataDir.absolutePath) }.getOrElse { it.message }
        if (initResult != "ok") {
            Toast.makeText(this, "原生后端初始化失败: $initResult", Toast.LENGTH_LONG).show()
        }
        viewModel.start()

        setContent {
            PaiApp(viewModel)
        }
    }

    override fun onDestroy() {
        viewModel.stop()
        super.onDestroy()
    }
}
