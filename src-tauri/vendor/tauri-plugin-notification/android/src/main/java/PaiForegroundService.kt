package app.tauri.notification

import android.app.Notification
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import java.util.concurrent.ConcurrentHashMap

/**
 * 后台任务保活前台服务。
 *
 * 语义：仅当「应用处于后台」且存在 ongoing + promoted 的 live 通知（任务运行中）时，
 * 将最后一条活跃 live 通知绑定为前台服务通知，提升进程优先级，保证后台轮询 /
 * 网络请求不被系统回收或进入 Doze 深度限制。
 * 应用回到前台或所有 live 通知结束后服务自动停止（通知保留，由后续终态通知覆盖）。
 */
class PaiForegroundService : Service() {
  companion object {
    private const val EXTRA_NOTIFICATION_ID = "notificationId"
    private const val EXTRA_NOTIFICATION = "notification"

    // 活跃 live 通知 id 集合（CHAT / GOAL 各一条）与最近一次通知对象缓存
    private val activeLiveIds: MutableSet<Int> = ConcurrentHashMap.newKeySet()
    private val liveNotifications = ConcurrentHashMap<Int, Notification>()

    @Volatile
    private var appInForeground = true

    @JvmStatic
    fun startLive(context: Context, notificationId: Int, notification: Notification) {
      activeLiveIds.add(notificationId)
      liveNotifications[notificationId] = notification
      if (!appInForeground) {
        startService(context, notificationId, notification)
      }
    }

    @JvmStatic
    fun stopLive(context: Context, notificationId: Int) {
      activeLiveIds.remove(notificationId)
      liveNotifications.remove(notificationId)
      if (activeLiveIds.isEmpty()) {
        try {
          context.stopService(Intent(context, PaiForegroundService::class.java))
        } catch (e: Exception) {
          // 服务可能已停止，忽略
        }
      }
    }

    @JvmStatic
    fun onAppForegroundChanged(context: Context, foreground: Boolean) {
      appInForeground = foreground
      if (foreground) {
        // 回到前台：解除前台服务，通知保留（onDestroy DETACH），避免前台常驻服务
        try {
          context.stopService(Intent(context, PaiForegroundService::class.java))
        } catch (e: Exception) {
          // 服务可能已停止，忽略
        }
      } else if (activeLiveIds.isNotEmpty()) {
        // 退到后台且仍有任务运行：用最后一条活跃 live 通知重新启动保活
        val lastId = activeLiveIds.maxOrNull()
        val notification = lastId?.let { liveNotifications[it] }
        if (lastId != null && notification != null) {
          startService(context, lastId, notification)
        }
      }
    }

    private fun startService(context: Context, notificationId: Int, notification: Notification) {
      val intent = Intent(context, PaiForegroundService::class.java).apply {
        putExtra(EXTRA_NOTIFICATION_ID, notificationId)
        putExtra(EXTRA_NOTIFICATION, notification)
      }
      try {
        context.startForegroundService(intent)
      } catch (e: Exception) {
        // 后台启动受限（如部分厂商后台限制）时降级为普通通知，不阻断通知流程
        activeLiveIds.remove(notificationId)
      }
    }
  }

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    if (intent != null && intent.hasExtra(EXTRA_NOTIFICATION_ID)) {
      val notificationId = intent.getIntExtra(EXTRA_NOTIFICATION_ID, 0)
      val notification = getNotificationExtra(intent)
      if (notification != null) {
        startAsForeground(notificationId, notification)
        return START_STICKY
      }
    }
    stopSelf()
    return START_NOT_STICKY
  }

  override fun onDestroy() {
    // 解除前台状态但保留通知，由后续终态通知覆盖；避免与插件 notify 竞态误删。
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
      stopForeground(STOP_FOREGROUND_DETACH)
    } else {
      @Suppress("DEPRECATION")
      stopForeground(false)
    }
    super.onDestroy()
  }

  @Suppress("DEPRECATION")
  private fun getNotificationExtra(intent: Intent): Notification? {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      intent.getParcelableExtra(EXTRA_NOTIFICATION, Notification::class.java)
    } else {
      intent.getParcelableExtra(EXTRA_NOTIFICATION)
    }
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
      stopSelf()
    }
  }
}
