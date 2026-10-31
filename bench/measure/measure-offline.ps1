# Control run: same 60 s idle measurement, but the app cannot load any image.

# How: fetch.rs builds reqwest's client without an explicit proxy when the WinINET
# system proxy is off (it is off on this machine), so reqwest falls back to the
# HTTP_PROXY / HTTPS_PROXY environment variables. Pointing those at 127.0.0.1:1
# (nothing listens there) makes fetch_images fail for the child process only, so
# the window reaches its "error, no images" state while the WebView2 shell and the
# glass theme are still fully live.
#
# This deliberately does NOT touch the system proxy: that is machine-wide, and
# docs/BENCHMARK.md 1 requires two baselines to be measured under the same
# conditions, not "conditions I changed for one run".
#
# Why it matters: it separates "WebView2 shell + glass theme" from "decoded
# images" in the idle-memory number, without touching code or config.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Samples = 2,
    [int]$IdleSeconds = 60,
    [string]$OutDir = 'D:\bench\out-offline'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib-window.ps1')
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
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
    if ($CommandLine -match '--type=([a-z-]+)') { return $Matches[1] }
    return 'browser'
}

"system proxy (must be off for the env-var trick to apply): ProxyEnable=" +
    (Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -Name ProxyEnable -ErrorAction SilentlyContinue).ProxyEnable

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = (Split-Path -Parent $exe)
$psi.EnvironmentVariables['HTTP_PROXY'] = 'http://127.0.0.1:1'
$psi.EnvironmentVariables['HTTPS_PROXY'] = 'http://127.0.0.1:1'
$psi.EnvironmentVariables['ALL_PROXY'] = 'http://127.0.0.1:1'

foreach ($s in 1..$Samples) {
    $p = [System.Diagnostics.Process]::Start($psi)
    $h = Wait-AppWindow -Process $p -TimeoutSeconds 30 -PollMs 10
    if ($h -eq [IntPtr]::Zero) { throw "sample ${s}: window never appeared (exited=$($p.HasExited))" }
    Start-Sleep -Seconds $IdleSeconds

    $rows = New-Object System.Collections.ArrayList
    $proc = Get-Process -Id $p.Id
    [void]$rows.Add([pscustomobject]@{ Pid = $proc.Id; Role = 'app'; Private = $proc.PrivateMemorySize64; Working = $proc.WorkingSet64 })
    foreach ($c in (Get-TreeProcesses -RootPid $p.Id)) {
        $cp = Get-Process -Id ([int]$c.ProcessId) -ErrorAction SilentlyContinue
        if ($null -eq $cp) { continue }
        [void]$rows.Add([pscustomobject]@{ Pid = $cp.Id; Role = (Get-Role -CommandLine $c.CommandLine); Private = $cp.PrivateMemorySize64; Working = $cp.WorkingSet64 })
    }
    "=== sample {0} ({1} processes) ===" -f $s, $rows.Count
    foreach ($r in ($rows | Sort-Object Private -Descending)) {
        "  {0,-12} pid {1,-6} private={2,7:N1} MB  working set={3,7:N1} MB" -f $r.Role, $r.Pid, ($r.Private / 1MB), ($r.Working / 1MB)
    }
    "  TOTAL        private={0,7:N1} MB  working set={1,7:N1} MB" -f (($rows | Measure-Object Private -Sum).Sum / 1MB), (($rows | Measure-Object Working -Sum).Sum / 1MB)

    if ($s -eq 1) {
        try {
            Add-Type -AssemblyName System.Drawing
            Add-Type -Namespace Win32 -Name User3 -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
            $p.Refresh()
            $r = New-Object 'Win32.User3+RECT'
            if ([Win32.User3]::GetWindowRect($h, [ref]$r)) {
                $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
                $bmp = New-Object System.Drawing.Bitmap $w, $h
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
                $shot = Join-Path $OutDir ('window-offline-{0}.png' -f $s)
                $bmp.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
                $g.Dispose(); $bmp.Dispose()
                "  screenshot: $shot"
            }
        } catch { "  screenshot skipped: $($_.Exception.Message)" }
    }

    [void](Close-AppWindow -Handle $h)
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
    Start-Sleep -Milliseconds 600
}

