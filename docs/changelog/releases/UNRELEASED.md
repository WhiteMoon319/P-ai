# 未发布

## 修复

- 修复新建“隔离工作树”会话时 Git 根目录二次校验会弹出控制台窗口的问题：Windows 下以 `CREATE_NO_WINDOW` 执行校验进程。
- 修复后台子进程弹出控制台窗口的遗漏点：Git 幽灵快照、VSCode 桥接网络探测、winget 安装、WSL/Shell 终端启动器、默认程序打开文件，均以 `CREATE_NO_WINDOW` 抑制多余控制台窗口。

## 依赖

## 功能
