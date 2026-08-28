// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.device_control

import android.os.Process
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import kotlin.system.exitProcess

/**
 * 提权命令执行服务——由 Shizuku 以 shell（或 root）身份启动在独立进程，
 * 直接以该身份执行命令，绕开应用自身 UID 的权限限制（同 MAA-Meow 的
 * RemoteServiceImpl 模式）。stdout/stderr 并发读取避免管道缓冲死锁，带超时。
 */
class DeviceControlServiceImpl : IDeviceControlService.Stub() {

  /** 执行提权命令，返回 JSON：{"exitCode":N,"stdout":"...","stderr":"..."}。 */
  override fun execute(command: String, timeoutMs: Long): String {
    val result = collectCommand(command, timeoutMs)
    val json = JSONObject()
    json.put("exitCode", result.exitCode)
    json.put("stdout", result.stdout)
    json.put("stderr", result.stderr)
    return json.toString()
  }

  /** Shizuku server 在移除服务时调用：清理并退出当前进程。 */
  override fun destroy() {
    Process.killProcess(Process.myPid())
    exitProcess(0)
  }

  private data class CommandResult(val exitCode: Int, val stdout: String, val stderr: String)

  private fun collectCommand(command: String, timeoutMs: Long): CommandResult {
    return try {
      val process = ProcessBuilder("sh", "-c", command).start()
      val stdoutRef = AtomicReference("")
      val stderrRef = AtomicReference("")
      val outDone = CountDownLatch(1)
      val errDone = CountDownLatch(1)

      val outThread = Thread {
        try {
          stdoutRef.set(BufferedReader(InputStreamReader(process.inputStream)).readText())
        } catch (_: Exception) {
        } finally {
          outDone.countDown()
        }
      }
      val errThread = Thread {
        try {
          stderrRef.set(BufferedReader(InputStreamReader(process.errorStream)).readText())
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
        return CommandResult(-1, stdoutRef.get(), "命令超时（${timeoutMs}ms）\n${stderrRef.get()}")
      }
      outDone.await(2, TimeUnit.SECONDS)
      errDone.await(2, TimeUnit.SECONDS)
      CommandResult(process.exitValue(), stdoutRef.get(), stderrRef.get())
    } catch (e: Exception) {
      CommandResult(-1, "", "命令执行异常: ${e.message ?: e.javaClass.simpleName}")
    }
  }

  // ---- 注入式触控（MAA-Meow InputControlUtils 语义，见 InputInjector）----

  private val inputInjector = InputInjector()

  override fun tap(x: Int, y: Int): Boolean = inputInjector.tap(x, y)

  override fun swipe(x1: Int, y1: Int, x2: Int, y2: Int, durationMs: Long): Boolean =
    inputInjector.swipe(x1, y1, x2, y2, durationMs)

  override fun key(keycode: Int): Boolean = inputInjector.key(keycode)
}