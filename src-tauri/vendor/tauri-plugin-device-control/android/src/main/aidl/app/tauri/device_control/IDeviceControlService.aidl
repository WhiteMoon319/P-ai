// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

package app.tauri.device_control;

/**
 * 运行在 Shizuku shell 身份进程中的提权服务契约。
 *
 * Shizuku 强制 UserService 模式（Shizuku#newProcess 已移除），服务端类
 * 由 Shizuku 以 shell 身份启动在独立进程，本接口为其调用入口。
 */
interface IDeviceControlService {

    // Shizuku server 约定的 destroy transaction code：服务端收到后应清理并退出进程
    oneway void destroy() = 16777114;

    // 执行提权命令，返回 JSON：{"exitCode":N,"stdout":"...","stderr":"..."}
    String execute(String command, long timeoutMs) = 1;

    // 注入式触控（MAA-Meow InputControlUtils 语义，UserService 进程内反射 injectInputEvent，
    // 事件序列在服务端 Binder 线程执行，避免占用应用进程 UI 线程）
    boolean tap(int x, int y) = 2;
    boolean swipe(int x1, int y1, int x2, int y2, long durationMs) = 3;
    boolean key(int keycode) = 4;
}