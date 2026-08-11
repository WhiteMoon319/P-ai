package com.whitemoon319.pai.platform

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.whitemoon319.pai.R
import java.util.concurrent.atomic.AtomicInteger

/**
 * 后台任务保活前台服务（纯原生，不依赖任何 Tauri/WebView 运行时）。
 *
 * Rust 侧在任务启动（回复轮次开始 / 目标激活）时通过 native 事件队列推送
 * app.keepAlive {active:true}，任务全部结束（回复完成 / 目标结束）时推送
 * {active:false}。Kotlin 侧 [com.whitemoon319.pai.viewmodel.AppViewModel]
 * 消费该事件后调用本服务的 start/stop，把进程提升为前台，保证后台轮询 /
 * 网络请求不被系统回收或进入 Doze 深度限制。
 *
 * 保活不依赖通知权限：API 33+ 前台服务可脱离 POST_NOTIFICATIONS 运行，
 * 通知不显示但进程仍保持前台优先级；API 34+ 使用 specialUse 类型。
 */
class PaiForegroundService : Service() {
    companion object {
        private const val CHANNEL_ID = "pai_keep_alive"
        private const val NOTIFICATION_ID = 0x50414910

        // 活跃后台任务数（回复 + 目标），大于 0 时保活
        private val activeTaskCount = AtomicInteger(0)

        @JvmStatic
        fun start(context: Context) {
            val count = activeTaskCount.incrementAndGet()
            if (count > 1) return
            try {
                context.startForegroundService(Intent(context, PaiForegroundService::class.java))
            } catch (e: Exception) {
                // 后台启动受限（如部分厂商后台限制）时降级，不阻塞任务流程
                activeTaskCount.decrementAndGet()
                android.util.Log.w("PaiNotify", "keepAlive startForegroundService 失败: ${e.message}")
            }
        }

        @JvmStatic
        fun stop(context: Context) {
            val remaining = activeTaskCount.decrementAndGet()
            if (remaining > 0) return
            activeTaskCount.set(0)
            try {
                context.stopService(Intent(context, PaiForegroundService::class.java))
            } catch (e: Exception) {
                // 服务可能已停止，忽略
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        ensureChannel()
        startAsForeground(NOTIFICATION_ID, buildNotification())
    }

    override fun onDestroy() {
        // 解除前台状态并移除保活通知；live update 通知独立于本服务，不受影响。
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        super.onDestroy()
    }

    private fun ensureChannel() {
        val manager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        if (manager.getNotificationChannel(CHANNEL_ID) != null) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "后台任务保活",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "后台任务运行期间保持进程存活"
            setShowBadge(false)
        }
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_pai)
            .setContentTitle("PAI 正在后台运行任务")
            .setContentText("任务结束后自动停止")
            .setOngoing(true)
            .setSilent(true)
            .build()
    }

    private fun startAsForeground(notificationId: Int, notification: Notification) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                startForeground(
                    notificationId,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE,
                )
            } else {
                startForeground(notificationId, notification)
            }
        } catch (e: Exception) {
            // API 34+ specialUse 类型在部分设备可能受限；失败则停止服务，保活降级
            android.util.Log.w("PaiNotify", "keepAlive startForeground 失败: ${e.message}")
            stopSelf()
        }
    }
}
