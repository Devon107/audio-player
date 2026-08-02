import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePlaylistStore } from '../../store/playlistStore'
import { PlusIcon } from '../common/Icons'

interface PlaylistListProps {
  selectedId: number | null
  onSelect: (id: number) => void
}

export function PlaylistList({ selectedId, onSelect }: PlaylistListProps) {
  const { t } = useTranslation()
  const playlists = usePlaylistStore((s) => s.playlists)
  const createPlaylist = usePlaylistStore((s) => s.createPlaylist)

  const [creating, setCreating] = useState(false)
  const [name, setName] = useState('')

  const submitCreate = async () => {
    const trimmed = name.trim()
    if (!trimmed) {
      setCreating(false)
      return
    }
    const id = await createPlaylist(trimmed)
    setName('')
    setCreating(false)
    onSelect(id)
  }

  return (
    <div className="flex w-64 shrink-0 flex-col border-r border-app-border">
      <div className="flex items-center justify-between border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-app-text">{t('playlists.title')}</h2>
        <button
          type="button"
          onClick={() => setCreating(true)}
          title={t('playlists.newPlaylist')}
          className="rounded p-1 text-app-text-muted hover:bg-app-surface-hover hover:text-app-text"
        >
          <PlusIcon className="h-4 w-4" />
        </button>
      </div>

      {creating && (
        <div className="border-b border-app-border px-3 py-2">
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submitCreate()
              if (e.key === 'Escape') {
                setCreating(false)
                setName('')
              }
            }}
            onBlur={() => void submitCreate()}
            placeholder={t('playlists.namePlaceholder')}
            className="w-full rounded border border-app-border bg-app-bg px-2 py-1 text-sm text-app-text focus:border-app-accent focus:outline-none"
          />
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {playlists.length === 0 && !creating ? (
          <p className="px-4 py-6 text-center text-sm text-app-text-muted">
            {t('playlists.empty')}
          </p>
        ) : (
          playlists.map((playlist) => (
            <button
              key={playlist.id}
              type="button"
              onClick={() => onSelect(playlist.id)}
              className={`block w-full truncate px-4 py-2.5 text-left text-sm ${
                selectedId === playlist.id
                  ? 'bg-app-accent/10 font-medium text-app-accent'
                  : 'text-app-text hover:bg-app-surface-hover'
              }`}
            >
              {playlist.name}
              <span className="ml-2 text-xs text-app-text-muted">{playlist.track_count}</span>
            </button>
          ))
        )}
      </div>
    </div>
  )
}
