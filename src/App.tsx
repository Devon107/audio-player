import { useEffect, useState } from 'react'
import { Sidebar } from './components/layout/Sidebar'
import { PlayerBar } from './components/layout/PlayerBar'
import { QueuePanel } from './components/layout/QueuePanel'
import { LibraryView } from './components/library/LibraryView'
import { PlaylistsView } from './components/playlists/PlaylistsView'
import { EqualizerView } from './components/equalizer/EqualizerView'
import { SettingsView } from './components/settings/SettingsView'
import { NowPlayingView } from './components/nowplaying/NowPlayingView'
import { ErrorToast } from './components/common/ErrorToast'
import { usePlayerStore } from './store/playerStore'
import { useLibraryStore } from './store/libraryStore'
import { useGlobalShortcuts } from './lib/useGlobalShortcuts'
import { initI18n } from './i18n'

export type ViewId = 'library' | 'playlists' | 'equalizer' | 'settings' | 'nowPlaying'

function App() {
  const [view, setView] = useState<ViewId>('library')
  const [previousView, setPreviousView] = useState<ViewId>('library')
  const [queueOpen, setQueueOpen] = useState(false)
  const initPlayer = usePlayerStore((s) => s.init)
  const initLibrary = useLibraryStore((s) => s.init)
  const setArtistFilter = useLibraryStore((s) => s.setArtistFilter)

  useEffect(() => {
    void initI18n()
    void initPlayer()
    // La biblioteca se carga acá (no solo perezosamente al visitar la vista) porque la barra de
    // reproducción necesita la lista de artistas para resolver el filtro al hacer clic en el
    // nombre del artista, sin importar qué vista esté activa en ese momento.
    void initLibrary()
  }, [initPlayer, initLibrary])

  useGlobalShortcuts()

  const openNowPlaying = () => {
    if (view !== 'nowPlaying') setPreviousView(view)
    setView('nowPlaying')
  }

  const selectView = (next: ViewId) => {
    if (view !== 'nowPlaying') setPreviousView(view)
    setView(next)
  }

  const filterByArtist = (artistId: number) => {
    setArtistFilter(artistId)
    selectView('library')
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-app-bg text-app-text">
      <div className="flex min-h-0 flex-1">
        <Sidebar active={view} onSelect={selectView} />

        <main className="min-w-0 flex-1 overflow-hidden">
          {view === 'library' && <LibraryView />}
          {view === 'playlists' && <PlaylistsView />}
          {view === 'equalizer' && <EqualizerView />}
          {view === 'settings' && <SettingsView />}
          {view === 'nowPlaying' && <NowPlayingView onBack={() => setView(previousView)} />}
        </main>

        {queueOpen && <QueuePanel onClose={() => setQueueOpen(false)} />}
      </div>

      <PlayerBar
        queueOpen={queueOpen}
        onToggleQueue={() => setQueueOpen((v) => !v)}
        onOpenNowPlaying={openNowPlaying}
        onFilterArtist={filterByArtist}
      />
      <ErrorToast />
    </div>
  )
}

export default App
