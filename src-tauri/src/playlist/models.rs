use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistRecord {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistTrackRecord {
    pub position: i64,
    pub track_id: i64,
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
