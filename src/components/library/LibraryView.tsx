import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useLibraryStore } from '../../store/libraryStore'
import { SearchIcon } from '../common/Icons'
import { WatchedFolders } from './WatchedFolders'
import { TrackTable } from './TrackTable'

export function LibraryView() {
  const { t } = useTranslation()
  const init = useLibraryStore((s) => s.init)
  const tracks = useLibraryStore((s) => s.tracks)
  const artists = useLibraryStore((s) => s.artists)
  const albums = useLibraryStore((s) => s.albums)
  const genres = useLibraryStore((s) => s.genres)
  const filter = useLibraryStore((s) => s.filter)
  const setSearch = useLibraryStore((s) => s.setSearch)
  const setArtistFilter = useLibraryStore((s) => s.setArtistFilter)
  const setAlbumFilter = useLibraryStore((s) => s.setAlbumFilter)
  const setGenreFilter = useLibraryStore((s) => s.setGenreFilter)
  const addFolder = useLibraryStore((s) => s.addFolder)
  const rescan = useLibraryStore((s) => s.rescan)
  const isScanning = useLibraryStore((s) => s.isScanning)
  const watchedFolders = useLibraryStore((s) => s.watchedFolders)

  useEffect(() => {
    void init()
  }, [init])

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-app-border px-4 py-3">
        <div className="relative flex-1 min-w-48">
          <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-app-text-muted" />
          <input
            value={filter.search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('library.searchPlaceholder')}
            className="w-full rounded-lg border border-app-border bg-app-bg py-1.5 pl-8 pr-3 text-sm text-app-text placeholder:text-app-text-muted focus:border-app-accent focus:outline-none"
          />
        </div>

        <select
          value={filter.artistId ?? ''}
          onChange={(e) => setArtistFilter(e.target.value ? Number(e.target.value) : null)}
          className="rounded-lg border border-app-border bg-app-bg px-2 py-1.5 text-sm text-app-text focus:border-app-accent focus:outline-none"
        >
          <option value="">{t('library.filters.allArtists')}</option>
          {artists.map((artist) => (
            <option key={artist.id} value={artist.id}>
              {artist.name} ({artist.track_count})
            </option>
          ))}
        </select>

        <select
          value={filter.albumId ?? ''}
          onChange={(e) => setAlbumFilter(e.target.value ? Number(e.target.value) : null)}
          className="rounded-lg border border-app-border bg-app-bg px-2 py-1.5 text-sm text-app-text focus:border-app-accent focus:outline-none"
        >
          <option value="">{t('library.filters.allAlbums')}</option>
          {albums.map((album) => (
            <option key={album.id} value={album.id}>
              {album.title} ({album.track_count})
            </option>
          ))}
        </select>

        <select
          value={filter.genreId ?? ''}
          onChange={(e) => setGenreFilter(e.target.value ? Number(e.target.value) : null)}
          className="rounded-lg border border-app-border bg-app-bg px-2 py-1.5 text-sm text-app-text focus:border-app-accent focus:outline-none"
        >
          <option value="">{t('library.filters.allGenres')}</option>
          {genres.map((genre) => (
            <option key={genre.id} value={genre.id}>
              {genre.name} ({genre.track_count})
            </option>
          ))}
        </select>

        <button
          type="button"
          onClick={() => void addFolder()}
          className="rounded-lg bg-app-accent px-3 py-1.5 text-sm font-medium text-white hover:bg-app-accent-hover"
        >
          {t('library.addFolder')}
        </button>

        {watchedFolders.length > 0 && (
          <button
            type="button"
            onClick={() => void rescan()}
            disabled={isScanning}
            className="rounded-lg border border-app-border px-3 py-1.5 text-sm font-medium text-app-text hover:bg-app-surface-hover disabled:opacity-50"
          >
            {isScanning ? t('library.rescanning') : t('library.rescan')}
          </button>
        )}
      </div>

      <WatchedFolders />

      <div className="flex-1 overflow-y-auto">
        {watchedFolders.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1 px-4 text-center">
            <p className="text-sm text-app-text">{t('library.noFolders')}</p>
            <p className="text-xs text-app-text-muted">{t('library.noFoldersHint')}</p>
          </div>
        ) : isScanning && tracks.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-4 text-center">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-app-border border-t-app-accent" />
            <p className="text-sm text-app-text">{t('library.scanningInProgress')}</p>
          </div>
        ) : (
          <TrackTable tracks={tracks} emptyMessage={t('library.noTracks')} />
        )}
      </div>

      <div className="border-t border-app-border px-4 py-1.5 text-xs text-app-text-muted">
        {t('library.trackCount', { count: tracks.length })}
      </div>
    </div>
  )
}
