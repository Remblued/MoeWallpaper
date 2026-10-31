pub mod backdrop;
pub mod commands;
pub mod fs;
pub mod net;
pub mod sources;
pub mod wallpaper;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 启动时就把缓存目录建好，别等第一次下载才现建。
            // 失败不致命——真正用到时会再试一次，并报出具体是哪个路径出错。
            if let Err(e) = fs::dirs::ensure_base_layout() {
                eprintln!("初始化缓存目录失败: {e}");
            }

            // 真·窗口背景材质（Acrylic）：玻璃主题靠它把桌面透上来。
            // 失败不致命——Windows 10 没有 DWMWA_SYSTEMBACKDROP_TYPE，CSS 半透明兜底。
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                if let Err(e) = backdrop::apply(&window) {
                    eprintln!("系统背景材质不可用，回退到 CSS 半透明: {e}");
                }
                backdrop::watch_theme(&window);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_sources,
            commands::fetch_images,
            commands::set_wallpaper,
            commands::save_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
