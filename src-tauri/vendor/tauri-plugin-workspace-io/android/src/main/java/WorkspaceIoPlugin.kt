// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.workspace_io

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
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

@InvokeArg
class ShareFileArgs {
  var path: String? = null
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
      invoke.resolveObject("")
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
    invoke.resolveObject(name ?: "")
  }

  /**
   * 通过系统分享面板导出沙盒工作区文件。
   *
   * WebView 里的 `navigator.share` 在 wry Android WebView 中不可用（前端已证
   * `share`/`canShare` 均为 false），改用原生 ACTION_SEND + FileProvider
   * 唤起系统分享面板，绕开 base64 与 Web Share API。
   */
  @Command
  fun shareFromDevice(invoke: Invoke) {
    val args = invoke.parseArgs(ShareFileArgs::class.java)
    val pathText = args.path?.trim().orEmpty()
    if (pathText.isEmpty()) {
      invoke.reject("缺少文件路径")
      return
    }
    val file = File(pathText)
    if (!file.isFile) {
      invoke.reject("文件不存在: $pathText")
      return
    }
    try {
      val uri = FileProvider.getUriForFile(
        activity,
        "${activity.packageName}.workspaceio.fileprovider",
        file
      )
      val mime = activity.contentResolver.getType(uri) ?: "application/octet-stream"
      val share = Intent(Intent.ACTION_SEND).apply {
        type = mime
        putExtra(Intent.EXTRA_STREAM, uri)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      }
      val chooser = Intent.createChooser(share, "分享文件").apply {
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      }
      activity.startActivity(chooser)
      invoke.resolveObject("")
    } catch (e: Exception) {
      invoke.reject("分享失败: ${e.message}")
    }
  }
}