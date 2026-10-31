# 找"这个进程的界面窗口"，给其他测量脚本用（dot-source）。
#
# **为什么不能用 `Process.MainWindowHandle`**：窗口开 `transparent: true` 之后，进程里
# 多了一个 15×15 的辅助顶层窗口，`MainWindowHandle` 有时候给的就是它（实测拿到过
# 158×26 和 15×15）。拿它去截图，截到的不是界面；拿它去计时，"窗口出现"的时刻也不对。
#
# 规则：**可见 + 标题含 MoeWallpaper + 面积最大**。
# 面积最大这条是必要的：Windows 自己还会建 0×0 的 MSCTFIME UI / Default IME 窗口。

if (-not ('Win32.Win' -as [type])) {
    Add-Type -Namespace Win32 -Name Win -MemberDefinition @'
public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, System.Text.StringBuilder text, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
[DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@
}

# **第一次调用前必须声明 DPI 感知**：PowerShell 宿主默认 DPI 不感知，此时 `GetWindowRect`
# 返回"逻辑像素"（被系统缩放过的），而 `CopyFromScreen` 用物理像素。125% 缩放时两者差
# 110 px 左右——截出来窗口跑到右下角、左上角露出别的程序（实测踩过）。声明之后两边都是
# 物理像素，坐标才对得上。这句只对截图有用，但放在这里（统一入口）最不容易漏。
[void][Win32.Win]::SetProcessDPIAware()

function Get-AppWindow {
    param(
        [Parameter(Mandatory = $true)][int]$AppProcessId,
        [string]$TitleLike = 'MoeWallpaper'
    )

    $script:bestHandle = [IntPtr]::Zero
    $script:bestArea = -1

    $callback = [Win32.Win+EnumProc] {
        param($hWnd, $lParam)
        $owner = 0
        [void][Win32.Win]::GetWindowThreadProcessId($hWnd, [ref]$owner)
        if ($owner -eq $AppProcessId -and [Win32.Win]::IsWindowVisible($hWnd)) {
            $sb = New-Object System.Text.StringBuilder 260
            [void][Win32.Win]::GetWindowTextW($hWnd, $sb, 260)
            if ($sb.ToString() -like "*$TitleLike*") {
                $r = New-Object 'Win32.Win+RECT'
                if ([Win32.Win]::GetWindowRect($hWnd, [ref]$r)) {
                    $area = ($r.Right - $r.Left) * ($r.Bottom - $r.Top)
                    if ($area -gt $script:bestArea) {
                        $script:bestArea = $area
                        $script:bestHandle = $hWnd
                    }
                }
            }
        }
        return $true
    }
    [void][Win32.Win]::EnumWindows($callback, [IntPtr]::Zero)
    return $script:bestHandle
}

function Wait-AppWindow {
    param(
        [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds = 30,
        [int]$PollMs = 10,
        [string]$TitleLike = 'MoeWallpaper'
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if ($Process.HasExited) { return [IntPtr]::Zero }
        $h = Get-AppWindow -AppProcessId $Process.Id -TitleLike $TitleLike
        if ($h -ne [IntPtr]::Zero) { return $h }
        Start-Sleep -Milliseconds $PollMs
    }
    return [IntPtr]::Zero
}

# 关窗口：必须发给真正的那个句柄。
# `Process.CloseMainWindow()` 走的是 `MainWindowHandle`，也就是可能指到 15×15 辅助窗口的
# 那个值——那样窗口不关、进程不退出，后面的采样全被污染。
function Close-AppWindow {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)
    if ($Handle -eq [IntPtr]::Zero) { return $false }
    [void][Win32.Win]::PostMessageW($Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)   # WM_CLOSE
    return $true
}

function Get-WindowRect {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)
    $r = New-Object 'Win32.Win+RECT'
    if (-not [Win32.Win]::GetWindowRect($Handle, [ref]$r)) { return $null }
    return [pscustomobject]@{ Left = $r.Left; Top = $r.Top; Width = $r.Right - $r.Left; Height = $r.Bottom - $r.Top }
}
