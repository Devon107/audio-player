use tauri::State;

use crate::db::{settings, DbHandle};

const LANGUAGE_KEY: &str = "language";
const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "es"];

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
}
