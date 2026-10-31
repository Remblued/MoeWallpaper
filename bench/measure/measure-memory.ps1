# Second idle-memory pass: labels each process with its WebView2 role (from the
# command line) so the record can say which process owns the memory, and reports
# private bytes + private working set (perf counter) + working set.
#
# Note: these WebView2 processes belong to MoeWallpaper only because they are
# descendants of the app process. This machine also runs six unrelated
# msedgewebview2.exe processes (SearchHost.exe), so process-name matching would
# overcount. docs/BENCHMARK.md 2.4 has a command-line filter for that; the tree
# walk is stricter.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Samples = 3,
    [int]$IdleSeconds = 60
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib-window.ps1')
if (-not (Test-Path -LiteralPath $ExePath)) { throw "exe not found: $ExePath" }
$exe = (Resolve-Path -LiteralPath $ExePath).Path

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

function Get-Role {
    param([string]$CommandLine)
    if (-not $CommandLine) { return 'unknown' }
    if ($CommandLine -match '--type=([a-z-]+)') {
        $t = $Matches[1]
        if ($t -eq 'utility' -and $CommandLine -match '--utility-sub-type=([^ ]+)') { return 'utility:' + ($Matches[1] -replace '.*\.', '') }
        return $t
    }
    return 'browser'
}

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = (Split-Path -Parent $exe)

foreach ($s in 1..$Samples) {
    $p = [System.Diagnostics.Process]::Start($psi)
    $h = Wait-AppWindow -Process $p -TimeoutSeconds 30 -PollMs 10
    if ($h -eq [IntPtr]::Zero) { throw "sample ${s}: 窗口没出现（exited=$($p.HasExited)）" }
    Start-Sleep -Seconds $IdleSeconds

    $rows = New-Object System.Collections.ArrayList
    $proc = Get-Process -Id $p.Id
    $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)").CommandLine
    [void]$rows.Add([pscustomobject]@{ Pid = $proc.Id; Role = 'app'; PrivateBytes = $proc.PrivateMemorySize64; WorkingSet = $proc.WorkingSet64 })
    foreach ($c in (Get-TreeProcesses -RootPid $p.Id)) {
        $cp = Get-Process -Id ([int]$c.ProcessId) -ErrorAction SilentlyContinue
        if ($null -eq $cp) { continue }
        [void]$rows.Add([pscustomobject]@{ Pid = $cp.Id; Role = (Get-Role -CommandLine $c.CommandLine); PrivateBytes = $cp.PrivateMemorySize64; WorkingSet = $cp.WorkingSet64 })
    }

    "=== sample {0} ({1} processes) ===" -f $s, $rows.Count
    foreach ($r in ($rows | Sort-Object PrivateBytes -Descending)) {
        "  {0,-18} pid {1,-6} private={2,7:N1} MB  working set={3,7:N1} MB" -f $r.Role, $r.Pid, ($r.PrivateBytes / 1MB), ($r.WorkingSet / 1MB)
    }
    "  TOTAL              private={0,7:N1} MB  working set={1,7:N1} MB" -f (($rows | Measure-Object PrivateBytes -Sum).Sum / 1MB), (($rows | Measure-Object WorkingSet -Sum).Sum / 1MB)
    "  grid state         (screenshot skipped in this pass)"

    [void](Close-AppWindow -Handle $h)
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline -and @(Get-Process -Name 'moe-wallpaper' -ErrorAction SilentlyContinue).Count -gt 0) { Start-Sleep -Milliseconds 150 }
    Start-Sleep -Milliseconds 500
}
