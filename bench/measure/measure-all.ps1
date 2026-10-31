# Measure MoeWallpaper cold start and idle memory.
# Follows docs/BENCHMARK.md:2.3 (CreateProcess only, no ShellExecuteEx) and 2.4
# (60 s idle, every process of the app's tree, not just the main process).
#
# Why the process tree instead of a command-line filter: this machine currently
# runs six unrelated msedgewebview2.exe processes owned by SearchHost.exe, so
# "all msedgewebview2 processes" would overcount. Parent-pid recursion cannot.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Runs = 10,
    [int]$Samples = 3,
    [int]$IdleSeconds = 60,
    [string]$OutDir = 'D:\bench\out'
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if (-not (Test-Path -LiteralPath $ExePath)) { throw "exe not found: $ExePath" }

function Get-TreeProcesses {
    param([int]$RootPid)
    $all = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue
    $byParent = @{}
    foreach ($p in $all) {
        if (-not $byParent.ContainsKey([int]$p.ParentProcessId)) {
            $byParent[[int]$p.ParentProcessId] = New-Object System.Collections.ArrayList
        }
        [void]$byParent[[int]$p.ParentProcessId].Add($p)
    }
    $result = New-Object System.Collections.ArrayList
    $queue = New-Object System.Collections.Queue
    $queue.Enqueue($RootPid)
    while ($queue.Count -gt 0) {
        $id = [int]$queue.Dequeue()
        if ($byParent.ContainsKey($id)) {
            foreach ($child in $byParent[$id]) {
                [void]$result.Add($child)
                $queue.Enqueue([int]$child.ProcessId)
            }
        }
    }
    return , $result
}

function Wait-NoAppRunning {
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline) {
        $left = @(Get-Process -Name 'moe-wallpaper' -ErrorAction SilentlyContinue)
        if ($left.Count -eq 0) { return $true }
        Start-Sleep -Milliseconds 150
    }
    return $false
}

if (-not (Wait-NoAppRunning)) { throw "a moe-wallpaper.exe process is already running; close it before measuring" }

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = (Resolve-Path -LiteralPath $ExePath).Path
$psi.UseShellExecute = $false          # CreateProcess, not ShellExecuteEx (docs/BENCHMARK.md 2.3)
$psi.WorkingDirectory = (Split-Path -Parent $psi.FileName)

Write-Output "=== cold start: $Runs launches, to MainWindowHandle != 0 ==="
$times = New-Object System.Collections.ArrayList
for ($i = 1; $i -le $Runs; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $limit = (Get-Date).AddSeconds(30)
    $ok = $false
    while ((Get-Date) -lt $limit) {
        if ($p.HasExited) { break }
        $p.Refresh()
        if ($p.MainWindowHandle -ne [IntPtr]::Zero) { $ok = $true; break }
        Start-Sleep -Milliseconds 5
    }
    $sw.Stop()
    if (-not $ok) {
        throw ("launch {0}: no main window within 30 s (HasExited={1}, ExitCode={2})" -f $i, $p.HasExited, $(if ($p.HasExited) { $p.ExitCode } else { 'n/a' }))
    }
    [void]$times.Add([int]$sw.ElapsedMilliseconds)
    Write-Output ("  run {0,2}: {1,4} ms" -f $i, $sw.ElapsedMilliseconds)
    $p.CloseMainWindow() | Out-Null
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
    if (-not (Wait-NoAppRunning)) { throw "process tree did not exit after run $i" }
    Start-Sleep -Milliseconds 400
}

$sorted = $times | Sort-Object
$median = if ($Runs % 2 -eq 1) { $sorted[[int](($Runs - 1) / 2)] } else { ($sorted[$Runs / 2 - 1] + $sorted[$Runs / 2]) / 2 }
Write-Output ("summary  min={0}  median={1}  max={2}  runs=[{3}]" -f $sorted[0], $median, $sorted[$Runs - 1], ($times -join ', '))
$times | Set-Content -LiteralPath (Join-Path $OutDir 'coldstart-ms.txt')

Write-Output ""
Write-Output "=== idle memory: $Samples samples, ${IdleSeconds}s idle each, whole process tree ==="
$rows = New-Object System.Collections.ArrayList
for ($s = 1; $s -le $Samples; $s++) {
    $p = [System.Diagnostics.Process]::Start($psi)
    $limit = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $limit) {
        if ($p.HasExited) { throw "process exited during memory sample $s" }
        $p.Refresh()
        if ($p.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 10
    }
    $windowAt = Get-Date
    Start-Sleep -Seconds $IdleSeconds

    $detail = New-Object System.Collections.ArrayList
    $main = Get-Process -Id $p.Id
    [void]$detail.Add([pscustomobject]@{ Kind = 'app'; Pid = $main.Id; PrivateMB = $main.PrivateMemorySize64 / 1MB; WorkingMB = $main.WorkingSet64 / 1MB })
    foreach ($c in (Get-TreeProcesses -RootPid $p.Id)) {
        $proc = Get-Process -Id ([int]$c.ProcessId) -ErrorAction SilentlyContinue
        if ($null -eq $proc) { continue }
        [void]$detail.Add([pscustomobject]@{ Kind = 'child'; Pid = $proc.Id; PrivateMB = $proc.PrivateMemorySize64 / 1MB; WorkingMB = $proc.WorkingSet64 / 1MB })
    }
    $privateSum = ($detail | Measure-Object PrivateMB -Sum).Sum
    $workingSum = ($detail | Measure-Object WorkingMB -Sum).Sum
    [void]$rows.Add([pscustomobject]@{ Sample = $s; Processes = $detail.Count; PrivateMB = $privateSum; WorkingMB = $workingSum })

    Write-Output ("  sample {0}: {1} processes  private={2:N1} MB  working set={3:N1} MB" -f $s, $detail.Count, $privateSum, $workingSum)
    foreach ($d in ($detail | Sort-Object PrivateMB -Descending)) {
        Write-Output ("      pid {0,-6} {1}  private={2,7:N1} MB  working={3,7:N1} MB" -f $d.Pid, $d.Kind, $d.PrivateMB, $d.WorkingMB)
    }

    if ($s -eq 1) {
        try {
            Add-Type -AssemblyName System.Drawing
            Add-Type -Namespace Win32 -Name User -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
            $p.Refresh()
            $h = $p.MainWindowHandle
            $r = New-Object 'Win32.User+RECT'
            if ([Win32.User]::GetWindowRect($h, [ref]$r)) {
                $w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
                $bmp = New-Object System.Drawing.Bitmap $w, $hh
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $hh))
                $shot = Join-Path $OutDir 'window-idle.png'
                $bmp.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
                $g.Dispose(); $bmp.Dispose()
                Write-Output "  screenshot: $shot"
            }
        } catch {
            Write-Output ("  screenshot skipped: {0}" -f $_.Exception.Message)
        }
    }

    $p.CloseMainWindow() | Out-Null
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
    if (-not (Wait-NoAppRunning)) { throw "process tree did not exit after memory sample $s" }
    Start-Sleep -Milliseconds 500
}

$privSorted = $rows.PrivateMB | Sort-Object
$privMedian = if ($Samples % 2 -eq 1) { $privSorted[[int](($Samples - 1) / 2)] } else { ($privSorted[$Samples / 2 - 1] + $privSorted[$Samples / 2]) / 2 }
$workMedian = if ($Samples % 2 -eq 1) { ($rows.WorkingMB | Sort-Object)[[int](($Samples - 1) / 2)] } else { ((($rows.WorkingMB | Sort-Object)[$Samples / 2 - 1] + ($rows.WorkingMB | Sort-Object)[$Samples / 2]) / 2) }
Write-Output ("summary  private median={0:N1} MB  working set median={1:N1} MB  samples=[{2}]" -f $privMedian, $workMedian, (($rows | ForEach-Object { '{0:N1}' -f $_.PrivateMB }) -join ', '))
$rows | Export-Csv -LiteralPath (Join-Path $OutDir 'memory.csv') -NoTypeInformation -Encoding UTF8
