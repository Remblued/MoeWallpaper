//! 把一张本地图片设为桌面壁纸（Win32 部分）。

use std::path::Path;

/// 把 Rust 字符串转成 Win32 要的宽字符串。
///
/// Windows 的 NT 内核从 1993 年起就用 UTF-16 当原生编码，所有 `...W` 结尾的 API
/// 都吃 UTF-16；而 Rust 的 `String` 是 UTF-8，必须转。
///
/// 另外 Win32 的字符串**没有长度参数**，它靠「读到 0 就停」判断结尾。
/// 所以末尾那个 `0` 不是可选的——少一个 0，Windows 会继续往后读内存，
/// 一直到撞见某个 0 为止，轻则路径错乱，重则崩溃。
pub fn to_wide(s: &str) -> Vec<u16> {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    buf.push(0);
    buf
}

// ============================================================================
//  下面这个函数由你实现（A3 的核心）
// ============================================================================

/// 把本地图片文件设为桌面壁纸。
///
/// # 你要调的 Win32 函数
///
/// 只有一个：`SystemParametersInfoW`，在 `user32.dll` 里，C 签名是：
///
/// ```c
/// BOOL SystemParametersInfoW(
///     UINT  uiAction,   // 做什么  → SPI_SETDESKWALLPAPER = 0x0014
///     UINT  uiParam,    // 固定传 0
///     PVOID pvParam,    // 参数   → 指向图片绝对路径的 u16 数组
///     UINT  fWinIni     // 副作用 → SPIF_UPDATEINIFILE | SPIF_SENDCHANGE
/// );
/// ```
///
/// 返回值是 `BOOL`：成功 `1`，失败 `0`（不会告诉你失败原因，就是这么原始）。
///
/// # Rust 怎么声明它
///
/// 1. 用 `unsafe extern "system" { ... }` 包起来。
///    `unsafe` 是必须的：Rust 2024 要求所有外部函数声明都显式标注不安全。
///    `"system"` 指 Windows 的调用约定，写错会让栈错乱。
/// 2. 块上面加 `#[link(name = "user32")]`，告诉链接器去 user32 库找符号。
/// 3. 参数类型对应关系：
///    `UINT` → `u32`，`PVOID` → `*mut std::ffi::c_void`，`BOOL` → `i32`
/// 4. 函数名照抄 `SystemParametersInfoW`，**W 不能丢**，丢了会链接到 ANSI 版本
///
/// # 实现步骤
///
/// 1. 绝对路径：`path.canonicalize()`，失败要转成 `Err`——Windows 不认相对路径
/// 2. 转宽字符：`to_wide(&绝对路径.to_string_lossy())`
/// 3. 取指针：`wide.as_ptr()` 是 `*const u16`，要 `.cast()` 成 `*mut c_void`
/// 4. 调用放在 `unsafe { }` 里（编译器不会帮你检查外部函数的安全性）
/// 5. 判断返回值：`0` 就是失败，返回 `Err`
///
/// 提示：`canonicalize` 在 Windows 上会加 `\\?\` 前缀，多数情况没问题；
/// 如果发现壁纸设不上，把它去掉再试（用 `strip_prefix` 或者干脆不 canonicalize，
/// 改用 `std::path::absolute`，标准库 1.79 起可用）。
pub fn set_wallpaper(path: &Path) -> Result<(), String> {
    // TODO(你写)：按上面 5 步实现。
    // 现在这行只是为了让变量被"用到"，你写完后删掉它。
    let _ = path;
    todo!("set_wallpaper 由你实现")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_wide_ends_with_nul() {
        let w = to_wide("C:\\a.jpg");
        assert_eq!(w.last(), Some(&0));
        assert_eq!(w.len(), "C:\\a.jpg".len() + 1);
    }

    #[test]
    fn to_wide_handles_non_ascii() {
        // 用户名的目录可能是中文，必须能处理
        let w = to_wide("壁纸");
        assert_eq!(w.last(), Some(&0));
        assert_eq!(w.len(), 2 + 1);
    }
}
