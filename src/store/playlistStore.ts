import { create } from 'zustand'
import { playlists as playlistsApi } from '../lib/tauri'
import type { PlaylistRecord, PlaylistTrackRecord } from '../lib/types'

interface PlaylistStore {
  playlists: PlaylistRecord[]
  selectedPlaylistId: number | null
  selectedPlaylistTracks: PlaylistTrackRecord[]
  isLoadingTracks: boolean
  lastError: string | null

  refreshPlaylists: () => Promise<void>
  selectPlaylist: (id: number | null) => Promise<void>
  refreshSelectedTracks: () => Promise<void>
  createPlaylist: (name: string) => Promise<number>
  renamePlaylist: (id: number, name: string) => Promise<void>
  deletePlaylist: (id: number) => Promise<void>
  addTracks: (playlistId: number, trackIds: number[]) => Promise<void>
  removeTrack: (playlistId: number, trackId: number) => Promise<void>
  reorderTrack: (playlistId: number, trackId: number, newIndex: number) => Promise<void>
  dismissError: () => void
}

export const usePlaylistStore = create<PlaylistStore>((set, get) => ({
  playlists: [],
  selectedPlaylistId: null,
  selectedPlaylistTracks: [],
  isLoadingTracks: false,
  lastError: null,

  refreshPlaylists: async () => {
    try {
      const playlists = await playlistsApi.list()
      set({ playlists })
    } catch (error) {
      set({ lastError: String(error) })
    }
  },

  selectPlaylist: async (id) => {
    set({ selectedPlaylistId: id })
    await get().refreshSelectedTracks()
  },

  refreshSelectedTracks: async () => {
    const id = get().selectedPlaylistId
    if (id == null) {
      set({ selectedPlaylistTracks: [] })
      return
    }
    set({ isLoadingTracks: true })
    try {
      const tracks = await playlistsApi.listTracks(id)
      set({ selectedPlaylistTracks: tracks })
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isLoadingTracks: false })
    }
  },

  createPlaylist: async (name) => {
    const id = await playlistsApi.create(name)
    await get().refreshPlaylists()
    return id
  },

  renamePlaylist: async (id, name) => {
    await playlistsApi.rename(id, name)
    await get().refreshPlaylists()
  },

  deletePlaylist: async (id) => {
    await playlistsApi.remove(id)
    if (get().selectedPlaylistId === id) {
      set({ selectedPlaylistId: null, selectedPlaylistTracks: [] })
    }
    await get().refreshPlaylists()
  },

  addTracks: async (playlistId, trackIds) => {
    await playlistsApi.addTracks(playlistId, trackIds)
    if (get().selectedPlaylistId === playlistId) {
      await get().refreshSelectedTracks()
    }
    await get().refreshPlaylists()
  },

  removeTrack: async (playlistId, trackId) => {
    await playlistsApi.removeTrack(playlistId, trackId)
    if (get().selectedPlaylistId === playlistId) {
      await get().refreshSelectedTracks()
    }
    await get().refreshPlaylists()
  },

  reorderTrack: async (playlistId, trackId, newIndex) => {
    await playlistsApi.reorderTrack(playlistId, trackId, newIndex)
    if (get().selectedPlaylistId === playlistId) {
      await get().refreshSelectedTracks()
    }
  },

  dismissError: () => set({ lastError: null }),
}))
