// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.device_control

import android.app.Activity
import android.content.ComponentName
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.IBinder
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import rikka.shizuku.Shizuku
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * 提权设备控制插件（Shizuku UserService 首选 / root 兜底）——仿 MAA-Meow 的
 * RemoteServiceManager + ShizukuRemoteServiceConnector 模式：
 *
 *  - Shizuku：`bindUserService` 把 [DeviceControlServiceImpl] 以 shell 身份启动在
 *    独立进程，经 AIDL（[IDeviceControlService]）调用执行提权命令。
 *  - root：`su` 存在时 `su -c <cmd>` 兜底。
 *
 * Rust 侧只负责命令白名单校验与调用转发；真正的提权 shell 执行在这里完成。
 */
@TauriPlugin
class DeviceControlPlugin(private val activity: Activity) : Plugin(activity) {

  @InvokeArg
  class ExecuteCommandArgs {
    var command: String? = null
    var timeoutMs: Long = 30_000
  }

  /** 注入式触控请求（Rust 侧 TouchAction 枚举白名单校验后转发）。 */
  @InvokeArg
  class TouchInvokeArgs {
    /** 动作：tap | swipe | key */
    var action: String? = null
    var x: Int? = null
    var y: Int? = null
    var x1: Int? = null
    var y1: Int? = null
    var x2: Int? = null
    var y2: Int? = null
    var durationMs: Long? = null
    var keycode: Int? = null
  }

  /** 查询提权状态。 */
  @Command
  fun status(invoke: Invoke) {
    val shizukuAvailable = isShizukuAvailable()
    val shizukuGranted = shizukuAvailable && isShizukuGranted()
    val rootAvailable = isRootAvailable()
    val privilegeState = when {
      shizukuGranted -> "shizuku_ready"
      rootAvailable -> "root_ready"
      shizukuAvailable -> "shizuku_pending"
      else -> "disabled"
    }
    val result = JSObject()
    result.put("shizukuAvailable", shizukuAvailable)
    result.put("shizukuGranted", shizukuGranted)
    result.put("rootAvailable", rootAvailable)
    result.put("privilegeState", privilegeState)
    invoke.resolve(result)
  }

  /** 触发 Shizuku 授权弹窗（异步，结果不阻塞调用方）。 */
  @Command
  fun requestPrivilege(invoke: Invoke) {
    if (!isShizukuAvailable()) {
      invoke.reject("Shizuku 服务不可用：请先安装并激活 Shizuku（ADB 或 root 方式），或使用 root")
      return
    }
    if (isShizukuGranted()) {
      invoke.resolveObject("already granted")
      return
    }
    if (Shizuku.isPreV11()) {
      invoke.resolveObject("pre-v11 auto granted")
      return
    }
    try {
      val requestCode = (1000..9999).random()
      val listener = object : Shizuku.OnRequestPermissionResultListener {
        override fun onRequestPermissionResult(code: Int, grantResult: Int) {
          if (code != requestCode) return
          try {
            Shizuku.removeRequestPermissionResultListener(this)
          } catch (_: Exception) {
          }
          // 授权结果经插件事件回传前端（计划 5.1/5.5 privilegeChanged 契约），
          // 前端 registerListener 订阅后即时刷新提权状态，不再只靠轮询。
          try {
            val payload = JSObject()
            payload.put("granted", grantResult == PackageManager.PERMISSION_GRANTED)
            triggerObject("device_control_privilege_changed", payload)
          } catch (_: Exception) {
          }
        }
      }
      Shizuku.addRequestPermissionResultListener(listener)
      Shizuku.requestPermission(requestCode)
      invoke.resolveObject("requested")
    } catch (e: Exception) {
      invoke.reject("请求 Shizuku 授权失败: ${e.message}")
    }
  }

  /**
   * 以提权身份执行受控命令。
   *
   * Rust 侧已做命令白名单校验，此处仅执行并收集输出。Shizuku 优先，
   * root 兜底。stdout/stderr 并发读取避免管道缓冲死锁，带超时。
   */
  @Command
  fun executeCommand(invoke: Invoke) {
    val args = invoke.parseArgs(ExecuteCommandArgs::class.java)
    val command = args.command?.trim().orEmpty()
    if (command.isEmpty()) {
      invoke.reject("缺少命令")
      return
    }
    val timeoutMs = if (args.timeoutMs in 1..600_000L) args.timeoutMs else 30_000L

    val result = runCatching {
      if (isShizukuGranted()) {
        executeViaShizuku(command, timeoutMs)
      } else if (isRootAvailable()) {
        executeViaSu(command, timeoutMs)
      } else {
        ErrorResult("无可用提权通道：请先通过 Shizuku 授权或开启 root")
      }
    }.getOrElse { err ->
      ErrorResult("命令执行异常: ${err.message ?: err.javaClass.simpleName}")
    }

    if (result is ErrorResult) {
      invoke.reject(result.message)
      return
    }
    val out = result as ExecResult
    val obj = JSObject()
    obj.put("exitCode", out.exitCode)
    obj.put("stdout", out.stdout)
    obj.put("stderr", out.stderr)
    invoke.resolve(obj)
  }

  /**
   * 注入式触控（MAA-Meow 语义）。
   *
   * Shizuku 首选：UserService（shell 身份）进程内反射 `injectInputEvent` 注入；
   * root 兜底：`su -c input ...` 降级（计划 v1 允许）。事件序列统一在服务端
   * Binder 线程执行，不占用应用进程 UI 线程。
   */
  @Command
  fun injectTouch(invoke: Invoke) {
    val args = invoke.parseArgs(TouchInvokeArgs::class.java)
    val action = args.action?.trim().orEmpty()
    if (action.isEmpty()) {
      invoke.reject("缺少触控动作")
      return
    }
    if (!isShizukuGranted() && !isRootAvailable()) {
      invoke.reject("无可用提权通道：请先通过 Shizuku 授权或开启 root")
      return
    }
    try {
      val ok = if (isShizukuGranted()) {
        injectTouchViaShizuku(args, action)
      } else {
        injectTouchViaSu(args, action)
      }
      if (!ok) {
        invoke.reject("触控注入失败：设备拒绝注入（确认 Shizuku 授权与 INJECT_EVENTS 权限）")
        return
      }
      invoke.resolveObject("injected")
    } catch (e: Exception) {
      invoke.reject("触控注入失败: ${e.message ?: e.javaClass.simpleName}")
    }
  }

  /** Shizuku 注入：经 UserService AIDL 调用（服务端 Binder 线程内完成事件序列）。 */
  private fun injectTouchViaShizuku(args: TouchInvokeArgs, action: String): Boolean {
    val service = obtainShizukuService()
      ?: throw IllegalStateException("Shizuku 服务连接失败（服务未启动或超时）")
    return when (action) {
      "tap" -> {
        val x = requireNotNull(args.x) { "缺少坐标 x" }
        val y = requireNotNull(args.y) { "缺少坐标 y" }
        service.tap(x, y)
      }
      "swipe" -> {
        val x1 = requireNotNull(args.x1) { "缺少 x1" }
        val y1 = requireNotNull(args.y1) { "缺少 y1" }
        val x2 = requireNotNull(args.x2) { "缺少 x2" }
        val y2 = requireNotNull(args.y2) { "缺少 y2" }
        val duration = (args.durationMs ?: 300L).coerceIn(0L, 10_000L)
        service.swipe(x1, y1, x2, y2, duration)
      }
      "key" -> {
        val keycode = requireNotNull(args.keycode) { "缺少 keycode" }
        service.key(keycode)
      }
      else -> throw IllegalArgumentException("未知触控动作: $action")
    }
  }

  /** root 兜底：`su -c input ...`（计划 v1 允许的降级路径）。 */
  private fun injectTouchViaSu(args: TouchInvokeArgs, action: String): Boolean {
    val command = when (action) {
      "tap" -> {
        val x = requireNotNull(args.x) { "缺少坐标 x" }
        val y = requireNotNull(args.y) { "缺少坐标 y" }
        "input tap $x $y"
      }
      "swipe" -> {
        val x1 = requireNotNull(args.x1) { "缺少 x1" }
        val y1 = requireNotNull(args.y1) { "缺少 y1" }
        val x2 = requireNotNull(args.x2) { "缺少 x2" }
        val y2 = requireNotNull(args.y2) { "缺少 y2" }
        val duration = args.durationMs
        if (duration != null && duration > 0) {
          "input swipe $x1 $y1 $x2 $y2 $duration"
        } else {
          "input swipe $x1 $y1 $x2 $y2"
        }
      }
      "key" -> {
        val keycode = requireNotNull(args.keycode) { "缺少 keycode" }
        "input keyevent $keycode"
      }
      else -> throw IllegalArgumentException("未知触控动作: $action")
    }
    val result = executeViaSu(command, 15_000)
    return result.exitCode == 0
  }

  // ---- helpers ----

  private fun isShizukuAvailable(): Boolean = try {
    Shizuku.pingBinder()
  } catch (_: Exception) {
    false
  }

  private fun isShizukuGranted(): Boolean = try {
    isShizukuAvailable() &&
      Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
  } catch (_: Exception) {
    false
  }

  /** root 可用性检测：`su -c id` 必须真实提权到 uid=0 才判定可用，且结果缓存 30s
   * 避免每次 status 查询都同步跑进程；文件存在但 SELinux/权限拒绝的误报不再出现。 */
  @Volatile
  private var rootAvailableCache = false
  @Volatile
  private var rootCheckedAt = 0L

  private fun isRootAvailable(): Boolean {
    val now = System.currentTimeMillis()
    if (now - rootCheckedAt < 30_000L) return rootAvailableCache
    synchronized(this) {
      if (now - rootCheckedAt < 30_000L) return rootAvailableCache
      val probeResult = probeRoot()
      rootAvailableCache = probeResult
      rootCheckedAt = System.currentTimeMillis()
      return probeResult
    }
  }

  private fun probeRoot(): Boolean {
    return try {
      val process = ProcessBuilder("su", "-c", "id").redirectErrorStream(true).start()
      val finished = process.waitFor(2, TimeUnit.SECONDS)
      if (!finished) {
        process.destroyForcibly()
        return false
      }
      val output = process.inputStream.bufferedReader().readText()
      process.waitFor(1, TimeUnit.SECONDS)
      // Magisk/KernelSU 的 su 提权成功后 id 输出 uid=0；SELinux 拒绝或未授权时不输出
      output.contains("uid=0")
    } catch (_: Exception) {
      false
    }
  }

  // ---- Shizuku UserService 连接（仿 MAA-Meow ShizukuRemoteServiceConnector） ----

  private var boundShizukuService: IDeviceControlService? = null
  private var boundUserServiceArgs: Shizuku.UserServiceArgs? = null
  private var boundConnection: ServiceConnection? = null

  /** 懒连接：首次需要时 bindUserService，成功后缓存复用。 */
  private fun executeViaShizuku(command: String, timeoutMs: Long): ExecResult {
    val service = obtainShizukuService()
      ?: return ErrorResult("Shizuku 服务连接失败（服务未启动或超时）")
    return try {
      val json = service.execute(command, timeoutMs)
      val obj = JSONObject(json)
      ExecResult(obj.getInt("exitCode"), obj.getString("stdout"), obj.getString("stderr"))
    } catch (e: Exception) {
      ErrorResult("Shizuku 命令执行异常: ${e.message ?: e.javaClass.simpleName}")
    }
  }

  @Synchronized
  private fun obtainShizukuService(): IDeviceControlService? {
    boundShizukuService?.let { return it }
    if (!isShizukuGranted()) return null

    val latch = CountDownLatch(1)
    val args = Shizuku.UserServiceArgs(
      ComponentName(activity.packageName, DeviceControlServiceImpl::class.java.name)
    ).apply {
      processNameSuffix("device_control")
      daemon(false)
      version(1)
    }
    val connection = object : ServiceConnection {
      override fun onServiceConnected(name: ComponentName?, binder: IBinder?) {
        if (binder != null) {
          boundShizukuService = IDeviceControlService.Stub.asInterface(binder)
        }
        latch.countDown()
      }

      override fun onServiceDisconnected(name: ComponentName?) {
        boundShizukuService = null
        latch.countDown()
      }
    }
    boundUserServiceArgs = args
    boundConnection = connection
    return try {
      Shizuku.bindUserService(args, connection)
      if (!latch.await(20, TimeUnit.SECONDS)) {
        null
      } else {
        boundShizukuService
      }
    } catch (e: Exception) {
      null
    }
  }

  // ---- root 兜底 ----

  private fun executeViaSu(command: String, timeoutMs: Long): ExecResult {
    val process = ProcessBuilder("su", "-c", command).redirectErrorStream(false).start()
    return collectProcess(process, timeoutMs)
  }

  /** 并发读 stdout/stderr + 带超时 waitFor，避免管道缓冲满死锁。 */
  private fun collectProcess(process: Process, timeoutMs: Long): ExecResult {
    val stdoutRef = java.util.concurrent.atomic.AtomicReference("")
    val stderrRef = java.util.concurrent.atomic.AtomicReference("")
    val outDone = CountDownLatch(1)
    val errDone = CountDownLatch(1)

    val outThread = Thread {
      try {
        stdoutRef.set(process.inputStream.bufferedReader().readText())
      } catch (_: Exception) {
      } finally {
        outDone.countDown()
      }
    }
    val errThread = Thread {
      try {
        stderrRef.set(process.errorStream.bufferedReader().readText())
      } catch (_: Exception) {
      } finally {
        errDone.countDown()
      }
    }
    outThread.isDaemon = true
    errThread.isDaemon = true
    outThread.start()
    errThread.start()

    val finished = try {
      process.waitFor(timeoutMs, TimeUnit.MILLISECONDS)
    } catch (e: InterruptedException) {
      Thread.currentThread().interrupt()
      false
    }
    if (!finished) {
      process.destroyForcibly()
      outDone.await(2, TimeUnit.SECONDS)
      errDone.await(2, TimeUnit.SECONDS)
      return ExecResult(-1, stdoutRef.get(), "命令超时（${timeoutMs}ms）\n${stderrRef.get()}")
    }
    outDone.await(2, TimeUnit.SECONDS)
    errDone.await(2, TimeUnit.SECONDS)
    return ExecResult(process.exitValue(), stdoutRef.get(), stderrRef.get())
  }

  private open class ExecResult(val exitCode: Int, val stdout: String, val stderr: String)

  private class ErrorResult(message: String) : ExecResult(-1, "", message) {
    val message: String = message
  }
}