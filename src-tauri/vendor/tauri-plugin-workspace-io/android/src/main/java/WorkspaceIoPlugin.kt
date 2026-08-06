// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.workspace_io

import android.app.Activity
import android.net.Uri
import android.provider.OpenableColumns
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.io.FileOutputStream
import java.io.IOException

@InvokeArg
class ImportStreamArgs {
  var uri: String? = null
  var targetPath: String? = null
}

@InvokeArg
class ResolveNameArgs {
  var uri: String? = null
}

/**
 * 把 Android `content://` URI 流式导入 P-AI 沙盒工作区目标路径。
 *
 * WebView 只传 URI 字符串，不接触文件字节；Kotlin 侧用 ContentResolver
 * 打开输入流，分块写入沙盒 `targetPath`（Rust 侧已完成绝对路径校验）。
 */
@TauriPlugin
class WorkspaceIoPlugin(private val activity: Activity) : Plugin(activity) {

  @Command
  fun importStream(invoke: Invoke) {
    val args = invoke.parseArgs(ImportStreamArgs::class.java)
    val uriText = args.uri?.trim().orEmpty()
    val targetText = args.targetPath?.trim().orEmpty()
    if (uriText.isEmpty()) {
      invoke.reject("缺少 content URI")
      return
    }
    if (targetText.isEmpty()) {
      invoke.reject("缺少目标路径")
      return
    }
    val uri = Uri.parse(uriText)
    if (uri.scheme == null) {
      invoke.reject("不是有效的 content URI: $uriText")
      return
    }

    val targetFile = File(targetText)
    var bytesWritten = 0L
    try {
      targetFile.parentFile?.mkdirs()
      activity.contentResolver.openInputStream(uri).use { input ->
        if (input == null) {
          invoke.reject("无法打开所选内容")
          return
        }
        FileOutputStream(targetFile).use { output ->
          val buffer = ByteArray(64 * 1024)
          while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            output.write(buffer, 0, read)
            bytesWritten += read
          }
        }
      }
    } catch (e: IOException) {
      invoke.reject("流式写入失败: ${e.message}")
      return
    } catch (e: SecurityException) {
      invoke.reject("无法读取所选内容: ${e.message}")
      return
    } catch (e: Exception) {
      invoke.reject("导入失败: ${e.message}")
      return
    }

    if (targetFile.length() != bytesWritten) {
      invoke.reject("写入字节数不一致")
      return
    }
    val result = JSObject()
    result.put("bytes", bytesWritten)
    result.put("path", targetFile.absolutePath)
    invoke.resolve(result)
  }

  @Command
  fun resolveDisplayName(invoke: Invoke) {
    val args = invoke.parseArgs(ResolveNameArgs::class.java)
    val uriText = args.uri?.trim().orEmpty()
    if (uriText.isEmpty()) {
      invoke.resolve("")
      return
    }
    val uri = Uri.parse(uriText)
    var name = uri.lastPathSegment
    try {
      val cursor = activity.contentResolver.query(
        uri,
        arrayOf(OpenableColumns.DISPLAY_NAME),
        null,
        null,
        null
      )
      cursor?.use {
        if (it.moveToFirst()) {
          val idx = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
          if (idx >= 0) name = it.getString(idx)
        }
      }
    } catch (_: Exception) {
      // fallback to lastPathSegment
    }
    invoke.resolve(name ?: "")
  }
}