//! 让窗口的背景真正由 DWM 画（Windows 11 的 Acrylic / Mica），而不是 CSS 假装。
//!
//! **为什么需要这个模块**：玻璃主题原来是"窗口内模糊"——`backdrop-filter` 模糊的是
//! 我们自己画的渐变，桌面上什么都没有透进来。要做出 TranslucentTB 那种观感，必须
//! 两件事一起做：
//!
//! 1. `tauri.conf.json` 里 `"transparent": true`（窗口本身不能有不透明底色）
//! 2. `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` 告诉 DWM 画哪种系统材质
//!
//! 只做第 1 步会得到一个"没模糊、直接透出桌面"的窗口，字会看不清；只做第 2 步什么
//! 都不会发生。所以这两个改动必须成对出现。
//!
//! **Windows 10 怎么办**：`DWMWA_SYSTEMBACKDROP_TYPE`（38）是 Win11 22H2 才有的。
//! 老系统上这个调用会返回失败，此处只打日志、不把它当致命错误——CSS 那边本来就留了
//! 半透明底色（alpha 0.3–0.5），没有系统模糊时仍然是"半透明但可读"，不会变成全透明
//! 裸窗口。**这是有意的降级，不是漏了判断。**

/// 让窗口使用系统的 Acrylic 背景（桌面/后面的窗口模糊透上来）。
///
/// 失败只返回 `Err` 文字，由调用方决定要不要当致命错误——见本模块文档末尾。
pub fn apply(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| format!("拿不到窗口句柄: {e}"))?.0;
    set_backdrop(hwnd, system_is_dark(window))
}

/// 系统的亮/暗。WebView2 跟随 Windows 应用主题，Tauri 的 `theme()` 拿的是同一个值。
fn system_is_dark(window: &tauri::WebviewWindow) -> bool {
    matches!(window.theme(), Ok(tauri::Theme::Dark))
}

/// 系统主题变了要跟着改标题栏的明暗，否则暗色桌面配白标题栏。
///
/// 句柄以 `isize` 形式进闭包：`*mut c_void` 不是 `Send`，而窗口事件回调要求 `Send`。
pub fn watch_theme(window: &tauri::WebviewWindow) {
    let hwnd = match window.hwnd() {
        Ok(h) => h.0 as isize,
        Err(e) => {
            eprintln!("监听主题变化失败（拿不到窗口句柄）: {e}");
            return;
        }
    };

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::ThemeChanged(theme) = event {
            let dark = *theme == tauri::Theme::Dark;
            if let Err(e) = set_backdrop(hwnd as *mut _, dark) {
                eprintln!("跟随系统主题失败: {e}");
            }
        }
    });
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    /// `DWMWA_USE_IMMERSIVE_DARK_MODE`：标题栏/边框用暗色。
    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    /// `DWMWA_SYSTEMBACKDROP_TYPE`：Win11 22H2+ 的系统背景材质。
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
    /// `DWMSBT_TRANSIENTWINDOW` = Acrylic：模糊"窗口后面的东西"。
    ///
    /// 选它而不是 Mica（2）的原因：Mica 只采样**桌面壁纸**，窗口叠在别的程序上时
    /// 不会跟着变；Acrylic 模糊的是窗口正后方的内容，才是"透上来"的观感。
    const DWMSBT_TRANSIENTWINDOW: i32 = 3;

    // `DwmSetWindowAttribute` 返回 HRESULT：0 是成功，负数失败。
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: *mut c_void,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    pub fn set(hwnd: *mut c_void, dark: bool) -> Result<(), String> {
        if hwnd.is_null() {
            return Err("窗口句柄是空指针".to_string());
        }

        let dark_flag: i32 = i32::from(dark);
        // SAFETY: 句柄来自 tauri 的真实窗口；`dark_flag` / `backdrop` 都是本栈上的 i32，
        // 生命周期覆盖整个调用；size 按 DWM 的约定传 sizeof(i32)。
        let dark_hr = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&raw const dark_flag).cast(),
                std::mem::size_of::<i32>() as u32,
            )
        };
        if dark_hr < 0 {
            return Err(format!(
                "设置标题栏明暗失败（HRESULT {dark_hr:#x}），老系统上这很正常"
            ));
        }

        let backdrop = DWMSBT_TRANSIENTWINDOW;
        // SAFETY: 同上，`backdrop` 是栈上的 i32，指针只在调用期间有效。
        let hr = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&raw const backdrop).cast(),
                std::mem::size_of::<i32>() as u32,
            )
        };
        if hr < 0 {
            return Err(format!(
                "DWM 系统背景材质不支持（HRESULT {hr:#x}）：Windows 10 或不支持 Acrylic 的版本，\
                 CSS 半透明会兜底"
            ));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn set(_hwnd: *mut std::ffi::c_void, _dark: bool) -> Result<(), String> {
        Err("只有 Windows 支持系统背景材质".to_string())
    }
}

fn set_backdrop(hwnd: *mut std::ffi::c_void, dark: bool) -> Result<(), String> {
    platform::set(hwnd, dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_handle_is_rejected_not_dereferenced() {
        let err = set_backdrop(std::ptr::null_mut(), false).unwrap_err();
        assert!(err.contains("空指针"), "错误要说清原因，实际: {err}");
    }

    // FFI 本身没有单测：`DwmSetWindowAttribute` 只有真的窗口句柄才有意义。
    // 效果验证靠截图（`bench/measure/measure-theme.ps1` 截亮/暗两套），
    // 失败路径靠上面那条空句柄测试保证"不会 deref 空指针、错误信息可读"。
}
