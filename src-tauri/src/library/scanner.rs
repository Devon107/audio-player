use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Emitter, Runtime};
use walkdir::WalkDir;

use crate::db::DbHandle;
use crate::metadata::cover_cache;
use crate::metadata::reader::{self, TrackMetadata};

use super::models::ScanSummary;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "oga", "wav", "m4a", "mp4", "aac"];

/// Cada cuánto se emite `library://scan-progress` durante un escaneo largo. Emitir por archivo
/// saturaría el puente IPC en carpetas con miles de canciones; con este intervalo el frontend
/// recibe actualizaciones fluidas sin ahogarse en eventos.
const PROGRESS_EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(300);

pub fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(ext))
        })
}

/// Escanea recursivamente `root`, agregando/actualizando pistas en la base de datos y
/// eliminando las que ya no existen en disco. Es seguro llamarlo repetidamente: los archivos
/// cuyo tamaño y fecha de modificación no cambiaron se saltan sin releer sus etiquetas.
///
/// Cada pista se guarda en SQLite a medida que se procesa (no al final), así que mientras dura
/// el escaneo se emite `library://scan-progress` a intervalos con el conteo parcial — el
/// frontend lo usa para refrescar la tabla y mostrar canciones apareciendo en vivo, en vez de
/// quedarse sin novedades hasta que termine de recorrer toda la carpeta.
pub fn scan_folder<R: Runtime>(db: &DbHandle, app: &AppHandle<R>, root: &Path) -> ScanSummary {
    let mut summary = ScanSummary::default();
    let mut seen_paths = Vec::new();
    let mut last_emit = Instant::now();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || !is_supported_audio_file(entry.path()) {
            continue;
        }

        summary.scanned += 1;
        seen_paths.push(entry.path().to_path_buf());

        match upsert_track_file(db, app, entry.path()) {
            Ok(Some(true)) => summary.added += 1,
            Ok(Some(false)) => summary.updated += 1,
            Ok(None) => {}
            Err(_) => summary.errors += 1,
        }

        if last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL {
            let _ = app.emit("library://scan-progress", summary.clone());
            last_emit = Instant::now();
        }
    }

    summary.removed = remove_missing_tracks(db, root, &seen_paths);
    let _ = app.emit("library://scan-progress", summary.clone());
    summary
}

/// Analiza un único archivo y lo inserta/actualiza en la base de datos.
///
/// Devuelve `Ok(Some(true))` si la pista es nueva, `Ok(Some(false))` si se actualizó una
/// existente, y `Ok(None)` si no había cambios en disco y se saltó sin releer etiquetas.
pub fn upsert_track_file<R: Runtime>(
    db: &DbHandle,
    app: &AppHandle<R>,
    path: &Path,
) -> Result<Option<bool>, String> {
    let fs_meta = std::fs::metadata(path)
        .map_err(|e| format!("No se pudo leer '{}': {e}", path.display()))?;
    let file_size = fs_meta.len() as i64;
    let modified_at = fs_meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let path_str = path.to_string_lossy();

    {
        let conn = db.lock();
        if let Some((existing_size, existing_mtime)) =
            existing_file_stats(&conn, &path_str).map_err(|e| e.to_string())?
        {
            if existing_size == file_size && existing_mtime == modified_at {
                return Ok(None);
            }
        }
    }

    let track_meta = reader::read_metadata(path)?;
    let cover_path = cover_cache::get_or_extract_cover(app, path)?;

    let conn = db.lock();
    let inserted = upsert_track_row(
        &conn,
        path,
        &track_meta,
        cover_path.as_deref(),
        file_size,
        modified_at,
    )
    .map_err(|e| format!("No se pudo guardar la pista en la base de datos: {e}"))?;

    Ok(Some(inserted))
}

/// Elimina una pista de la base de datos por su ruta (usado por el watcher cuando un archivo se
/// borra). Devuelve `true` si existía.
pub fn remove_track_by_path(db: &DbHandle, path: &Path) -> Result<bool, String> {
    let conn = db.lock();
    let affected = conn
        .execute(
            "DELETE FROM tracks WHERE path = ?1",
            params![path.to_string_lossy()],
        )
        .map_err(|e| format!("No se pudo eliminar la pista: {e}"))?;
    Ok(affected > 0)
}

/// Elimina todas las pistas registradas bajo `root` (usado al dejar de vigilar una carpeta).
pub fn remove_tracks_under(db: &DbHandle, root: &Path) -> usize {
    remove_missing_tracks(db, root, &[])
}

fn existing_file_stats(conn: &Connection, path: &str) -> rusqlite::Result<Option<(i64, i64)>> {
    conn.query_row(
        "SELECT file_size, modified_at FROM tracks WHERE path = ?1",
        params![path],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

fn upsert_track_row(
    conn: &Connection,
    path: &Path,
    meta: &TrackMetadata,
    cover_path: Option<&Path>,
    file_size: i64,
    modified_at: i64,
) -> rusqlite::Result<bool> {
    let artist_id = meta
        .artist
        .as_deref()
        .map(|name| get_or_create_artist(conn, name))
        .transpose()?;
    let album_id = meta
        .album
        .as_deref()
        .map(|title| get_or_create_album(conn, title, artist_id))
        .transpose()?;
    let genre_id = meta
        .genre
        .as_deref()
        .map(|name| get_or_create_genre(conn, name))
        .transpose()?;

    let path_str = path.to_string_lossy();
    let cover_path_str = cover_path.map(|p| p.to_string_lossy().into_owned());
    let now = unix_now();

    let existed: bool = conn
        .query_row(
            "SELECT 1 FROM tracks WHERE path = ?1",
            params![path_str],
            |_| Ok(()),
        )
        .optional()?
        .is_some();

    conn.execute(
        "INSERT INTO tracks
            (path, title, artist_id, album_id, genre_id, year, track_number, duration_secs,
             cover_path, file_size, modified_at, added_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
         ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist_id = excluded.artist_id,
            album_id = excluded.album_id,
            genre_id = excluded.genre_id,
            year = excluded.year,
            track_number = excluded.track_number,
            duration_secs = excluded.duration_secs,
            cover_path = excluded.cover_path,
            file_size = excluded.file_size,
            modified_at = excluded.modified_at,
            updated_at = excluded.updated_at",
        params![
            path_str,
            meta.title,
            artist_id,
            album_id,
            genre_id,
            meta.year,
            meta.track_number,
            meta.duration_secs,
            cover_path_str,
            file_size,
            modified_at,
            now,
        ],
    )?;

    Ok(!existed)
}

fn get_or_create_artist(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO artists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![name],
    )?;
    conn.query_row(
        "SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |row| row.get(0),
    )
}

fn get_or_create_genre(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO genres (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![name],
    )?;
    conn.query_row(
        "SELECT id FROM genres WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |row| row.get(0),
    )
}

fn get_or_create_album(
    conn: &Connection,
    title: &str,
    artist_id: Option<i64>,
) -> rusqlite::Result<i64> {
    // La restricción UNIQUE(title, artist_id) no atrapa duplicados cuando artist_id es NULL,
    // porque SQLite trata cada NULL como distinto de los demás. Por eso el caso sin artista se
    // busca manualmente antes de insertar.
    match artist_id {
        Some(id) => {
            conn.execute(
                "INSERT INTO albums (title, artist_id) VALUES (?1, ?2)
                 ON CONFLICT(title, artist_id) DO NOTHING",
                params![title, id],
            )?;
            conn.query_row(
                "SELECT id FROM albums WHERE title = ?1 AND artist_id = ?2",
                params![title, id],
                |row| row.get(0),
            )
        }
        None => {
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT id FROM albums WHERE title = ?1 AND artist_id IS NULL",
                    params![title],
                    |row| row.get(0),
                )
                .optional()?;

            match existing {
                Some(id) => Ok(id),
                None => {
                    conn.execute(
                        "INSERT INTO albums (title, artist_id) VALUES (?1, NULL)",
                        params![title],
                    )?;
                    Ok(conn.last_insert_rowid())
                }
            }
        }
    }
}

fn remove_missing_tracks(db: &DbHandle, root: &Path, seen: &[PathBuf]) -> usize {
    let seen: HashSet<PathBuf> = seen.iter().cloned().collect();
    let conn = db.lock();

    let Ok(mut stmt) = conn.prepare("SELECT id, path FROM tracks") else {
        return 0;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    }) else {
        return 0;
    };

    let stale_ids: Vec<i64> = rows
        .filter_map(Result::ok)
        .filter_map(|(id, path_str)| {
            let candidate = PathBuf::from(&path_str);
            (candidate.starts_with(root) && !seen.contains(&candidate)).then_some(id)
        })
        .collect();

    if stale_ids.is_empty() {
        return 0;
    }

    let placeholders = vec!["?"; stale_ids.len()].join(",");
    let sql = format!("DELETE FROM tracks WHERE id IN ({placeholders})");
    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = stale_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();
    let _ = conn.execute(&sql, rusqlite_params.as_slice());

    stale_ids.len()
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")))
    }

    /// Crea una carpeta temporal (aislada por prueba) con copias de los fixtures de audio, una
    /// subcarpeta anidada (para probar la recursividad) y un archivo no soportado (para probar
    /// el filtro de extensión).
    fn setup_temp_library() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "audio-player-scan-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = dir.join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        let fixtures = fixtures_dir();
        std::fs::copy(
            fixtures.join("test-tone-with-cover.mp3"),
            dir.join("con-caratula.mp3"),
        )
        .unwrap();
        std::fs::copy(
            fixtures.join("test-tone-no-tags.mp3"),
            dir.join("sin-tags.mp3"),
        )
        .unwrap();
        std::fs::copy(fixtures.join("test-tone.mp3"), nested.join("tono.mp3")).unwrap();
        std::fs::write(dir.join("notas.txt"), b"esto no es audio").unwrap();

        dir
    }

    #[test]
    fn detects_supported_extensions_case_insensitively() {
        assert!(is_supported_audio_file(Path::new("cancion.mp3")));
        assert!(is_supported_audio_file(Path::new("cancion.MP3")));
        assert!(is_supported_audio_file(Path::new("cancion.flac")));
        assert!(!is_supported_audio_file(Path::new("portada.jpg")));
        assert!(!is_supported_audio_file(Path::new("sin_extension")));
    }

    #[test]
    fn scans_folder_inserts_tracks_and_skips_unchanged_on_rescan() {
        let dir = setup_temp_library();
        let app = tauri::test::mock_app();
        let handle = app.handle();
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();

        let summary = scan_folder(&db, handle, &dir);
        assert_eq!(
            summary.scanned, 3,
            "debería encontrar 3 archivos de audio (uno anidado)"
        );
        assert_eq!(summary.added, 3);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.removed, 0);
        assert_eq!(summary.errors, 0);

        {
            let conn = db.lock();
            let track_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
                .unwrap();
            assert_eq!(track_count, 3);

            struct Row {
                title: String,
                artist: Option<String>,
                album: Option<String>,
                genre: Option<String>,
                year: Option<i64>,
                track_number: Option<i64>,
                cover_path: Option<String>,
            }

            let row = conn
                .query_row(
                    "SELECT t.title, ar.name, al.title, g.name, t.year, t.track_number, t.cover_path
                     FROM tracks t
                     LEFT JOIN artists ar ON ar.id = t.artist_id
                     LEFT JOIN albums al ON al.id = t.album_id
                     LEFT JOIN genres g ON g.id = t.genre_id
                     WHERE t.path LIKE '%con-caratula.mp3'",
                    [],
                    |r| {
                        Ok(Row {
                            title: r.get(0)?,
                            artist: r.get(1)?,
                            album: r.get(2)?,
                            genre: r.get(3)?,
                            year: r.get(4)?,
                            track_number: r.get(5)?,
                            cover_path: r.get(6)?,
                        })
                    },
                )
                .unwrap();

            assert_eq!(row.title, "Con Caratula");
            assert_eq!(row.artist.as_deref(), Some("Artista Prueba"));
            assert_eq!(row.album.as_deref(), Some("Album Prueba"));
            assert_eq!(row.genre.as_deref(), Some("Electronic"));
            assert_eq!(row.year, Some(2024));
            assert_eq!(row.track_number, Some(3));
            assert!(
                row.cover_path.is_some(),
                "debería haber cacheado una carátula"
            );
        }

        // Segundo escaneo sin cambios: no debería reprocesar nada.
        let summary2 = scan_folder(&db, handle, &dir);
        assert_eq!(summary2.scanned, 3);
        assert_eq!(summary2.added, 0);
        assert_eq!(summary2.updated, 0);
        assert_eq!(summary2.removed, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removes_track_from_db_when_file_deleted_before_rescan() {
        let dir = setup_temp_library();
        let app = tauri::test::mock_app();
        let handle = app.handle();
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();

        scan_folder(&db, handle, &dir);

        std::fs::remove_file(dir.join("sin-tags.mp3")).unwrap();
        let summary = scan_folder(&db, handle, &dir);

        assert_eq!(summary.removed, 1);
        let conn = db.lock();
        let track_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(track_count, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_tracks_under_deletes_everything_below_root() {
        let dir = setup_temp_library();
        let app = tauri::test::mock_app();
        let handle = app.handle();
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();

        scan_folder(&db, handle, &dir);
        let removed = remove_tracks_under(&db, &dir);
        assert_eq!(removed, 3);

        let conn = db.lock();
        let track_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(track_count, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
