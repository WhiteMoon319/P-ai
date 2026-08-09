package ai.easycall.app

import ai.easycall.app.ui.PaiApp
import ai.easycall.app.viewmodel.AppViewModel
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.activity.enableEdgeToEdge
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy

/**
 * P-AI Android 原生前端入口（Compose）。
 *
 * 路线 B：保留 Tauri 壳承载 Rust 后端运行时（super.onCreate 触发 Rust 初始化、启动
 * ws://127.0.0.1:8429 服务），但将根视图替换为 Jetpack Compose 原生 UI。
 * Rust 引擎创建 WebView 时经 [setContentView] 传入，我们把它挂到根容器底层保证后端
 * 正常运行，同时 Compose UI 覆盖其上作为前端。前后端通过 ws://8429 JSON-RPC 通信。
 */
class MainActivity : TauriActivity() {

    private lateinit var viewModel: AppViewModel
    private lateinit var root: FrameLayout
    private var composeView: ComposeView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()

        // 先构建根容器，保证 Rust 引擎后续 setContentView 时能挂到我们管理的内容上
        root = FrameLayout(this)
        super.onCreate(savedInstanceState)

        viewModel = (application as PaiApplication).viewModel
        ensureComposeView()
    }

    /**
     * Tauri 引擎创建好 WebView 后调用 Activity.setContentView(view)。
     * 这里把 Rust 的 WebView 放进根容器底层（保证后端与桥正常运行），
     * Compose UI 覆盖在其上。
     */
    override fun setContentView(view: View?) {
        if (view == null) return
        if (view === composeView) {
            super.setContentView(view)
            return
        }
        // 把 Rust WebView 挂在根容器的底层
        if (view.parent !== root) {
            if (view.parent is ViewGroup) {
                (view.parent as ViewGroup).removeView(view)
            }
            val lp = FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            )
            root.addView(view, 0, lp)
        }
        ensureComposeView()
        super.setContentView(root)
    }

    private fun ensureComposeView() {
        if (composeView != null) return
        val compose = ComposeView(this).apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                PaiApp(viewModel)
            }
        }
        composeView = compose
        root.addView(
            compose,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
    }

    override fun onDestroy() {
        viewModel.stop()
        super.onDestroy()
    }
}