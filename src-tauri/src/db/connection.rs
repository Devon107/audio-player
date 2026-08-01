use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use tauri::{AppHandle, Manager, Runtime};

const SCHEMA: &str = include_str!("schema.sql");
const DB_FILE_NAME: &str = "library.sqlite";

/// Conexión SQLite compartida entre comandos Tauri y el hilo del watcher de biblioteca.
#[derive(Clone)]
pub struct DbHandle(Arc<Mutex<Connection>>);

impl DbHandle {
    pub fn open_at(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path)
            .map_err(|e| format!("No se pudo abrir la base de datos: {e}"))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("No se pudo aplicar el esquema: {e}"))?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Abre (creando si hace falta) la base de datos de la biblioteca en el directorio de datos de
/// la app y aplica el esquema.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> Result<DbHandle, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No se pudo determinar el directorio de datos: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos: {e}"))?;
    DbHandle::open_at(&dir.join(DB_FILE_NAME))
}
