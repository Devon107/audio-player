use std::path::PathBuf;

use tauri::AppHandle;

use super::cover_cache;
use super::reader::{self, TrackMetadata};

#[tauri::command]
pub fn read_track_metadata(path: String) -> Result<TrackMetadata, String> {
    reader::read_metadata(&PathBuf::from(path))
}

/// Devuelve la ruta absoluta en disco de la carátula cacheada, o `None` si la pista no tiene
/// ninguna embebida. El frontend debe convertir la ruta con `convertFileSrc` antes de usarla
/// como `src` de una imagen.
#[tauri::command]
pub fn get_cover_art(app: AppHandle, path: String) -> Result<Option<String>, String> {
    let cover_path = cover_cache::get_or_extract_cover(&app, &PathBuf::from(path))?;
    Ok(cover_path.map(|p| p.to_string_lossy().into_owned()))
}
