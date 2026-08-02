use tauri::State;

use crate::db::{settings, DbHandle};

const LANGUAGE_KEY: &str = "language";
const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "es"];
const VOLUME_KEY: &str = "volume";

/// Preferencia de idioma guardada, o `None` si el usuario nunca la fijó explícitamente (en ese
/// caso el frontend debería recurrir al idioma del sistema, p. ej. vía `navigator.language`).
#[tauri::command]
pub fn get_language_preference(db: State<DbHandle>) -> Result<Option<String>, String> {
    settings::get(&db.lock(), LANGUAGE_KEY).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_language_preference(db: State<DbHandle>, language: String) -> Result<(), String> {
    validate_language(&language)?;
    settings::set(&db.lock(), LANGUAGE_KEY, &language).map_err(|e| e.to_string())
}

fn validate_language(language: &str) -> Result<(), String> {
    if SUPPORTED_LANGUAGES.contains(&language) {
        Ok(())
    } else {
        Err(format!(
            "Idioma no soportado: '{language}' (soportados: {})",
            SUPPORTED_LANGUAGES.join(", ")
        ))
    }
}

/// Posición guardada del slider de volumen (0.0-1.0, antes de aplicar la curva cúbica que la
/// convierte en ganancia real), o `None` si el usuario nunca lo tocó — en ese caso el frontend
/// se queda con su valor por defecto.
#[tauri::command]
pub fn get_volume_preference(db: State<DbHandle>) -> Result<Option<f32>, String> {
    Ok(settings::get(&db.lock(), VOLUME_KEY)
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse::<f32>().ok()))
}

#[tauri::command]
pub fn set_volume_preference(db: State<DbHandle>, volume: f32) -> Result<(), String> {
    validate_volume(volume)?;
    settings::set(&db.lock(), VOLUME_KEY, &volume.to_string()).map_err(|e| e.to_string())
}

fn validate_volume(volume: f32) -> Result<(), String> {
    if (0.0..=1.0).contains(&volume) {
        Ok(())
    } else {
        Err(format!(
            "Posición de volumen fuera de rango: {volume} (debe estar entre 0.0 y 1.0)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_language_accepts_supported_codes() {
        assert!(validate_language("en").is_ok());
        assert!(validate_language("es").is_ok());
    }

    #[test]
    fn validate_language_rejects_unsupported_codes() {
        assert!(validate_language("fr").is_err());
        assert!(validate_language("").is_err());
        assert!(
            validate_language("ES").is_err(),
            "los códigos son sensibles a mayúsculas"
        );
    }

    #[test]
    fn validate_volume_accepts_the_full_slider_range() {
        assert!(validate_volume(0.0).is_ok());
        assert!(validate_volume(1.0).is_ok());
        assert!(validate_volume(0.5).is_ok());
    }

    #[test]
    fn validate_volume_rejects_out_of_range_values() {
        assert!(validate_volume(-0.01).is_err());
        assert!(validate_volume(1.01).is_err());
    }
}
