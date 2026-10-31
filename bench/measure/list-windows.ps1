# 列出某个进程的所有顶层窗口（句柄、标题、尺寸、是否可见）。
#
# 为什么需要：开了 `transparent: true` 之后，`Process.MainWindowHandle` 可能返回进程里
# **另一个**顶层窗口（实测两次分别拿到 158×26 和 15×15 的小窗），截出来完全不是界面。
# 所以截图脚本必须按"标题 + 面积"自己挑主窗口，不能信 MainWindowHandle。

param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [string]$TitleLike = ''
)

Add-Type -Namespace Win32 -Name Enum -MemberDefinition @'
public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, System.Text.StringBuilder text, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
'@

$rows = New-Object System.Collections.ArrayList
$callback = [Win32.Enum+EnumProc] {
    param($hWnd, $lParam)
    $pid_ = 0
    [void][Win32.Enum]::GetWindowThreadProcessId($hWnd, [ref]$pid_)
    if ($pid_ -eq $ProcessId) {
        $sb = New-Object System.Text.StringBuilder 260
        [void][Win32.Enum]::GetWindowTextW($hWnd, $sb, 260)
        $title = $sb.ToString()
        $r = New-Object 'Win32.Enum+RECT'
        [void][Win32.Enum]::GetWindowRect($hWnd, [ref]$r)
        if ($TitleLike -eq '' -or $title -like "*$TitleLike*") {
            [void]$rows.Add([pscustomobject]@{
                    Handle  = $hWnd
                    Title   = $title
                    Width   = $r.Right - $r.Left
                    Height  = $r.Bottom - $r.Top
                    Visible = [Win32.Enum]::IsWindowVisible($hWnd)
                })
        }
    }
    return $true
}
[void][Win32.Enum]::EnumWindows($callback, [IntPtr]::Zero)

$rows | Sort-Object { $_.Width * $_.Height } -Descending |
    Format-Table Handle, Width, Height, Visible, Title -AutoSize
