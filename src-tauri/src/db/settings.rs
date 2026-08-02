use rusqlite::{params, Connection, OptionalExtension};

/// Lee un valor de la tabla `settings` (clave-valor genérica para preferencias que deben
/// persistir entre sesiones: ecualizador, idioma, etc.).
pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
}

pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbHandle;
    use std::path::Path;

    #[test]
    fn set_then_get_round_trips() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let conn = db.lock();

        assert_eq!(get(&conn, "lang").unwrap(), None);

        set(&conn, "lang", "es").unwrap();
        assert_eq!(get(&conn, "lang").unwrap(), Some("es".to_string()));

        set(&conn, "lang", "en").unwrap();
        assert_eq!(
            get(&conn, "lang").unwrap(),
            Some("en".to_string()),
            "set debe sobrescribir"
        );
    }
}
