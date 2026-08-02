import { create } from 'zustand'
import { library } from '../lib/tauri'
import { onLibraryError, onLibraryUpdated } from '../lib/events'
import type { AlbumRecord, ArtistRecord, GenreRecord, ScanSummary, TrackRecord } from '../lib/types'

const SEARCH_DEBOUNCE_MS = 250

interface LibraryFilterState {
  search: string
  artistId: number | null
  albumId: number | null
  genreId: number | null
}

interface LibraryStore {
  tracks: TrackRecord[]
  artists: ArtistRecord[]
  albums: AlbumRecord[]
  genres: GenreRecord[]
  watchedFolders: string[]
  filter: LibraryFilterState
  isLoadingTracks: boolean
  isScanning: boolean
  lastScanSummary: ScanSummary | null
  lastError: string | null
  initialized: boolean

  init: () => Promise<void>
  refreshTracks: () => Promise<void>
  refreshSidebarLists: () => Promise<void>
  refreshWatchedFolders: () => Promise<void>
  setSearch: (search: string) => void
  setArtistFilter: (artistId: number | null) => void
  setAlbumFilter: (albumId: number | null) => void
  setGenreFilter: (genreId: number | null) => void
  addFolder: () => Promise<string | null>
  removeFolder: (path: string) => Promise<void>
  rescan: () => Promise<void>
  dismissError: () => void
}

let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  tracks: [],
  artists: [],
  albums: [],
  genres: [],
  watchedFolders: [],
  filter: { search: '', artistId: null, albumId: null, genreId: null },
  isLoadingTracks: false,
  isScanning: false,
  lastScanSummary: null,
  lastError: null,
  initialized: false,

  init: async () => {
    if (get().initialized) return
    set({ initialized: true })

    onLibraryUpdated(() => {
      void get().refreshTracks()
      void get().refreshSidebarLists()
      set({ isScanning: false })
    })
    onLibraryError((message) => set({ lastError: message }))

    await Promise.all([
      get().refreshTracks(),
      get().refreshSidebarLists(),
      get().refreshWatchedFolders(),
    ])
  },

  refreshTracks: async () => {
    const { search, artistId, albumId, genreId } = get().filter
    set({ isLoadingTracks: true })
    try {
      const tracks = await library.listTracks({
        search: search.trim() || null,
        artistId,
        albumId,
        genreId,
        limit: 1000,
        offset: 0,
      })
      set({ tracks })
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isLoadingTracks: false })
    }
  },

  refreshSidebarLists: async () => {
    try {
      const [artists, albums, genres] = await Promise.all([
        library.listArtists(),
        library.listAlbums(null),
        library.listGenres(),
      ])
      set({ artists, albums, genres })
    } catch (error) {
      set({ lastError: String(error) })
    }
  },

  refreshWatchedFolders: async () => {
    try {
      const watchedFolders = await library.listWatchedFolders()
      set({ watchedFolders })
    } catch (error) {
      set({ lastError: String(error) })
    }
  },

  setSearch: (search) => {
    set((state) => ({ filter: { ...state.filter, search } }))
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer)
    searchDebounceTimer = setTimeout(() => {
      void get().refreshTracks()
    }, SEARCH_DEBOUNCE_MS)
  },

  setArtistFilter: (artistId) => {
    set((state) => ({ filter: { ...state.filter, artistId } }))
    void get().refreshTracks()
  },

  setAlbumFilter: (albumId) => {
    set((state) => ({ filter: { ...state.filter, albumId } }))
    void get().refreshTracks()
  },

  setGenreFilter: (genreId) => {
    set((state) => ({ filter: { ...state.filter, genreId } }))
    void get().refreshTracks()
  },

  addFolder: async () => {
    const path = await library.pickAndAddFolder()
    if (path) {
      // El escaneo inicial de la carpeta corre en el backend y puede tardar (lee metadatos y
      // extrae carátulas de cada archivo). `isScanning` se apaga cuando llega el evento
      // `library://updated`, no acá — así la UI muestra que está trabajando mientras tanto.
      set({ isScanning: true })
      await get().refreshWatchedFolders()
    }
    return path
  },

  removeFolder: async (path) => {
    await library.removeWatchedFolder(path)
    await get().refreshWatchedFolders()
    await get().refreshTracks()
    await get().refreshSidebarLists()
  },

  rescan: async () => {
    set({ isScanning: true })
    try {
      const summary = await library.rescanLibrary()
      set({ lastScanSummary: summary })
      await get().refreshTracks()
      await get().refreshSidebarLists()
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isScanning: false })
    }
  },

  dismissError: () => set({ lastError: null }),
}))
