# One idle sample with the window pulled to the foreground, so the screenshot that
# goes with the memory record actually shows this app and not whatever was stacked
# on top of it. CopyFromScreen copies the screen, not the window: the first
# screenshots caught neighbouring windows, which makes them useless as evidence.
#
# -BlockNetwork points HTTP(S)_PROXY at a dead port so fetch_images fails and the
# grid stays empty (see measure-offline.ps1 for why that isolates image memory).

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [switch]$BlockNetwork,
    [int]$IdleSeconds = 60,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = 'Stop'
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

Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win32 -Name Fg -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = (Split-Path -Parent $exe)
if ($BlockNetwork) {
    $psi.EnvironmentVariables['HTTP_PROXY'] = 'http://127.0.0.1:1'
    $psi.EnvironmentVariables['HTTPS_PROXY'] = 'http://127.0.0.1:1'
    $psi.EnvironmentVariables['ALL_PROXY'] = 'http://127.0.0.1:1'
    "mode: network blocked (dead env proxy), expecting an empty/error grid"
} else {
    "mode: normal network"
}

$p = [System.Diagnostics.Process]::Start($psi)
$limit = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $limit) {
    if ($p.HasExited) { throw 'process exited before showing a window' }
    $p.Refresh()
    if ($p.MainWindowHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 10
}
$h = $p.MainWindowHandle
[void][Win32.Fg]::ShowWindow($h, 9)      # SW_RESTORE
[void][Win32.Fg]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 400
$fg = [Win32.Fg]::GetForegroundWindow()
"foreground == app window: " + ($fg -eq $h)

Start-Sleep -Seconds $IdleSeconds

$rows = New-Object System.Collections.ArrayList
$proc = Get-Process -Id $p.Id
[void]$rows.Add([pscustomobject]@{ Role = 'app'; Private = $proc.PrivateMemorySize64; Working = $proc.WorkingSet64 })
foreach ($c in (Get-TreeProcesses -RootPid $p.Id)) {
    $cp = Get-Process -Id ([int]$c.ProcessId) -ErrorAction SilentlyContinue
    if ($null -eq $cp) { continue }
    [void]$rows.Add([pscustomobject]@{ Role = (Get-Role -CommandLine $c.CommandLine); Private = $cp.PrivateMemorySize64; Working = $cp.WorkingSet64 })
}
foreach ($r in ($rows | Sort-Object Private -Descending)) {
    "  {0,-18} private={1,7:N1} MB  working set={2,7:N1} MB" -f $r.Role, ($r.Private / 1MB), ($r.Working / 1MB)
}
"  TOTAL              private={0,7:N1} MB  working set={1,7:N1} MB" -f (($rows | Measure-Object Private -Sum).Sum / 1MB), (($rows | Measure-Object Working -Sum).Sum / 1MB)

$p.Refresh()
$h = $p.MainWindowHandle
[void][Win32.Fg]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 500

$r = New-Object 'Win32.Fg+RECT'
if ([Win32.Fg]::GetWindowRect($h, [ref]$r)) {
    $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
    $shot = Join-Path $OutDir (('window-{0}.png') -f $(if ($BlockNetwork) { 'offline' } else { 'online' }))
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $ht))
    $bmp.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
    "screen capture : $shot  (foreground was app: $([Win32.Fg]::GetForegroundWindow() -eq $h))"
    # also try PrintWindow, which renders the window even if something is on top
    $bmp2 = New-Object System.Drawing.Bitmap $w, $ht
    $g2 = [System.Drawing.Graphics]::FromImage($bmp2)
    $hdc = $g2.GetHdc()
    $ok = [Win32.Fg]::PrintWindow($h, $hdc, 2)   # PW_RENDERFULLCONTENT
    $g2.ReleaseHdc($hdc)
    $shot2 = Join-Path $OutDir (('window-{0}-printwindow.png') -f $(if ($BlockNetwork) { 'offline' } else { 'online' }))
    $bmp2.Save($shot2, [System.Drawing.Imaging.ImageFormat]::Png)
    "printwindow    : $shot2  (ok=$ok)"
    $g.Dispose(); $bmp.Dispose(); $g2.Dispose(); $bmp2.Dispose()
}

$p.CloseMainWindow() | Out-Null
[void]$p.WaitForExit(15000)
if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
