use std::ffi::c_void;
use std::path::Path;

#[link(name = "user32")]
unsafe extern "system" {
    fn SystemParametersInfoW(
        ui_action: u32,        // UINT
        ui_param: u32,         // UINT
        pv_param: *mut c_void, // PVOID
        f_win_ini: u32,        // UINT
    ) -> i32; // BOOL
}

const SPI_SETDESKWALLPAPER: u32 = 0x0014;
const SPIF_UPDATEINIFILE: u32 = 0x01;
const SPIF_SENDCHANGE: u32 = 0x02;

/// 把 Rust 字符串转成 Win32 要的宽字符串。
pub fn to_wide(s: &str) -> Vec<u16> {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    buf.push(0);
    buf
}

//UNC 路径不管!
pub fn set_wallpaper(path: &Path) -> Result<(), String> {
    // 1. 绝对路径
    let abs = path
        .canonicalize()
        .map_err(|e| format!("无法解析壁纸路径 {}: {}", path.display(), e))?;

    // canonicalize 在 Windows 上会加 \\?\ 前缀，某些旧接口不认,小改一下
    let abs_str = abs.to_string_lossy();
    let abs_str = abs_str.strip_prefix(r"\\?\").unwrap_or(abs_str.as_ref());

    // 2. 转宽字符 —— 必须绑定到局部变量！
    let wide = to_wide(abs_str);

    // 3 + 4. 取指针、调用
    let ret = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr().cast_mut().cast(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };

    // 5. BOOL：0 就是失败，而且不告诉原因
    if ret == 0 {
        return Err(format!("SystemParametersInfoW 设置壁纸失败: {}", abs_str));
    }
    Ok(())
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

    #[test]
    #[ignore = "会真的修改桌面，只能手动跑"]
    fn sets_real_wallpaper() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/128x128.png");
        assert!(p.exists(), "图标不存在: {}", p.display());

        set_wallpaper(&p).expect("设置壁纸应该能成功");
    }
}
