# 截一张"真窗口在屏幕上长什么样"的图，用来验证 DWM 背景材质（Acrylic）。
#
# 为什么不能用 PrintWindow：PrintWindow 只渲染窗口**自己画**的东西，DWM 画在窗口后面的
# Acrylic 不在里面，截出来必然误判成"材质没生效"。所以这条必须抄屏幕（CopyFromScreen）。
#
# 为什么抄**整屏**而不是窗口那一块：要判断"桌面有没有透上来"，光看窗口内部没用，得同时
# 看到窗口背后的东西才能比。窗口先置顶（SetWindowPos HWND_TOPMOST），保证它前面没有
# 别的窗口挡着——注意置顶不等于拿到焦点，Windows 前台锁会让 SetForegroundWindow 静默
# 失败，所以只靠 z 序。
#
# 两个前提都由 lib-window.ps1 负责：
#   - 挑窗口要按"标题 + 面积"，不能用 MainWindowHandle（transparent 之后可能指到 15×15 的辅助窗）
#   - 进程要声明 DPI 感知，否则 GetWindowRect 和 CopyFromScreen 差一个缩放系数

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][ValidateSet('light', 'dark')][string]$Theme,
    [int]$SettleSeconds = 8,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib-window.ps1')
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$name = 'AppsUseLightTheme'

$saved = (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name
"before: AppsUseLightTheme = $saved"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -Namespace Win32 -Name Bk -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true)] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
'@

$HWND_TOPMOST = [IntPtr](-1)
$HWND_NOTOPMOST = [IntPtr](-2)
$SWP_NOMOVE = 0x0002
$SWP_NOSIZE = 0x0001
$flags = $SWP_NOMOVE -bor $SWP_NOSIZE

try {
    $want = if ($Theme -eq 'dark') { 0 } else { 1 }
    Set-ItemProperty -Path $key -Name $name -Value $want -Type DWord
    "during: AppsUseLightTheme = $want ($Theme)"

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.UseShellExecute = $false
    $psi.WorkingDirectory = (Split-Path -Parent $exe)
    $p = [System.Diagnostics.Process]::Start($psi)
    $h = Wait-AppWindow -Process $p -TimeoutSeconds 30 -PollMs 10
    if ($h -eq [IntPtr]::Zero) { throw '进程在窗口出现前就退出了' }

    if (-not [Win32.Bk]::SetWindowPos($h, $HWND_TOPMOST, 0, 0, 0, 0, $flags)) {
        'warn: SetWindowPos(TOPMOST) 失败，截图可能被别的窗口盖住'
    }
    Start-Sleep -Seconds $SettleSeconds

    $rect = Get-WindowRect -Handle $h
    "window rect: {0},{1} {2}x{3}" -f $rect.Left, $rect.Top, $rect.Width, $rect.Height

    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bounds.Size)
    $shot = Join-Path $OutDir ('backdrop-{0}.png' -f $Theme)
    $bmp.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    "screenshot: $shot  ({0}x{1} full screen)" -f $bounds.Width, $bounds.Height

    [void][Win32.Bk]::SetWindowPos($h, $HWND_NOTOPMOST, 0, 0, 0, 0, $flags)
    [void](Close-AppWindow -Handle $h)
    [void]$p.WaitForExit(15000)
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit() }
}
finally {
    if ($null -eq $saved) { Remove-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue }
    else { Set-ItemProperty -Path $key -Name $name -Value $saved -Type DWord }
    'after : AppsUseLightTheme = ' + (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name + ' (restored)'
}
