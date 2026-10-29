pub mod commands;
pub mod fs;
pub mod sources;
pub mod wallpaper;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_sources,
            commands::fetch_images,
            commands::set_wallpaper,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
