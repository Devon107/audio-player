import { usePlayerStore } from '../../store/playerStore'
import { useLibraryStore } from '../../store/libraryStore'
import { usePlaylistStore } from '../../store/playlistStore'
import { CloseIcon } from './Icons'

/// Muestra el último error reportado por cualquiera de los stores (motor de audio, biblioteca,
/// playlists). Sin esto, los errores del backend quedaban guardados en estado pero nunca se
/// mostraban en ningún lado.
export function ErrorToast() {
  const playerError = usePlayerStore((s) => s.lastError)
  const dismissPlayerError = usePlayerStore((s) => s.dismissError)
  const libraryError = useLibraryStore((s) => s.lastError)
  const dismissLibraryError = useLibraryStore((s) => s.dismissError)
  const playlistError = usePlaylistStore((s) => s.lastError)
  const dismissPlaylistError = usePlaylistStore((s) => s.dismissError)

  const entry = playerError
    ? { message: playerError, dismiss: dismissPlayerError }
    : libraryError
      ? { message: libraryError, dismiss: dismissLibraryError }
      : playlistError
        ? { message: playlistError, dismiss: dismissPlaylistError }
        : null

  if (!entry) return null

  return (
    <div className="pointer-events-none fixed inset-x-0 bottom-24 z-50 flex justify-center px-4">
      <div className="pointer-events-auto flex max-w-lg items-start gap-3 rounded-lg border border-red-500/30 bg-app-surface px-4 py-3 text-sm text-app-text shadow-xl">
        <p className="flex-1">{entry.message}</p>
        <button
          type="button"
          onClick={entry.dismiss}
          className="text-app-text-muted hover:text-app-text"
        >
          <CloseIcon className="h-4 w-4" />
        </button>
      </div>
    </div>
  )
}
