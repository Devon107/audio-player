use std::path::Path;

use tauri::{AppHandle, State};

use crate::audio::output::AudioCommand;
use crate::audio::queue::QueueTrackInput;
use crate::audio::AudioEngineHandle;
use crate::db::DbHandle;

use super::m3u;
use super::models::{PlaylistRecord, PlaylistTrackRecord};
use super::queries;

#[tauri::command]
pub fn create_playlist(db: State<DbHandle>, name: String) -> Result<i64, String> {
    queries::create_playlist(&db.lock(), &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_playlist(db: State<DbHandle>, playlist_id: i64, name: String) -> Result<(), String> {
    queries::rename_playlist(&db.lock(), playlist_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_playlist(db: State<DbHandle>, playlist_id: i64) -> Result<(), String> {
    queries::delete_playlist(&db.lock(), playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_playlists(db: State<DbHandle>) -> Result<Vec<PlaylistRecord>, String> {
    queries::list_playlists(&db.lock()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_playlist_tracks(
    db: State<DbHandle>,
    playlist_id: i64,
) -> Result<Vec<PlaylistTrackRecord>, String> {
    queries::list_playlist_tracks(&db.lock(), playlist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tracks_to_playlist(
    db: State<DbHandle>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), String> {
    queries::add_tracks(&db.lock(), playlist_id, &track_ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_track_from_playlist(
    db: State<DbHandle>,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), String> {
    queries::remove_track(&db.lock(), playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_playlist_track(
    db: State<DbHandle>,
    playlist_id: i64,
    track_id: i64,
    new_index: usize,
) -> Result<(), String> {
    queries::reorder_track(&mut db.lock(), playlist_id, track_id, new_index)
        .map_err(|e| e.to_string())
}

/// Carga la playlist completa en la cola de reproducción (ver Fase 4) y, opcionalmente, arranca
/// la reproducción desde una posición.
#[tauri::command]
pub fn play_playlist(
    db: State<DbHandle>,
    audio: State<AudioEngineHandle>,
    playlist_id: i64,
    start_index: Option<usize>,
    autoplay: bool,
) -> Result<(), String> {
    let tracks =
        queries::list_playlist_tracks(&db.lock(), playlist_id).map_err(|e| e.to_string())?;

    let items = tracks
        .into_iter()
        .map(|t| QueueTrackInput {
            path: t.path,
            track_id: Some(t.track_id),
        })
        .collect();

    audio.send(AudioCommand::SetQueue {
        items,
        start_index,
        autoplay,
    })
}

#[tauri::command]
pub fn export_playlist_m3u(
    db: State<DbHandle>,
    playlist_id: i64,
    target_path: String,
) -> Result<(), String> {
    m3u::export_m3u(&db.lock(), playlist_id, Path::new(&target_path))
}

#[tauri::command]
pub fn import_playlist_m3u(
    app: AppHandle,
    db: State<DbHandle>,
    source_path: String,
    playlist_name: Option<String>,
) -> Result<i64, String> {
    m3u::import_m3u(&db, &app, Path::new(&source_path), playlist_name)
}
