import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePlaylistStore } from '../../store/playlistStore'
import { ChevronDownIcon, ListIcon, MoreIcon, PlayIcon, PlusIcon } from './Icons'

interface TrackRowMenuProps {
  onPlayNow: () => void
  onAddToQueue: () => void
  onAddToPlaylist: (playlistId: number) => void
}

export function TrackRowMenu({ onPlayNow, onAddToQueue, onAddToPlaylist }: TrackRowMenuProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [showPlaylists, setShowPlaylists] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const playlists = usePlaylistStore((s) => s.playlists)
  const refreshPlaylists = usePlaylistStore((s) => s.refreshPlaylists)

  useEffect(() => {
    if (!open) return
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false)
        setShowPlaylists(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [open])

  useEffect(() => {
    if (open) void refreshPlaylists()
  }, [open, refreshPlaylists])

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation()
          setOpen((v) => !v)
        }}
        className="rounded p-1.5 text-app-text-muted hover:bg-app-surface-hover hover:text-app-text"
        aria-label="Menu"
      >
        <MoreIcon className="h-4 w-4" />
      </button>

      {open && (
        <div
          className="absolute right-0 z-20 mt-1 w-56 overflow-hidden rounded-lg border border-app-border bg-app-surface shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          <MenuButton
            icon={<PlayIcon className="h-4 w-4" />}
            label={t('library.playNow')}
            onClick={() => {
              onPlayNow()
              setOpen(false)
            }}
          />
          <MenuButton
            icon={<ListIcon className="h-4 w-4" />}
            label={t('library.addToQueue')}
            onClick={() => {
              onAddToQueue()
              setOpen(false)
            }}
          />
          <button
            type="button"
            onClick={() => setShowPlaylists((v) => !v)}
            className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-app-text hover:bg-app-surface-hover"
          >
            <PlusIcon className="h-4 w-4" />
            <span className="flex-1">{t('library.addToPlaylist')}</span>
            <ChevronDownIcon
              className={`h-3.5 w-3.5 transition-transform ${showPlaylists ? 'rotate-180' : ''}`}
            />
          </button>
          {showPlaylists && (
            <div className="max-h-48 overflow-y-auto border-t border-app-border bg-app-bg/40">
              {playlists.length === 0 ? (
                <p className="px-3 py-2 text-xs text-app-text-muted">{t('playlists.empty')}</p>
              ) : (
                playlists.map((playlist) => (
                  <button
                    key={playlist.id}
                    type="button"
                    onClick={() => {
                      onAddToPlaylist(playlist.id)
                      setOpen(false)
                      setShowPlaylists(false)
                    }}
                    className="block w-full truncate px-4 py-1.5 text-left text-sm text-app-text hover:bg-app-surface-hover"
                  >
                    {playlist.name}
                  </button>
                ))
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function MenuButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode
  label: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-app-text hover:bg-app-surface-hover"
    >
      {icon}
      {label}
    </button>
  )
}
