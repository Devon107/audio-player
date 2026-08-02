import type { PlaylistTrackRecord, QueueTrackInput, TrackRecord } from './types'

/// Forma mínima común entre `TrackRecord` (biblioteca) y `PlaylistTrackRecord` (playlist), para
/// poder reutilizar el mismo componente de fila de pista en ambas vistas.
export interface DisplayTrack {
  trackId: number
  path: string
  title: string
  artist: string | null
  album: string | null
  genre: string | null
  durationSecs: number
  coverPath: string | null
}

export function fromTrackRecord(track: TrackRecord): DisplayTrack {
  return {
    trackId: track.id,
    path: track.path,
    title: track.title,
    artist: track.artist,
    album: track.album,
    genre: track.genre,
    durationSecs: track.duration_secs,
    coverPath: track.cover_path,
  }
}

export function fromPlaylistTrackRecord(track: PlaylistTrackRecord): DisplayTrack {
  return {
    trackId: track.track_id,
    path: track.path,
    title: track.title,
    artist: track.artist,
    album: track.album,
    genre: track.genre,
    durationSecs: track.duration_secs,
    coverPath: track.cover_path,
  }
}

export function toQueueInput(track: DisplayTrack): QueueTrackInput {
  return { path: track.path, track_id: track.trackId }
}
