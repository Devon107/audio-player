mod audio;
mod db;
mod library;
mod metadata;
mod playlist;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(audio::AudioEngineHandle::spawn(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            audio::commands::load_track,
            audio::commands::play,
            audio::commands::pause,
            audio::commands::stop,
            audio::commands::seek,
            audio::commands::set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
