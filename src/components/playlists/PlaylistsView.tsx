import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePlaylistStore } from '../../store/playlistStore'
import { PlaylistList } from './PlaylistList'
import { PlaylistDetail } from './PlaylistDetail'
import { MusicNoteIcon } from '../common/Icons'

export function PlaylistsView() {
  const { t } = useTranslation()
  const refreshPlaylists = usePlaylistStore((s) => s.refreshPlaylists)
  const playlists = usePlaylistStore((s) => s.playlists)
  const [selectedId, setSelectedId] = useState<number | null>(null)
  // Si no hay selección explícita todavía, se muestra la primera playlist disponible (derivado
  // en el render, no en un efecto, para no disparar un ciclo extra de renders).
  const effectiveSelectedId = selectedId ?? playlists[0]?.id ?? null

  useEffect(() => {
    void refreshPlaylists()
  }, [refreshPlaylists])

  return (
    <div className="flex h-full">
      <PlaylistList selectedId={effectiveSelectedId} onSelect={setSelectedId} />
      {effectiveSelectedId != null ? (
        <PlaylistDetail playlistId={effectiveSelectedId} />
      ) : (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-app-text-muted">
          <MusicNoteIcon className="h-10 w-10" />
          <p className="text-sm">{t('playlists.empty')}</p>
        </div>
      )}
    </div>
  )
}
