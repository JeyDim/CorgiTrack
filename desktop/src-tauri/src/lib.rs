#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // store — локальное хранение base URL / токена / выбранной семьи.
        .plugin(tauri_plugin_store::Builder::new().build())
        // http — drop-in fetch через Rust-стек (минует CORS/CSP/mixed-content).
        .plugin(tauri_plugin_http::init())
        // dialog + fs — сохранение CSV-отчёта.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // opener — открыть ссылку календаря во внешнем приложении.
        .plugin(tauri_plugin_opener::init())
        // updater + process — авто-обновление с GitHub Releases и перезапуск после установки.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
