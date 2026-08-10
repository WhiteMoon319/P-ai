#!/usr/bin/env bash
# 安装 patchelf（MSYS2/Git Bash 环境）
if command -v pacman >/dev/null 2>&1; then
  echo "使用 pacman 安装 patchelf..."
  pacman -S --noconfirm mingw-w64-x86_64-patchelf 2>&1 | tail -5
elif command -v apt-get >/dev/null 2>&1; then
  echo "使用 apt 安装 patchelf..."
  sudo apt-get install -y -qq patchelf 2>&1 | tail -3
else
  echo "未找到 pacman/apt，请手动安装 patchelf" >&2
  exit 1
fi
command -v patchelf && patchelf --version
