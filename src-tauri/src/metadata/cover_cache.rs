use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use super::reader;

const COVERS_SUBDIR: &str = "covers";

/// Obtiene la ruta en disco de la carátula de `path`, extrayéndola y cacheándola en el
/// directorio de caché de la app si todavía no existe. Devuelve `None` si el archivo no tiene
/// ninguna carátula embebida.
///
/// Genérico sobre `R: Runtime` para poder probarse con `tauri::test::mock_app()`, que usa
/// `MockRuntime` en lugar del runtime `Wry` por defecto.
pub fn get_or_extract_cover<R: Runtime>(
    app: &AppHandle<R>,
    path: &Path,
) -> Result<Option<PathBuf>, String> {
    let covers_dir = covers_dir(app)?;
    let key = cache_key(path)?;

    // Ya se extrajo antes: se reutiliza sin volver a leer las etiquetas del archivo.
    if let Some(cached) = find_cached(&covers_dir, &key) {
        return Ok(Some(cached));
    }

    let Some((bytes, extension)) = reader::extract_cover_bytes(path)? else {
        return Ok(None);
    };

    let cache_path = covers_dir.join(format!("{key}.{extension}"));
    fs::write(&cache_path, &bytes)
        .map_err(|e| format!("No se pudo escribir la carátula en caché: {e}"))?;

    Ok(Some(cache_path))
}

fn covers_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("No se pudo determinar el directorio de caché: {e}"))?;
    let dir = base.join(COVERS_SUBDIR);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear el directorio de caché de carátulas: {e}"))?;
    Ok(dir)
}

fn find_cached(covers_dir: &Path, key: &str) -> Option<PathBuf> {
    for extension in ["jpg", "png", "tiff", "bmp", "gif", "bin"] {
        let candidate = covers_dir.join(format!("{key}.{extension}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Clave de caché derivada de la ruta absoluta, tamaño y fecha de modificación del archivo, de
/// forma que si el archivo cambia (p. ej. se reemplaza con otra versión) la carátula se vuelve a
/// extraer en lugar de servir una copia obsoleta.
fn cache_key(path: &Path) -> Result<String, String> {
    let meta =
        fs::metadata(path).map_err(|e| format!("No se pudo leer '{}': {e}", path.display()))?;
    let modified_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut hash: u64 = 0xcbf29ce484222325;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    feed(path.to_string_lossy().as_bytes());
    feed(&meta.len().to_le_bytes());
    feed(&modified_secs.to_le_bytes());

    Ok(format!("{hash:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    #[test]
    fn extracts_and_caches_then_reuses_cache() {
        let app = tauri::test::mock_app();
        let handle = app.handle();

        // Aislar la caché de esta prueba de otras ejecuciones/pruebas en paralelo.
        let unique_cache_dir = std::env::temp_dir().join(format!(
            "audio-player-test-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&unique_cache_dir).unwrap();

        let path = fixture("test-tone-with-cover.mp3");
        let key = cache_key(&path).expect("debería calcular la clave de caché");

        // Primera extracción: no hay caché todavía.
        assert!(find_cached(&unique_cache_dir, &key).is_none());
        let (bytes, extension) = reader::extract_cover_bytes(&path)
            .unwrap()
            .expect("debería tener carátula");
        let cache_path = unique_cache_dir.join(format!("{key}.{extension}"));
        std::fs::write(&cache_path, &bytes).unwrap();

        // Segunda "extracción": debe reutilizar el archivo cacheado.
        let cached = find_cached(&unique_cache_dir, &key).expect("debería encontrar la caché");
        assert_eq!(cached, cache_path);
        assert_eq!(std::fs::read(&cached).unwrap(), bytes);

        // También se verifica el flujo completo `get_or_extract_cover` usando el AppHandle real,
        // que escribe en el directorio de caché de la app (no el aislado de arriba).
        let result = get_or_extract_cover(handle, &path)
            .expect("no debería fallar")
            .expect("debería devolver una ruta de carátula");
        assert!(result.is_file());

        let _ = std::fs::remove_dir_all(&unique_cache_dir);
    }

    #[test]
    fn returns_none_for_track_without_cover() {
        let app = tauri::test::mock_app();
        let handle = app.handle();

        let result = get_or_extract_cover(handle, &fixture("test-tone-no-tags.mp3"))
            .expect("no debería fallar");
        assert!(result.is_none());
    }
}
