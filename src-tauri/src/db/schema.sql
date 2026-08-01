PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS artists (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE IF NOT EXISTS albums (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    title     TEXT NOT NULL,
    artist_id INTEGER REFERENCES artists (id) ON DELETE SET NULL,
    UNIQUE (title, artist_id)
);

CREATE TABLE IF NOT EXISTS genres (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE IF NOT EXISTS tracks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    path          TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    artist_id     INTEGER REFERENCES artists (id) ON DELETE SET NULL,
    album_id      INTEGER REFERENCES albums (id) ON DELETE SET NULL,
    genre_id      INTEGER REFERENCES genres (id) ON DELETE SET NULL,
    year          INTEGER,
    track_number  INTEGER,
    duration_secs REAL NOT NULL,
    cover_path    TEXT,
    file_size     INTEGER NOT NULL,
    modified_at   INTEGER NOT NULL,
    added_at      INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks (artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks (album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks (genre_id);
CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks (title COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS playlists (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, track_id)
);

CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks (playlist_id, position);

CREATE TABLE IF NOT EXISTS watched_folders (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
