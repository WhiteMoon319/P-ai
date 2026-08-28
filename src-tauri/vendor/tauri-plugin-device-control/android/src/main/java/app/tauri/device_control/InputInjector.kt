// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.device_control

import android.os.IBinder
import android.os.IInterface
import android.os.SystemClock
import android.util.Log
import android.view.InputDevice
import android.view.InputEvent
import android.view.KeyEvent
import android.view.MotionEvent
import java.lang.reflect.Method

/**
 * 注入式触控——参照 MAA-Meow `InputControlUtils` + `third/wrappers/InputManager` 链路：
 *
 *  - 反射 `ServiceManager.getService("input")` 拿 `IInputManager`（绕过隐藏 API 限制）；
 *  - 反射调用其 `injectInputEvent(InputEvent, int)` 注入 MotionEvent / KeyEvent。
 *
 * 事件语义与 MAA-Meow 逐行对齐：
 *  - DOWN 用 `WAIT_FOR_FINISH`（确保起始状态被系统成功接收），MOVE/UP 用 `ASYNC`；
 *  - down 前若上一次序列未正常结束，强制发送 `ACTION_CANCEL` 清空触控槽位；
 *  - `currentDownTime` 跨事件保持（MOVE/UP 必须带相同 downTime）；
 *  - pressure：down/move=1.0，up/cancel=0.0；size=1.0；`SOURCE_TOUCHSCREEN`；displayId=0。
 *
 * 仅应运行在 Shizuku UserService（shell 身份）进程内；INJECT_EVENTS 权限由 shell uid 天然持有。
 */
class InputInjector {

  companion object {
    private const val TAG = "InputInjector"

    private const val INJECT_MODE_ASYNC = 0
    private const val INJECT_MODE_WAIT_FOR_FINISH = 2

    private const val DEFAULT_DEVICE_ID = 0
    private const val DEFAULT_SOURCE = InputDevice.SOURCE_TOUCHSCREEN

    private val pointerProperties = arrayOf(
      MotionEvent.PointerProperties().apply {
        id = 0
        toolType = MotionEvent.TOOL_TYPE_FINGER
      }
    )
    private val pointerCoords = arrayOf(MotionEvent.PointerCoords())

    @Volatile
    private var injectMethod: Method? = null
    @Volatile
    private var inputManager: Any? = null
  }

  private var currentDownTime = 0L

  private fun ensureInjectMethod(): Method {
    injectMethod?.let { return it }
    synchronized(this) {
      injectMethod?.let { return it }
      // android.os.ServiceManager.getService("input") -> IBinder
      val getService = Class.forName("android.os.ServiceManager")
        .getDeclaredMethod("getService", String::class.java)
      val binder = getService.invoke(null, "input") as IBinder
      // android.hardware.input.IInputManager$Stub.asInterface(IBinder)
      val stubClass = Class.forName("android.hardware.input.IInputManager\$Stub")
      val asInterface = stubClass.getMethod("asInterface", IBinder::class.java)
      inputManager = asInterface.invoke(null, binder) as IInterface
      val method = inputManager!!.javaClass.getMethod("injectInputEvent", InputEvent::class.java, Int::class.javaPrimitiveType)
      injectMethod = method
      return method
    }
  }

  private fun invokeInject(event: InputEvent, mode: Int): Boolean {
    return try {
      ensureInjectMethod().invoke(inputManager, event, mode) as Boolean
    } catch (e: Exception) {
      Log.w(TAG, "injectInputEvent 失败: ${e.message ?: e.javaClass.simpleName}")
      false
    }
  }

  private fun setPointerCoords(x: Float, y: Float, pressure: Float) {
    val coords = pointerCoords[0]
    coords.x = if (x > 0f) x else 0f
    coords.y = if (y > 0f) y else 0f
    coords.pressure = pressure
    coords.size = 1.0f
  }

  private fun obtainTouchEvent(downTime: Long, eventTime: Long, action: Int, x: Float, y: Float, pressure: Float): MotionEvent {
    setPointerCoords(x, y, pressure)
    return MotionEvent.obtain(
      downTime, eventTime, action,
      1, pointerProperties, pointerCoords,
      0, 0,
      1.0f, 1.0f,
      DEFAULT_DEVICE_ID, 0, DEFAULT_SOURCE, 0
    )
  }

  /** down：起始状态必须 WAIT_FOR_FINISH 模式确保被系统接收。 */
  @Synchronized
  fun touchDown(x: Int, y: Int): Boolean {
    // 上一序列未正常结束（异常/连续 down）时先发 CANCEL 清空触控槽位
    if (currentDownTime != 0L) {
      val cancel = obtainTouchEvent(
        currentDownTime, SystemClock.uptimeMillis(),
        MotionEvent.ACTION_CANCEL, x.toFloat(), y.toFloat(), 0.0f
      )
      invokeInject(cancel, INJECT_MODE_ASYNC)
      cancel.recycle()
    }

    currentDownTime = SystemClock.uptimeMillis()
    val down = obtainTouchEvent(
      currentDownTime, currentDownTime,
      MotionEvent.ACTION_DOWN, x.toFloat(), y.toFloat(), 1.0f
    )
    val ok = invokeInject(down, INJECT_MODE_WAIT_FOR_FINISH)
    down.recycle()
    return ok
  }

  /** move：保持同一 downTime，ASYNC 注入。 */
  @Synchronized
  fun touchMove(x: Int, y: Int): Boolean {
    if (currentDownTime == 0L) return false
    val eventTime = SystemClock.uptimeMillis()
    val move = obtainTouchEvent(
      currentDownTime, eventTime,
      MotionEvent.ACTION_MOVE, x.toFloat(), y.toFloat(), 1.0f
    )
    val ok = invokeInject(move, INJECT_MODE_ASYNC)
    move.recycle()
    return ok
  }

  /** up：抬起后重置 downTime。 */
  @Synchronized
  fun touchUp(x: Int, y: Int): Boolean {
    if (currentDownTime == 0L) return false
    val eventTime = SystemClock.uptimeMillis()
    val up = obtainTouchEvent(
      currentDownTime, eventTime,
      MotionEvent.ACTION_UP, x.toFloat(), y.toFloat(), 0.0f
    )
    val ok = invokeInject(up, INJECT_MODE_ASYNC)
    currentDownTime = 0L
    up.recycle()
    return ok
  }

  /** 按键：down（WAIT_FOR_FINISH）+ up（ASYNC）。 */
  fun keyDown(keyCode: Int): Boolean {
    val downTime = SystemClock.uptimeMillis()
    val keyEvent = KeyEvent(downTime, downTime, KeyEvent.ACTION_DOWN, keyCode, 0)
    return invokeInject(keyEvent, INJECT_MODE_WAIT_FOR_FINISH)
  }

  fun keyUp(keyCode: Int): Boolean {
    val upTime = SystemClock.uptimeMillis()
    val keyEvent = KeyEvent(upTime, upTime, KeyEvent.ACTION_UP, keyCode, 0)
    return invokeInject(keyEvent, INJECT_MODE_ASYNC)
  }

  /** 手势序列辅助（Plugin 侧编排）：tap = down + 50ms + up。 */
  fun tap(x: Int, y: Int): Boolean {
    if (!touchDown(x, y)) return false
    Thread.sleep(50)
    return touchUp(x, y)
  }

  /** 手势序列辅助：swipe = down + 逐帧 move + up（帧间隔约 10ms，总时长对齐请求值）。 */
  fun swipe(x1: Int, y1: Int, x2: Int, y2: Int, durationMs: Long): Boolean {
    if (durationMs <= 0) {
      // 即时滑动：down + 单帧 move + up
      return touchDown(x1, y1) && touchMove(x2, y2) && touchUp(x2, y2)
    }
    if (!touchDown(x1, y1)) return false
    val frameMs = 10L
    val steps = ((durationMs + frameMs - 1) / frameMs).toInt().coerceAtLeast(2)
    val sleepPerStep = durationMs / steps
    for (i in 1..steps) {
      val ratio = i.toFloat() / steps
      val x = x1 + ((x2 - x1) * ratio).toInt()
      val y = y1 + ((y2 - y1) * ratio).toInt()
      if (!touchMove(x, y)) return false
      if (sleepPerStep > 0) Thread.sleep(sleepPerStep)
    }
    return touchUp(x2, y2)
  }

  /** 按键序列：keyDown + 10ms + keyUp。 */
  fun key(keyCode: Int): Boolean {
    if (!keyDown(keyCode)) return false
    Thread.sleep(10)
    return keyUp(keyCode)
  }
}