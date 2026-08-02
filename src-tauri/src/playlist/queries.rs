use rusqlite::{params, Connection};

use super::models::{PlaylistRecord, PlaylistTrackRecord};

pub fn create_playlist(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    let now = unix_now();
    conn.execute(
        "INSERT INTO playlists (name, created_at, updated_at) VALUES (?1, ?2, ?2)",
        params![name, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_playlist(conn: &Connection, playlist_id: i64, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE playlists SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, unix_now(), playlist_id],
    )?;
    Ok(())
}

pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
    Ok(())
}

pub fn list_playlists(conn: &Connection) -> rusqlite::Result<Vec<PlaylistRecord>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, COUNT(pt.track_id), p.created_at, p.updated_at
         FROM playlists p
         LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
         GROUP BY p.id
         ORDER BY p.name COLLATE NOCASE",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(PlaylistRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            track_count: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn list_playlist_tracks(
    conn: &Connection,
    playlist_id: i64,
) -> rusqlite::Result<Vec<PlaylistTrackRecord>> {
    let mut stmt = conn.prepare(
        "SELECT pt.position, t.id, t.path, t.title, ar.name, al.title, g.name, t.year,
                t.track_number, t.duration_secs, t.cover_path
         FROM playlist_tracks pt
         JOIN tracks t ON t.id = pt.track_id
         LEFT JOIN artists ar ON ar.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN genres g ON g.id = t.genre_id
         WHERE pt.playlist_id = ?1
         ORDER BY pt.position",
    )?;

    let rows = stmt.query_map(params![playlist_id], |row| {
        Ok(PlaylistTrackRecord {
            position: row.get(0)?,
            track_id: row.get(1)?,
            path: row.get(2)?,
            title: row.get(3)?,
            artist: row.get(4)?,
            album: row.get(5)?,
            genre: row.get(6)?,
            year: row.get(7)?,
            track_number: row.get(8)?,
            duration_secs: row.get(9)?,
            cover_path: row.get(10)?,
        })
    })?;

    rows.collect()
}

/// Agrega pistas al final de la playlist. Las que ya estuvieran (mismo `track_id`) se ignoran en
/// silencio gracias a la clave primaria compuesta (playlist_id, track_id).
pub fn add_tracks(conn: &Connection, playlist_id: i64, track_ids: &[i64]) -> rusqlite::Result<()> {
    let mut next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
        params![playlist_id],
        |row| row.get(0),
    )?;

    for track_id in track_ids {
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, next_position],
        )?;
        if inserted > 0 {
            next_position += 1;
        }
    }

    touch_playlist(conn, playlist_id)
}

pub fn remove_track(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
        params![playlist_id, track_id],
    )?;
    touch_playlist(conn, playlist_id)
}

/// Mueve `track_id` a la posición `new_index` (base 0) dentro de la playlist, reacomodando el
/// resto. Reescribe todas las posiciones dentro de una transacción para no depender de
/// aritmética de huecos entre índices.
pub fn reorder_track(
    conn: &mut Connection,
    playlist_id: i64,
    track_id: i64,
    new_index: usize,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;

    let mut ids: Vec<i64> = {
        let mut stmt = tx.prepare(
            "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position",
        )?;
        let rows = stmt.query_map(params![playlist_id], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let Some(pos) = ids.iter().position(|id| *id == track_id) else {
        return Ok(());
    };
    let id = ids.remove(pos);
    let clamped = new_index.min(ids.len());
    ids.insert(clamped, id);

    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
            params![position as i64, playlist_id, id],
        )?;
    }

    tx.execute(
        "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
        params![unix_now(), playlist_id],
    )?;

    tx.commit()
}

fn touch_playlist(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
        params![unix_now(), playlist_id],
    )?;
    Ok(())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbHandle;
    use crate::library::scanner;
    use std::path::{Path, PathBuf};

    /// Escanea las fixtures de audio en una base en memoria y devuelve el handle junto con los
    /// ids de las 3 pistas resultantes, ordenados alfabéticamente por ruta de archivo (que es el
    /// orden en que `walkdir` normalmente los recorre en estos tests).
    fn seeded_db_with_track_ids() -> (DbHandle, Vec<i64>) {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let app = tauri::test::mock_app();
        let fixtures = PathBuf::from(format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")));
        scanner::scan_folder(&db, app.handle(), &fixtures);

        let conn = db.lock();
        let mut stmt = conn.prepare("SELECT id FROM tracks ORDER BY id").unwrap();
        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        drop(stmt);
        drop(conn);

        assert_eq!(ids.len(), 3, "las fixtures deberían producir 3 pistas");
        (db, ids)
    }

    #[test]
    fn create_rename_delete_playlist_lifecycle() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let conn = db.lock();

        let id = create_playlist(&conn, "Favoritas").unwrap();
        let playlists = list_playlists(&conn).unwrap();
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].name, "Favoritas");
        assert_eq!(playlists[0].track_count, 0);

        rename_playlist(&conn, id, "Mis favoritas").unwrap();
        assert_eq!(list_playlists(&conn).unwrap()[0].name, "Mis favoritas");

        delete_playlist(&conn, id).unwrap();
        assert!(list_playlists(&conn).unwrap().is_empty());
    }

    #[test]
    fn add_tracks_appends_in_order_and_ignores_duplicates() {
        let (db, ids) = seeded_db_with_track_ids();
        let conn = db.lock();
        let playlist_id = create_playlist(&conn, "Mi playlist").unwrap();

        add_tracks(&conn, playlist_id, &ids).unwrap();
        let tracks = list_playlist_tracks(&conn, playlist_id).unwrap();
        assert_eq!(tracks.len(), 3);
        assert_eq!(
            tracks.iter().map(|t| t.track_id).collect::<Vec<_>>(),
            ids,
            "deben quedar en el mismo orden en que se agregaron"
        );
        assert_eq!(list_playlists(&conn).unwrap()[0].track_count, 3);

        // Agregar las mismas pistas de nuevo no debería duplicarlas.
        add_tracks(&conn, playlist_id, &ids).unwrap();
        assert_eq!(list_playlist_tracks(&conn, playlist_id).unwrap().len(), 3);
    }

    #[test]
    fn remove_track_only_affects_the_playlist_not_the_library() {
        let (db, ids) = seeded_db_with_track_ids();
        let conn = db.lock();
        let playlist_id = create_playlist(&conn, "Mi playlist").unwrap();
        add_tracks(&conn, playlist_id, &ids).unwrap();

        remove_track(&conn, playlist_id, ids[1]).unwrap();

        let remaining = list_playlist_tracks(&conn, playlist_id).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().all(|t| t.track_id != ids[1]));

        let track_count_in_library: i64 = conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            track_count_in_library, 3,
            "quitar de la playlist no borra la pista de la biblioteca"
        );
    }

    #[test]
    fn reorder_track_moves_it_to_the_requested_position() {
        let (db_owner, ids) = seeded_db_with_track_ids();
        let playlist_id = {
            let conn = db_owner.lock();
            let id = create_playlist(&conn, "Mi playlist").unwrap();
            add_tracks(&conn, id, &ids).unwrap();
            id
        };

        {
            let mut conn = db_owner.lock();
            reorder_track(&mut conn, playlist_id, ids[0], 2).unwrap();
        }

        let conn = db_owner.lock();
        let tracks = list_playlist_tracks(&conn, playlist_id).unwrap();
        let track_ids: Vec<i64> = tracks.iter().map(|t| t.track_id).collect();
        assert_eq!(track_ids, vec![ids[1], ids[2], ids[0]]);

        // Las posiciones quedan reescritas de forma contigua 0..n.
        let positions: Vec<i64> = tracks.iter().map(|t| t.position).collect();
        assert_eq!(positions, vec![0, 1, 2]);
    }
}
