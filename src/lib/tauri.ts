import { invoke } from '@tauri-apps/api/core'
import type {
  AlbumRecord,
  ArtistRecord,
  EqPreset,
  EqStateSnapshot,
  GenreRecord,
  PlaylistRecord,
  PlaylistTrackRecord,
  QueueSnapshot,
  QueueTrackInput,
  RepeatMode,
  ScanSummary,
  TrackFilter,
  TrackMetadata,
  TrackRecord,
} from './types'

// Capa delgada y tipada sobre `invoke()`. Los nombres de los argumentos van en camelCase a
// propósito: el macro `#[tauri::command]` convierte los parámetros de Rust (snake_case) a
// camelCase para el lado JS automáticamente, así que las claves de este objeto deben coincidir
// con esa convención, no con el nombre del parámetro en Rust.

// --- Reproducción y cola (Fase 1 y 4) ---

export const audio = {
  loadTrack: (path: string, autoplay: boolean) => invoke<void>('load_track', { path, autoplay }),
  play: () => invoke<void>('play'),
  pause: () => invoke<void>('pause'),
  stop: () => invoke<void>('stop'),
  seek: (positionSecs: number) => invoke<void>('seek', { positionSecs }),
  setVolume: (volume: number) => invoke<void>('set_volume', { volume }),

  setQueue: (items: QueueTrackInput[], startIndex: number | null, autoplay: boolean) =>
    invoke<void>('set_queue', { items, startIndex, autoplay }),
  addToQueue: (items: QueueTrackInput[]) => invoke<void>('add_to_queue', { items }),
  removeFromQueue: (itemId: number) => invoke<void>('remove_from_queue', { itemId }),
  reorderQueue: (itemId: number, newIndex: number) =>
    invoke<void>('reorder_queue', { itemId, newIndex }),
  clearQueue: () => invoke<void>('clear_queue'),
  playQueueItem: (itemId: number) => invoke<void>('play_queue_item', { itemId }),
  nextTrack: () => invoke<void>('next_track'),
  previousTrack: () => invoke<void>('previous_track'),
  setShuffle: (enabled: boolean) => invoke<void>('set_shuffle', { enabled }),
  setRepeatMode: (mode: RepeatMode) => invoke<void>('set_repeat_mode', { mode }),
  getQueueState: () => invoke<QueueSnapshot>('get_queue_state'),

  setEqBandGain: (band: number, gainDb: number) =>
    invoke<void>('set_eq_band_gain', { band, gainDb }),
  setEqPreset: (preset: EqPreset) => invoke<void>('set_eq_preset', { preset }),
  getEqState: () => invoke<EqStateSnapshot>('get_eq_state'),
}

// --- Metadatos y carátulas (Fase 2) ---

export const metadata = {
  readTrackMetadata: (path: string) => invoke<TrackMetadata>('read_track_metadata', { path }),
  getCoverArt: (path: string) => invoke<string | null>('get_cover_art', { path }),
}

// --- Biblioteca (Fase 3) ---

export const library = {
  pickAndAddFolder: () => invoke<string | null>('pick_and_add_folder'),
  listWatchedFolders: () => invoke<string[]>('list_watched_folders'),
  removeWatchedFolder: (path: string) => invoke<void>('remove_watched_folder', { path }),
  rescanLibrary: () => invoke<ScanSummary>('rescan_library'),
  listTracks: (filter: TrackFilter) => invoke<TrackRecord[]>('list_tracks', { filter }),
  countTracks: (filter: TrackFilter) => invoke<number>('count_tracks', { filter }),
  listArtists: () => invoke<ArtistRecord[]>('list_artists'),
  listAlbums: (artistId: number | null) => invoke<AlbumRecord[]>('list_albums', { artistId }),
  listGenres: () => invoke<GenreRecord[]>('list_genres'),
}

// --- Playlists (Fase 5) ---

export const playlists = {
  create: (name: string) => invoke<number>('create_playlist', { name }),
  rename: (playlistId: number, name: string) =>
    invoke<void>('rename_playlist', { playlistId, name }),
  remove: (playlistId: number) => invoke<void>('delete_playlist', { playlistId }),
  list: () => invoke<PlaylistRecord[]>('list_playlists'),
  listTracks: (playlistId: number) =>
    invoke<PlaylistTrackRecord[]>('list_playlist_tracks', { playlistId }),
  addTracks: (playlistId: number, trackIds: number[]) =>
    invoke<void>('add_tracks_to_playlist', { playlistId, trackIds }),
  removeTrack: (playlistId: number, trackId: number) =>
    invoke<void>('remove_track_from_playlist', { playlistId, trackId }),
  reorderTrack: (playlistId: number, trackId: number, newIndex: number) =>
    invoke<void>('reorder_playlist_track', { playlistId, trackId, newIndex }),
  play: (playlistId: number, startIndex: number | null, autoplay: boolean) =>
    invoke<void>('play_playlist', { playlistId, startIndex, autoplay }),
  exportM3u: (playlistId: number, targetPath: string) =>
    invoke<void>('export_playlist_m3u', { playlistId, targetPath }),
  importM3u: (sourcePath: string, playlistName: string | null) =>
    invoke<number>('import_playlist_m3u', { sourcePath, playlistName }),
}

// --- Ajustes (Fase 7) ---

export const settings = {
  getLanguage: () => invoke<string | null>('get_language_preference'),
  setLanguage: (language: string) => invoke<void>('set_language_preference', { language }),
  getVolume: () => invoke<number | null>('get_volume_preference'),
  setVolume: (volume: number) => invoke<void>('set_volume_preference', { volume }),
}
