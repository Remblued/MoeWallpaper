# Cold start, docs/BENCHMARK.md 2.3 verbatim口径:
#   - CreateProcess only (UseShellExecute = $false), never ShellExecuteEx
#   - stopwatch from Process.Start until **the titled window** shows up
#     (not MainWindowHandle: with a transparent window that can be a 15x15 helper)
#   - 10 launches, report the median
# Difference from the snippet in the doc: the poll interval is a parameter (10 ms
# by default, same as the doc) and the wait is bounded at 30 s, so a launch that
# never shows a window fails loudly instead of hanging.
#
# Also waits for the whole process tree to be gone between launches: a launch that
# starts while the previous WebView2 helpers still hold the user-data folder is a
# hot start, which would report a cold start that never happened.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Runs = 10,
    [int]$PollMs = 10,
    [string]$OutDir = 'D:\bench\out'
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
. (Join-Path $PSScriptRoot 'lib-window.ps1')
$exe = (Resolve-Path -LiteralPath $ExePath).Path

if (@(Get-Process -Name 'moe-wallpaper' -ErrorAction SilentlyContinue).Count -gt 0) {
    throw 'a moe-wallpaper.exe process is already running; close it before measuring'
}

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = (Split-Path -Parent $exe)

$times = New-Object System.Collections.ArrayList
for ($i = 1; $i -le $Runs; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    # 等"标题窗口"出现，而不是 MainWindowHandle —— 窗口开了 transparent 之后
    # 进程里多一个 15×15 的辅助窗，MainWindowHandle 有时指的就是它。
    $h = Wait-AppWindow -Process $p -TimeoutSeconds 30 -PollMs $PollMs
    $sw.Stop()
    if ($h -eq [IntPtr]::Zero) {
        throw ("launch {0}: 30 秒内没出现标题窗口（exited={1}）" -f $i, $p.HasExited)
    }
    [void]$times.Add([int]$sw.ElapsedMilliseconds)
    "  run {0,2}: {1,4} ms" -f $i, $sw.ElapsedMilliseconds

    [void](Close-AppWindow -Handle $h)
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline -and @(Get-Process -Name 'moe-wallpaper' -ErrorAction SilentlyContinue).Count -gt 0) {
        Start-Sleep -Milliseconds 150
    }
    Start-Sleep -Milliseconds $PollMs
}

$sorted = @($times | Sort-Object)
$median = if ($Runs % 2 -eq 1) { $sorted[[int](($Runs - 1) / 2)] } else { ($sorted[$Runs / 2 - 1] + $sorted[$Runs / 2]) / 2 }
"summary  min={0}  median={1}  max={2}  raw=[{3}]" -f $sorted[0], $median, $sorted[$sorted.Count - 1], ($times -join ', ')
$times | Set-Content -LiteralPath (Join-Path $OutDir 'coldstart-ms.txt')
