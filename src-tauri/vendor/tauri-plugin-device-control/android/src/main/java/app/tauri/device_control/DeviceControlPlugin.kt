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

  /** root 可用性检测：`su` 可执行即视为有 root（实际提权结果由执行时暴露）。 */
  private fun isRootAvailable(): Boolean {
    for (path in listOf("/system/bin/su", "/system/xbin/su", "/sbin/su", "/su/bin/su")) {
      if (java.io.File(path).isFile) return true
    }
    return try {
      val process = ProcessBuilder("su", "-c", "id").redirectErrorStream(true).start()
      val alive = process.waitFor(3, TimeUnit.SECONDS)
      if (alive) process.destroyForcibly()
      true
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