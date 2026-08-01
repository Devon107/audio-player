use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TrackRecord {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub duration_secs: f64,
    pub cover_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtistRecord {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlbumRecord {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenreRecord {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
}

/// Filtros y paginación para consultar la biblioteca. Todos los campos son opcionales; `limit`
/// por defecto evita traer bibliotecas enteras a la UI de una sola vez.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackFilter {
    pub search: Option<String>,
    pub artist_id: Option<i64>,
    pub album_id: Option<i64>,
    pub genre_id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ScanSummary {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub errors: usize,
}
