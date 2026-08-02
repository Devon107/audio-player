use std::path::{Path, PathBuf};

use rusqlite::params;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::DbHandle;

use super::models::{
    AlbumRecord, ArtistRecord, GenreRecord, ScanSummary, TrackFilter, TrackRecord,
};
use super::watcher::LibraryWatcherHandle;
use super::{queries, scanner};

/// Abre el selector nativo de carpetas, y si el usuario elige una, la agrega a las carpetas
/// vigiladas y dispara un escaneo inicial en segundo plano.
///
/// Tiene que ser un comando `async`: `blocking_pick_folder()` bloquea el hilo actual esperando a
/// que el diálogo nativo (GTK en Linux) se muestre en el hilo principal y responda. Si este
/// comando corriera de forma sincrónica en el hilo principal, se produciría un interbloqueo o
/// crash — la documentación del plugin pide explícitamente usar `blocking_*` solo desde comandos
/// `async`, que Tauri despacha a un hilo del runtime en lugar del hilo principal.
#[tauri::command]
pub async fn pick_and_add_folder(
    app: AppHandle,
    db: State<'_, DbHandle>,
    watcher: State<'_, LibraryWatcherHandle>,
) -> Result<Option<String>, String> {
    let Some(file_path) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|e| format!("Ruta de carpeta inválida: {e}"))?;
    let path_str = path.to_string_lossy().into_owned();

    {
        let conn = db.lock();
        conn.execute(
            "INSERT INTO watched_folders (path) VALUES (?1) ON CONFLICT(path) DO NOTHING",
            params![path_str],
        )
        .map_err(|e| format!("No se pudo guardar la carpeta: {e}"))?;
    }

    watcher.add_folder(path);

    Ok(Some(path_str))
}

#[tauri::command]
pub async fn list_watched_folders(db: State<'_, DbHandle>) -> Result<Vec<String>, String> {
    queries::list_watched_folders(&db.lock()).map_err(|e| e.to_string())
}

/// Deja de vigilar una carpeta y elimina de la biblioteca las pistas que estaban bajo ella.
#[tauri::command]
pub async fn remove_watched_folder(
    db: State<'_, DbHandle>,
    watcher: State<'_, LibraryWatcherHandle>,
    path: String,
) -> Result<(), String> {
    {
        let conn = db.lock();
        conn.execute("DELETE FROM watched_folders WHERE path = ?1", params![path])
            .map_err(|e| format!("No se pudo quitar la carpeta: {e}"))?;
    }

    scanner::remove_tracks_under(&db, Path::new(&path));
    watcher.remove_folder(PathBuf::from(path));

    Ok(())
}

/// Vuelve a escanear todas las carpetas vigiladas. Los archivos sin cambios se saltan.
///
/// Tiene que ser `async`: recorrer y releer etiquetas de una biblioteca grande puede tardar
/// bastante, y los comandos Tauri no-`async` corren en el hilo principal — si este comando fuera
/// sincrónico, toda la ventana dejaría de responder (ni siquiera se podrían procesar clics)
/// durante todo el escaneo. Mismo motivo por el que `pick_and_add_folder` también es `async`.
#[tauri::command]
pub async fn rescan_library(
    app: AppHandle,
    db: State<'_, DbHandle>,
) -> Result<ScanSummary, String> {
    let folders = queries::list_watched_folders(&db.lock()).map_err(|e| e.to_string())?;

    let mut total = ScanSummary::default();
    for folder in folders {
        let summary = scanner::scan_folder(&db, &app, Path::new(&folder));
        total.scanned += summary.scanned;
        total.added += summary.added;
        total.updated += summary.updated;
        total.removed += summary.removed;
        total.errors += summary.errors;
    }

    Ok(total)
}

/// `async` por la misma razón que el resto de los comandos de este archivo: se invoca a
/// intervalos regulares mientras hay un escaneo en curso (para refrescar la tabla en vivo, ver
/// `library::scanner::scan_folder`), y si corriera en el hilo principal esas invocaciones
/// repetidas competirían con el propio escaneo y congelarían la ventana.
#[tauri::command]
pub async fn list_tracks(
    db: State<'_, DbHandle>,
    filter: TrackFilter,
) -> Result<Vec<TrackRecord>, String> {
    queries::list_tracks(&db.lock(), &filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_artists(db: State<'_, DbHandle>) -> Result<Vec<ArtistRecord>, String> {
    queries::list_artists(&db.lock()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_albums(
    db: State<'_, DbHandle>,
    artist_id: Option<i64>,
) -> Result<Vec<AlbumRecord>, String> {
    queries::list_albums(&db.lock(), artist_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_genres(db: State<'_, DbHandle>) -> Result<Vec<GenreRecord>, String> {
    queries::list_genres(&db.lock()).map_err(|e| e.to_string())
}
