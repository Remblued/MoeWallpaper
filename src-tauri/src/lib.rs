pub mod commands;
pub mod fs;
pub mod sources;
pub mod wallpaper;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // 启动时就把缓存目录建好，别等第一次下载才现建。
            // 失败不致命——真正用到时会再试一次，并报出具体是哪个路径出错。
            if let Err(e) = fs::dirs::ensure_base_layout() {
                eprintln!("初始化缓存目录失败: {e}");
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
