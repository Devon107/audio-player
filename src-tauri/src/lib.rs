mod audio;
mod db;
mod library;
mod metadata;
mod playlist;
mod settings;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_handle = db::init(app.handle())?;

            // Barrido único al arrancar: limpia artistas/álbumes/géneros que hayan quedado
            // huérfanos (sin ninguna pista) de versiones anteriores a que existiera esta limpieza
            // automática en cada borrado. Ver `library::scanner::cleanup_orphaned_taxonomy`.
            library::scanner::cleanup_orphaned_taxonomy(&db_handle.lock());

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

            let eq_control = audio::EqualizerControl::new();
            if let Some(saved_gains) = db::settings::get(&db_handle.lock(), "eq_gains")?
                .and_then(|json| serde_json::from_str(&json).ok())
            {
                eq_control.set_gains(&saved_gains);
            }

            app.manage(audio::AudioEngineHandle::spawn(
                app.handle().clone(),
                eq_control.clone(),
                db_handle.clone(),
            ));
            app.manage(eq_control);
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
            audio::commands::set_eq_band_gain,
            audio::commands::set_eq_preset,
            audio::commands::get_eq_state,
            metadata::commands::read_track_metadata,
            metadata::commands::get_cover_art,
            library::commands::pick_and_add_folder,
            library::commands::list_watched_folders,
            library::commands::remove_watched_folder,
            library::commands::rescan_library,
            library::commands::list_tracks,
            library::commands::count_tracks,
            library::commands::list_artists,
            library::commands::list_albums,
            library::commands::list_genres,
            playlist::commands::create_playlist,
            playlist::commands::rename_playlist,
            playlist::commands::delete_playlist,
            playlist::commands::list_playlists,
            playlist::commands::list_playlist_tracks,
            playlist::commands::add_tracks_to_playlist,
            playlist::commands::remove_track_from_playlist,
            playlist::commands::reorder_playlist_track,
            playlist::commands::play_playlist,
            playlist::commands::export_playlist_m3u,
            playlist::commands::import_playlist_m3u,
            settings::commands::get_language_preference,
            settings::commands::set_language_preference,
            settings::commands::get_volume_preference,
            settings::commands::set_volume_preference,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
