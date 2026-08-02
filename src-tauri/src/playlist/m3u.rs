use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Runtime};

use crate::db::DbHandle;
use crate::library::scanner;

use super::queries;

/// Escribe la playlist como M3U extendido (`#EXTM3U`). Las rutas se escriben absolutas, tal como
/// están guardadas en la biblioteca.
pub fn export_m3u(conn: &Connection, playlist_id: i64, target: &Path) -> Result<(), String> {
    let tracks = queries::list_playlist_tracks(conn, playlist_id).map_err(|e| e.to_string())?;

    let mut content = String::from("#EXTM3U\n");
    for track in &tracks {
        let label = match track.artist.as_deref() {
            Some(artist) if !artist.is_empty() => format!("{artist} - {}", track.title),
            _ => track.title.clone(),
        };
        content.push_str(&format!(
            "#EXTINF:{},{label}\n{}\n",
            track.duration_secs.round() as i64,
            track.path,
        ));
    }

    fs::write(target, content).map_err(|e| format!("No se pudo escribir el archivo M3U: {e}"))
}

/// Importa un archivo M3U como una nueva playlist. Las entradas que no estén ya en la biblioteca
/// se escanean y agregan automáticamente (si el archivo referenciado todavía existe en disco);
/// las que ya no existen se omiten en silencio.
pub fn import_m3u<R: Runtime>(
    db: &DbHandle,
    app: &AppHandle<R>,
    source: &Path,
    playlist_name: Option<String>,
) -> Result<i64, String> {
    let content = fs::read_to_string(source)
        .map_err(|e| format!("No se pudo leer '{}': {e}", source.display()))?;

    let base_dir = source.parent().map(Path::to_path_buf).unwrap_or_default();

    let track_paths: Vec<PathBuf> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let candidate = PathBuf::from(line);
            if candidate.is_absolute() {
                candidate
            } else {
                base_dir.join(candidate)
            }
        })
        .collect();

    let name = playlist_name.unwrap_or_else(|| {
        source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Playlist importada".to_string())
    });

    let playlist_id = {
        let conn = db.lock();
        queries::create_playlist(&conn, &name).map_err(|e| e.to_string())?
    };

    let mut track_ids = Vec::new();
    for path in track_paths {
        if !path.is_file() {
            continue;
        }

        let track_id = match find_track_id(db, &path).map_err(|e| e.to_string())? {
            Some(id) => Some(id),
            None => {
                scanner::upsert_track_file(db, app, &path)?;
                find_track_id(db, &path).map_err(|e| e.to_string())?
            }
        };

        if let Some(id) = track_id {
            track_ids.push(id);
        }
    }

    if !track_ids.is_empty() {
        let conn = db.lock();
        queries::add_tracks(&conn, playlist_id, &track_ids).map_err(|e| e.to_string())?;
    }

    Ok(playlist_id)
}

fn find_track_id(db: &DbHandle, path: &Path) -> rusqlite::Result<Option<i64>> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id FROM tracks WHERE path = ?1",
        params![path.to_string_lossy()],
        |row| row.get(0),
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::scanner;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")))
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "audio-player-m3u-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn export_then_import_roundtrip_preserves_order() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let app = tauri::test::mock_app();
        scanner::scan_folder(&db, app.handle(), &fixtures_dir());

        let original_ids: Vec<i64> = {
            let conn = db.lock();
            let mut stmt = conn.prepare("SELECT id FROM tracks ORDER BY id").unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };

        let source_playlist_id = {
            let conn = db.lock();
            let id = queries::create_playlist(&conn, "Para exportar").unwrap();
            queries::add_tracks(&conn, id, &original_ids).unwrap();
            id
        };

        let work_dir = temp_dir("roundtrip");
        let m3u_path = work_dir.join("playlist.m3u");
        {
            let conn = db.lock();
            export_m3u(&conn, source_playlist_id, &m3u_path).unwrap();
        }

        let content = std::fs::read_to_string(&m3u_path).unwrap();
        assert!(content.starts_with("#EXTM3U\n"));
        assert!(content.contains("#EXTINF:"));

        let imported_id =
            import_m3u(&db, app.handle(), &m3u_path, Some("Importada".to_string())).unwrap();

        let conn = db.lock();
        let imported_tracks = queries::list_playlist_tracks(&conn, imported_id).unwrap();
        let imported_ids: Vec<i64> = imported_tracks.iter().map(|t| t.track_id).collect();
        assert_eq!(
            imported_ids, original_ids,
            "la playlist importada debe referenciar las mismas pistas ya existentes, en el mismo orden"
        );

        let track_count_in_library: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            track_count_in_library, 3,
            "no debería haber creado pistas nuevas: ya estaban en la biblioteca"
        );

        let _ = std::fs::remove_dir_all(&work_dir);
    }

    #[test]
    fn import_scans_and_adds_tracks_missing_from_the_library() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let app = tauri::test::mock_app();
        // Base de datos vacía a propósito: ninguna pista está todavía en la biblioteca.

        let work_dir = temp_dir("missing-tracks");
        let audio_path = work_dir.join("cancion.mp3");
        std::fs::copy(fixtures_dir().join("test-tone.mp3"), &audio_path).unwrap();

        let m3u_path = work_dir.join("playlist.m3u");
        std::fs::write(&m3u_path, format!("#EXTM3U\n{}\n", audio_path.display())).unwrap();

        let playlist_id = import_m3u(&db, app.handle(), &m3u_path, None).unwrap();

        let conn = db.lock();
        let tracks = queries::list_playlist_tracks(&conn, playlist_id).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].path, audio_path.to_string_lossy());

        let track_count_in_library: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            track_count_in_library, 1,
            "debería haber escaneado y agregado la pista que faltaba en la biblioteca"
        );

        let _ = std::fs::remove_dir_all(&work_dir);
    }

    #[test]
    fn import_skips_entries_whose_file_no_longer_exists() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let app = tauri::test::mock_app();

        let work_dir = temp_dir("missing-file");
        let m3u_path = work_dir.join("playlist.m3u");
        std::fs::write(&m3u_path, "#EXTM3U\n/no/existe/cancion.mp3\n").unwrap();

        let playlist_id =
            import_m3u(&db, app.handle(), &m3u_path, Some("Rota".to_string())).unwrap();

        let conn = db.lock();
        let tracks = queries::list_playlist_tracks(&conn, playlist_id).unwrap();
        assert!(
            tracks.is_empty(),
            "las entradas rotas se omiten, no deberían fallar la importación"
        );

        let _ = std::fs::remove_dir_all(&work_dir);
    }
}
