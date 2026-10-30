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

$procName = 'BaselineElectron'

# ---------- 启动方式：不走 Shell ----------
# Start-Process 用的是 ShellExecuteEx。Windows Shell 会在启动前做一次"发布者验证"，
# 遇到未签名 / 带"来自 Internet"标记的文件就弹「打开文件 - 安全警告」。
# 那个弹窗不是程序弹的，程序甚至还没被创建；用户点一下，计时里就混进了人的反应时间。
#
# 用 UseShellExecute = $false 走 CreateProcess，直接创建进程：
#   - 不弹任何 Shell 安全确认
#   - 不加载 Shell 扩展，少一层和冷启动无关的开销
# 两个基线（本项目 / Electron）必须用同一种方式测，否则 §4 的"同一口径"就是空话。
function Start-Direct {
    param([string]$Path)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Path
    $psi.WorkingDirectory = Split-Path -Parent $Path
    $psi.UseShellExecute = $false
    [System.Diagnostics.Process]::Start($psi)
}

function Get-TopWindows {
    Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } |
        ForEach-Object { '{0}|{1}' -f $_.ProcessName, $_.MainWindowTitle }
}

function Stop-Baseline {
    # 先礼后兵：优雅关窗，压不下去再 Kill。冷启动要求进程彻底消失，
    # 所以最后必须等到 Get-Process 真的查不到。
    Get-Process -Name $procName -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_.CloseMainWindow() | Out-Null } catch { }
    }
    Start-Sleep -Milliseconds 1500
    Get-Process -Name $procName -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_.Kill() } catch { }
    }
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Process -Name $procName -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {
        Start-Sleep -Milliseconds 200
    }
}

# 万一还有别的窗口弹出来（安全软件、系统提示），把它揪出来并作废这一轮，
# 而不是默默把一个被污染的数字算进中位数。
$foreign = @{}

# ---------- 冷启动 ----------
$times = @()
for ($i = 1; $i -le 10; $i++) {
    Stop-Baseline
    Start-Sleep -Milliseconds 1000

    $before = @(Get-TopWindows)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Direct $exe
    $sawWindow = $false
    $reason = ''
    # 枚举全系统窗口本身要遍历进程表。每 10 ms 做一次就是给启动过程制造 CPU 争用，
    # 那是测量工具污染被测对象。每 500 ms 查一次足够抓到弹窗。
    $nextCheck = 0

    while ($sw.Elapsed.TotalSeconds -lt 30) {
        $p.Refresh()
        if ($p.HasExited) { $reason = "进程退出 ExitCode=$($p.ExitCode)"; break }
        if ($p.MainWindowHandle -ne 0) { $sawWindow = $true; break }

        if ($sw.Elapsed.TotalMilliseconds -ge $nextCheck) {
            $nextCheck = $sw.Elapsed.TotalMilliseconds + 500
            foreach ($w in @(Get-TopWindows)) {
                if ($before -notcontains $w) {
                    $foreign[$w] = 1
                    $reason = "出现了别的窗口：$w"
                }
            }
            if ($reason) { break }
        }

        Start-Sleep -Milliseconds 10
    }
    $ms = [int]$sw.Elapsed.TotalMilliseconds

    if ($sawWindow -and -not $reason) {
        $times += $ms
        Write-Host ("run {0,2}: {1,6} ms" -f $i, $ms)
    } else {
        Write-Host ("run {0,2}: {1,6} ms  {2} —— 这一发作废" -f $i, $ms, $reason)
    }
}

if ($foreign.Count) {
    Write-Host ''
    Write-Host '以下窗口在计时过程中出现，它们不属于基线程序：'
    $foreign.Keys | ForEach-Object { Write-Host "  $_" }
    Write-Host '（这些弹窗会把人的反应时间算进冷启动，必须消除后再测）'
    Write-Host ''
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
Start-Direct $exe | Out-Null
Start-Sleep -Seconds 60

$procs = @(Get-Process -Name $procName -ErrorAction SilentlyContinue)
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
