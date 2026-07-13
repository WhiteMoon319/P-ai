param(
  # 删除整个 target / target-tests（最干净，下次编译最慢）
  [switch]$All,
  # 跳过“应用正在运行”检查
  [switch]$Force,
  # 静默模式：无可清内容时不报错
  [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetRoots = @(
  (Join-Path $repoRoot "src-tauri\target"),
  (Join-Path $repoRoot "src-tauri\target-tests")
)

function Write-Info([string]$Message) {
  if (-not $Quiet) {
    Write-Host $Message
  }
}

function Get-RunningAppProcesses {
  # 开发态 / 安装态常见进程名
  Get-Process -ErrorAction SilentlyContinue |
    Where-Object {
      $_.ProcessName -in @("p-ai", "P-ai", "easy-call-ai")
    }
}

if (-not $Force) {
  $running = @(Get-RunningAppProcesses)
  if ($running.Count -gt 0) {
    $names = ($running | ForEach-Object { $_.ProcessName } | Select-Object -Unique) -join ", "
    throw "检测到应用进程仍在运行（$names）。请先退出应用，或使用 -Force 强制清理（可能失败）。"
  }
}

$removed = 0
$failed = 0

foreach ($root in $targetRoots) {
  if (-not (Test-Path -LiteralPath $root)) {
    Write-Info "[clean-cargo-cache] 跳过不存在目录: $root"
    continue
  }

  if ($All) {
    Write-Info "[clean-cargo-cache] 删除整个目录: $root"
    try {
      Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction Stop
      $removed += 1
    } catch {
      $failed += 1
      Write-Warning "[clean-cargo-cache] 删除失败: $root ; $_"
    }
    continue
  }

  $incrementalDirs = @(
    Get-ChildItem -LiteralPath $root -Recurse -Directory -Filter "incremental" -ErrorAction SilentlyContinue
  )
  if ($incrementalDirs.Count -eq 0) {
    Write-Info "[clean-cargo-cache] 无 incremental 缓存: $root"
    continue
  }

  foreach ($dir in $incrementalDirs) {
    Write-Info "[clean-cargo-cache] 删除: $($dir.FullName)"
    try {
      Remove-Item -LiteralPath $dir.FullName -Recurse -Force -ErrorAction Stop
      $removed += 1
    } catch {
      $failed += 1
      Write-Warning "[clean-cargo-cache] 删除失败: $($dir.FullName) ; $_"
    }
  }
}

if ($failed -gt 0) {
  throw "[clean-cargo-cache] 完成，但有 $failed 项失败；成功 $removed 项。"
}

Write-Info "[clean-cargo-cache] 完成。已清理 $removed 项。模式=$(if ($All) { 'all' } else { 'incremental' })"
