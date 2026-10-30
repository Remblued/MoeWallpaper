# 测 Electron 基线还缺的两个数：冷启动、空闲私有内存。
# 口径必须和 docs/BENCHMARK.md §2.3 / §2.4 完全一致，否则两个数字不能放在同一张表里。
#
# 用法（普通桌面会话里跑，它会启动 GUI 程序）：
#   cd D:\meowallpaper\Meowallpaper\bench\electron-baseline
#   powershell -ExecutionPolicy Bypass -File .\measure.ps1
#
# 前提：已经打包过（dist\win-unpacked\BaselineElectron.exe 存在）。

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root 'dist\win-unpacked\BaselineElectron.exe'
if (-not (Test-Path $exe)) {
    throw "找不到 $exe —— 先跑 npx electron-builder --win nsis"
}

function Stop-Baseline {
    # 先礼后兵：优雅关窗，压不下去再 Kill。冷启动要求进程彻底消失，
    # 所以最后必须等到 Get-Process 真的查不到。
    Get-Process -Name 'BaselineElectron' -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_.CloseMainWindow() | Out-Null } catch { }
    }
    Start-Sleep -Milliseconds 1500
    Get-Process -Name 'BaselineElectron' -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_.Kill() } catch { }
    }
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Process -Name 'BaselineElectron' -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {
        Start-Sleep -Milliseconds 200
    }
}

# ---------- 冷启动 ----------
$times = @()
for ($i = 1; $i -le 10; $i++) {
    Stop-Baseline
    Start-Sleep -Milliseconds 1000

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $exe -PassThru
    $sawWindow = $false
    while ($sw.Elapsed.TotalSeconds -lt 30) {
        $p.Refresh()
        if ($p.HasExited) { break }
        if ($p.MainWindowHandle -ne 0) { $sawWindow = $true; break }
        Start-Sleep -Milliseconds 10
    }
    $ms = [int]$sw.Elapsed.TotalMilliseconds

    if ($sawWindow) {
        $times += $ms
        Write-Host ("run {0,2}: {1,6} ms" -f $i, $ms)
    } else {
        Write-Host ("run {0,2}: {1,6} ms  窗口没出现或进程提前退出，这一发作废" -f $i, $ms)
    }
}

if ($times.Count -lt 5) {
    throw "只有 $($times.Count) 次拿到了窗口，样本不够，先查基线程序本身"
}

$s = $times | Sort-Object
$n = $s.Count
if ($n % 2 -eq 0) {
    $median = ($s[$n / 2 - 1] + $s[$n / 2]) / 2
} else {
    $median = $s[[int][math]::Floor($n / 2)]
}

# ---------- 空闲内存 ----------
Stop-Baseline
Start-Process -FilePath $exe | Out-Null
Start-Sleep -Seconds 60

$procs = @(Get-Process -Name 'BaselineElectron' -ErrorAction SilentlyContinue)
if ($procs.Count -eq 0) {
    throw "启动 60 秒后进程已经没了——应用自己退出了，内存测不到（这就是 main.js 那个引用 bug 的症状）"
}

$priv = ($procs | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB
$ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB

Stop-Baseline

Write-Host ''
Write-Host '=== 抄回 BENCHMARK.md 的数字 ==='
Write-Host ("冷启动  原始值: {0}" -f ($times -join ', '))
Write-Host ("         min={0}ms  median={1}ms  max={2}ms" -f $s[0], $median, $s[$n - 1])
Write-Host ("空闲内存 {0} 个进程  私有 {1:N1} MB   工作集 {2:N1} MB" -f $procs.Count, $priv, $ws)
