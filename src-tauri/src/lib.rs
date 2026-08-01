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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(audio::AudioEngineHandle::spawn(app.handle().clone()));

            let db_handle = db::init(app.handle())?;

            let watched_folders: Vec<std::path::PathBuf> = {
                let conn = db_handle.lock();
                let mut stmt = conn.prepare("SELECT path FROM watched_folders")?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect()
            };

            let watcher_handle = library::LibraryWatcherHandle::spawn(
                app.handle().clone(),
                db_handle.clone(),
                watched_folders,
            );

            app.manage(db_handle);
            app.manage(watcher_handle);

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
            audio::commands::set_queue,
            audio::commands::add_to_queue,
            audio::commands::remove_from_queue,
            audio::commands::reorder_queue,
            audio::commands::clear_queue,
            audio::commands::play_queue_item,
            audio::commands::next_track,
            audio::commands::previous_track,
            audio::commands::set_shuffle,
            audio::commands::set_repeat_mode,
            audio::commands::get_queue_state,
            metadata::commands::read_track_metadata,
            metadata::commands::get_cover_art,
            library::commands::pick_and_add_folder,
            library::commands::list_watched_folders,
            library::commands::remove_watched_folder,
            library::commands::rescan_library,
            library::commands::list_tracks,
            library::commands::list_artists,
            library::commands::list_albums,
            library::commands::list_genres,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
