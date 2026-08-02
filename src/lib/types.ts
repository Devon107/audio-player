// Tipos que reflejan exactamente los structs serde del backend (src-tauri/src/**/models.rs,
// audio/queue.rs, audio/equalizer.rs). Los nombres de campo se dejan en snake_case a propósito:
// así coinciden byte a byte con el JSON que realmente viaja por el puente IPC, sin necesidad de
// una capa de traducción que se pueda desincronizar en silencio.

export type RepeatMode = 'off' | 'track' | 'queue'

export type EqPreset = 'flat' | 'rock' | 'pop' | 'jazz' | 'custom'

export interface TrackMetadata {
  title: string
  artist: string | null
  album: string | null
  genre: string | null
  year: number | null
  track_number: number | null
  duration_secs: number
  has_cover_art: boolean
}

export interface TrackRecord {
  id: number
  path: string
  title: string
  artist: string | null
  album: string | null
  genre: string | null
  year: number | null
  track_number: number | null
  duration_secs: number
  cover_path: string | null
}

export interface ArtistRecord {
  id: number
  name: string
  track_count: number
}

export interface AlbumRecord {
  id: number
  title: string
  artist: string | null
  track_count: number
}

export interface GenreRecord {
  id: number
  name: string
  track_count: number
}

export interface TrackFilter {
  search?: string | null
  artistId?: number | null
  albumId?: number | null
  genreId?: number | null
  limit?: number | null
  offset?: number | null
}

export interface ScanSummary {
  scanned: number
  added: number
  updated: number
  removed: number
  errors: number
}

export interface QueueTrack {
  id: number
  path: string
  track_id: number | null
}

export interface QueueTrackInput {
  path: string
  track_id: number | null
}

export interface QueueSnapshot {
  items: QueueTrack[]
  current_id: number | null
  shuffle: boolean
  repeat: RepeatMode
  has_previous: boolean
}

export interface EqStateSnapshot {
  gains_db: number[]
  preset: EqPreset
}

export interface PlaylistRecord {
  id: number
  name: string
  track_count: number
  created_at: number
  updated_at: number
}

export interface PlaylistTrackRecord {
  position: number
  track_id: number
  path: string
  title: string
  artist: string | null
  album: string | null
  genre: string | null
  year: number | null
  track_number: number | null
  duration_secs: number
  cover_path: string | null
}

export interface ProgressPayload {
  position_secs: number
  duration_secs: number | null
  is_playing: boolean
}

export interface LoadedPayload {
  duration_secs: number | null
}
