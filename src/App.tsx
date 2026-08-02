import { useEffect, useState } from 'react'
import { Sidebar } from './components/layout/Sidebar'
import { PlayerBar } from './components/layout/PlayerBar'
import { QueuePanel } from './components/layout/QueuePanel'
import { LibraryView } from './components/library/LibraryView'
import { PlaylistsView } from './components/playlists/PlaylistsView'
import { EqualizerView } from './components/equalizer/EqualizerView'
import { SettingsView } from './components/settings/SettingsView'
import { ErrorToast } from './components/common/ErrorToast'
import { usePlayerStore } from './store/playerStore'
import { initI18n } from './i18n'

export type ViewId = 'library' | 'playlists' | 'equalizer' | 'settings'

function App() {
  const [view, setView] = useState<ViewId>('library')
  const [queueOpen, setQueueOpen] = useState(false)
  const initPlayer = usePlayerStore((s) => s.init)

  useEffect(() => {
    void initI18n()
    void initPlayer()
  }, [initPlayer])

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-app-bg text-app-text">
      <div className="flex min-h-0 flex-1">
        <Sidebar active={view} onSelect={setView} />

        <main className="min-w-0 flex-1 overflow-hidden">
          {view === 'library' && <LibraryView />}
          {view === 'playlists' && <PlaylistsView />}
          {view === 'equalizer' && <EqualizerView />}
          {view === 'settings' && <SettingsView />}
        </main>

        {queueOpen && <QueuePanel onClose={() => setQueueOpen(false)} />}
      </div>

      <PlayerBar queueOpen={queueOpen} onToggleQueue={() => setQueueOpen((v) => !v)} />
      <ErrorToast />
    </div>
  )
}

export default App
