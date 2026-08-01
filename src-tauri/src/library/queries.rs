use rusqlite::{params, Connection};

use super::models::{AlbumRecord, ArtistRecord, GenreRecord, TrackFilter, TrackRecord};

pub fn list_watched_folders(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM watched_folders ORDER BY path")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn list_tracks(conn: &Connection, filter: &TrackFilter) -> rusqlite::Result<Vec<TrackRecord>> {
    let search_pattern = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", escape_like(s)));
    let limit = filter.limit.unwrap_or(500).clamp(1, 2000);
    let offset = filter.offset.unwrap_or(0).max(0);

    let mut stmt = conn.prepare(
        "SELECT t.id, t.path, t.title, ar.name, al.title, g.name, t.year, t.track_number,
                t.duration_secs, t.cover_path
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN genres g ON g.id = t.genre_id
         WHERE (?1 IS NULL OR t.artist_id = ?1)
           AND (?2 IS NULL OR t.album_id = ?2)
           AND (?3 IS NULL OR t.genre_id = ?3)
           AND (?4 IS NULL OR t.title LIKE ?4 ESCAPE '\\'
                OR ar.name LIKE ?4 ESCAPE '\\' OR al.title LIKE ?4 ESCAPE '\\')
         ORDER BY ar.name COLLATE NOCASE, al.title COLLATE NOCASE, t.track_number, t.title COLLATE NOCASE
         LIMIT ?5 OFFSET ?6",
    )?;

    let rows = stmt.query_map(
        params![
            filter.artist_id,
            filter.album_id,
            filter.genre_id,
            search_pattern,
            limit,
            offset
        ],
        |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                genre: row.get(5)?,
                year: row.get(6)?,
                track_number: row.get(7)?,
                duration_secs: row.get(8)?,
                cover_path: row.get(9)?,
            })
        },
    )?;

    rows.collect()
}

pub fn list_artists(conn: &Connection) -> rusqlite::Result<Vec<ArtistRecord>> {
    let mut stmt = conn.prepare(
        "SELECT ar.id, ar.name, COUNT(t.id)
         FROM artists ar
         LEFT JOIN tracks t ON t.artist_id = ar.id
         GROUP BY ar.id
         ORDER BY ar.name COLLATE NOCASE",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ArtistRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            track_count: row.get(2)?,
        })
    })?;

    rows.collect()
}

pub fn list_albums(
    conn: &Connection,
    artist_id: Option<i64>,
) -> rusqlite::Result<Vec<AlbumRecord>> {
    let mut stmt = conn.prepare(
        "SELECT al.id, al.title, ar.name, COUNT(t.id)
         FROM albums al
         LEFT JOIN artists ar ON ar.id = al.artist_id
         LEFT JOIN tracks t ON t.album_id = al.id
         WHERE (?1 IS NULL OR al.artist_id = ?1)
         GROUP BY al.id
         ORDER BY al.title COLLATE NOCASE",
    )?;

    let rows = stmt.query_map(params![artist_id], |row| {
        Ok(AlbumRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            track_count: row.get(3)?,
        })
    })?;

    rows.collect()
}

pub fn list_genres(conn: &Connection) -> rusqlite::Result<Vec<GenreRecord>> {
    let mut stmt = conn.prepare(
        "SELECT g.id, g.name, COUNT(t.id)
         FROM genres g
         LEFT JOIN tracks t ON t.genre_id = g.id
         GROUP BY g.id
         ORDER BY g.name COLLATE NOCASE",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(GenreRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            track_count: row.get(2)?,
        })
    })?;

    rows.collect()
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbHandle;
    use crate::library::scanner;
    use std::path::{Path, PathBuf};

    /// Escanea la carpeta de fixtures (test-tone.mp3, test-tone-with-cover.mp3,
    /// test-tone-no-tags.mp3) en una base de datos en memoria, para tener datos reales y
    /// variados con los que probar los filtros de consulta.
    fn seeded_db() -> DbHandle {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let app = tauri::test::mock_app();
        let fixtures = PathBuf::from(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")));
        scanner::scan_folder(&db, app.handle(), &fixtures);
        db
    }

    #[test]
    fn lists_all_tracks_without_filter() {
        let db = seeded_db();
        let conn = db.lock();
        let tracks = list_tracks(&conn, &TrackFilter::default()).unwrap();
        assert_eq!(tracks.len(), 3);
    }

    #[test]
    fn filters_tracks_by_search_text() {
        let db = seeded_db();
        let conn = db.lock();
        let filter = TrackFilter {
            search: Some("Caratula".to_string()),
            ..Default::default()
        };
        let tracks = list_tracks(&conn, &filter).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Con Caratula");
    }

    #[test]
    fn filters_tracks_by_artist_id() {
        let db = seeded_db();
        let conn = db.lock();
        let artists = list_artists(&conn).unwrap();
        let claude = artists
            .iter()
            .find(|a| a.name == "Claude")
            .expect("debería existir el artista Claude");
        assert_eq!(claude.track_count, 1);

        let filter = TrackFilter {
            artist_id: Some(claude.id),
            ..Default::default()
        };
        let tracks = list_tracks(&conn, &filter).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].artist.as_deref(), Some("Claude"));
    }

    #[test]
    fn lists_albums_optionally_filtered_by_artist() {
        let db = seeded_db();
        let conn = db.lock();
        let albums = list_albums(&conn, None).unwrap();
        assert_eq!(albums.len(), 1, "solo una pista trae álbum en las fixtures");
        assert_eq!(albums[0].title, "Album Prueba");
        assert_eq!(albums[0].track_count, 1);
    }

    #[test]
    fn lists_genres() {
        let db = seeded_db();
        let conn = db.lock();
        let genres = list_genres(&conn).unwrap();
        assert_eq!(genres.len(), 1);
        assert_eq!(genres[0].name, "Electronic");
    }

    #[test]
    fn pagination_limits_results() {
        let db = seeded_db();
        let conn = db.lock();
        let filter = TrackFilter {
            limit: Some(1),
            offset: Some(0),
            ..Default::default()
        };
        let tracks = list_tracks(&conn, &filter).unwrap();
        assert_eq!(tracks.len(), 1);
    }
}
