package com.whitemoon319.pai.native

/**
 * P-AI Android 原生桥：Kotlin ↔ Rust 的 JNI 通信入口。
 *
 * 彻底拔掉 Tauri 运行时后，Rust 后端以纯原生 .so 形式加载（自建 Tokio runtime +
 * AppState，见 src-tauri/src/native_bridge.rs），不再依赖 WebView/WS。
 *
 * - [init] 用应用数据目录（context.dataDir）初始化后端，返回 "ok" 或错误信息。
 * - [call] 同步执行 JSON-RPC（method/params → result/error），返回响应 JSON 字符串。
 * - [pollEvents] 拉取后端下行事件（流式 delta / 通知），当前返回 "[]"，后续轮次补齐。
 */
object PaiNative {
    init {
        System.loadLibrary("easy_call_ai_lib")
    }

    @JvmStatic
    external fun init(appRoot: String): String

    @JvmStatic
    external fun call(requestJson: String): String

    @JvmStatic
    external fun pollEvents(): String
}
