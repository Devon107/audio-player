import { create } from 'zustand'
import { library } from '../lib/tauri'
import { onLibraryError, onLibraryScanProgress, onLibraryUpdated } from '../lib/events'
import type { AlbumRecord, ArtistRecord, GenreRecord, ScanSummary, TrackRecord } from '../lib/types'

const SEARCH_DEBOUNCE_MS = 250
// Tamaño de cada tanda traída por `list_tracks`, tanto para la carga inicial como para cada
// página siguiente que pide `loadMoreTracks`. No se cargan las decenas de miles de pistas de una
// biblioteca grande de una sola vez (ver Fase 3 del plan) — en cambio, se van pidiendo de a esta
// cantidad a medida que el usuario llega al final de la lista (scroll infinito).
const TRACKS_PAGE_SIZE = 500

interface LibraryFilterState {
  search: string
  artistId: number | null
  albumId: number | null
  genreId: number | null
}

interface LibraryStore {
  tracks: TrackRecord[]
  /** Total de pistas que matchean el filtro actual, sin el tope de `list_tracks` — puede ser
   * mayor a `tracks.length` en bibliotecas grandes (ver `refreshTracks`). */
  totalTracks: number
  artists: ArtistRecord[]
  albums: AlbumRecord[]
  genres: GenreRecord[]
  watchedFolders: string[]
  filter: LibraryFilterState
  isLoadingTracks: boolean
  /** Cargando la siguiente tanda de pistas por scroll infinito (distinto de `isLoadingTracks`,
   * que es la recarga completa al cambiar filtro/búsqueda). */
  isLoadingMoreTracks: boolean
  isScanning: boolean
  /** Conteo parcial del escaneo en curso, actualizado en vivo vía `library://scan-progress`.
   * `null` cuando no hay ningún escaneo corriendo. */
  scanProgress: ScanSummary | null
  lastScanSummary: ScanSummary | null
  lastError: string | null
  initialized: boolean

  init: () => Promise<void>
  refreshTracks: () => Promise<void>
  loadMoreTracks: () => Promise<void>
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
  totalTracks: 0,
  artists: [],
  albums: [],
  genres: [],
  watchedFolders: [],
  filter: { search: '', artistId: null, albumId: null, genreId: null },
  isLoadingTracks: false,
  isLoadingMoreTracks: false,
  isScanning: false,
  scanProgress: null,
  lastScanSummary: null,
  lastError: null,
  initialized: false,

  init: async () => {
    if (get().initialized) return
    set({ initialized: true })

    onLibraryUpdated(() => {
      void get().refreshTracks()
      void get().refreshSidebarLists()
      set({ isScanning: false, scanProgress: null })
    })
    onLibraryScanProgress((summary) => {
      set({ scanProgress: summary })
      void get().refreshTracks()
      void get().refreshSidebarLists()
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
    const filter = {
      search: search.trim() || null,
      artistId,
      albumId,
      genreId,
    }
    set({ isLoadingTracks: true })
    try {
      // `listTracks` trae como mucho `TRACKS_PAGE_SIZE` filas por pedido (evita cargar decenas
      // de miles de golpe a la UI), así que en bibliotecas grandes `tracks.length` por sí solo
      // no alcanza para mostrar un conteo correcto — `countTracks` pide el total real aparte, sin
      // ese tope. El resto de las pistas se van trayendo con `loadMoreTracks` a medida que el
      // usuario llega al final de la lista.
      const [tracks, totalTracks] = await Promise.all([
        library.listTracks({ ...filter, limit: TRACKS_PAGE_SIZE, offset: 0 }),
        library.countTracks(filter),
      ])
      set({ tracks, totalTracks })
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isLoadingTracks: false })
    }
  },

  loadMoreTracks: async () => {
    const { search, artistId, albumId, genreId } = get().filter
    const { tracks, totalTracks, isLoadingTracks, isLoadingMoreTracks } = get()
    if (isLoadingTracks || isLoadingMoreTracks || tracks.length >= totalTracks) return

    set({ isLoadingMoreTracks: true })
    try {
      const nextPage = await library.listTracks({
        search: search.trim() || null,
        artistId,
        albumId,
        genreId,
        limit: TRACKS_PAGE_SIZE,
        offset: tracks.length,
      })
      // Chequeo contra una condición de carrera: si el filtro cambió mientras esta página
      // estaba en vuelo, `refreshTracks` ya reemplazó `tracks` por una lista nueva — pegarle acá
      // esta respuesta vieja mezclaría resultados de dos filtros distintos.
      if (get().filter.search === search && get().filter.artistId === artistId) {
        set((state) => ({ tracks: [...state.tracks, ...nextPage] }))
      }
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isLoadingMoreTracks: false })
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
      set({ isScanning: true, scanProgress: null })
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
    set({ isScanning: true, scanProgress: null })
    try {
      const summary = await library.rescanLibrary()
      set({ lastScanSummary: summary })
      await get().refreshTracks()
      await get().refreshSidebarLists()
    } catch (error) {
      set({ lastError: String(error) })
    } finally {
      set({ isScanning: false, scanProgress: null })
    }
  },

  dismissError: () => set({ lastError: null }),
}))
