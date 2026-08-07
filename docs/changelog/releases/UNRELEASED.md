# 未发布

## 功能

- Android 日志文件输出到外部存储 `/sdcard/Android/data/<包名>/log/backend.log`，可通过 adb 直接查看，不再写入需 root 权限的内部 data 目录。

## 重构

## 修复

- 修复 Android 启动 ClassNotFoundException：workspace-io 插件的 Android 插件标识符从连字符 `app.tauri.workspace-io` 改为下划线 `app.tauri.workspace_io`（与 Kotlin 源码包名一致），避免启动时反射找不到插件类而闪退。

## 依赖
