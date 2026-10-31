# Grab one window screenshot in a chosen Windows app theme.
#
# The glass theme follows `prefers-color-scheme`, i.e. the Windows app theme
# (HKCU ...\Themes\Personalize\AppsUseLightTheme). This script flips that value,
# starts the app, captures the window with PrintWindow (CopyFromScreen catches
# whatever is stacked on top), then puts the original value back — including when
# it throws.
#
# Run it for both themes into the same folder; the two PNGs are the visual
# verification the plan asks for (2026-10-31: the light theme was captured before
# this script existed, dark was never looked at).

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][ValidateSet('light', 'dark')][string]$Theme,
    [int]$SettleSeconds = 8,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$name = 'AppsUseLightTheme'

$saved = (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name
"before: AppsUseLightTheme = $saved"

Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win32 -Name Theme -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@

try {
    $want = if ($Theme -eq 'dark') { 0 } else { 1 }
    Set-ItemProperty -Path $key -Name $name -Value $want -Type DWord
    "during: AppsUseLightTheme = $want ($Theme)"

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.UseShellExecute = $false
    $psi.WorkingDirectory = (Split-Path -Parent $exe)
    $p = [System.Diagnostics.Process]::Start($psi)

    $limit = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $limit) {
        if ($p.HasExited) { throw 'process exited before showing a window' }
        $p.Refresh()
        if ($p.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 10
    }
    Start-Sleep -Seconds $SettleSeconds
    $p.Refresh()

    $h = $p.MainWindowHandle
    $r = New-Object 'Win32.Theme+RECT'
    if (-not [Win32.Theme]::GetWindowRect($h, [ref]$r)) { throw 'GetWindowRect failed' }
    $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [Win32.Theme]::PrintWindow($h, $hdc, 2)
    $g.ReleaseHdc($hdc)
    $shot = Join-Path $OutDir ("window-{0}.png" -f $Theme)
    $bmp.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    "printwindow ok=$ok  ->  $shot"

    $p.CloseMainWindow() | Out-Null
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
}
finally {
    if ($null -eq $saved) { Remove-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue }
    else { Set-ItemProperty -Path $key -Name $name -Value $saved -Type DWord }
    "after : AppsUseLightTheme = " + (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name + ' (restored)'
}
