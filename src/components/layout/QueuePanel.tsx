import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePlayerStore } from '../../store/playerStore'
import { useLibraryStore } from '../../store/libraryStore'
import type { QueueTrack } from '../../lib/types'
import { CloseIcon, GripIcon, PauseIcon, PlayIcon, TrashIcon } from '../common/Icons'

interface QueuePanelProps {
  onClose: () => void
}

export function QueuePanel({ onClose }: QueuePanelProps) {
  const { t } = useTranslation()
  const queue = usePlayerStore((s) => s.queue)
  const isPlaying = usePlayerStore((s) => s.isPlaying)
  const playQueueItem = usePlayerStore((s) => s.playQueueItem)
  const removeFromQueue = usePlayerStore((s) => s.removeFromQueue)
  const reorderQueue = usePlayerStore((s) => s.reorderQueue)
  const clearQueue = usePlayerStore((s) => s.clearQueue)
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause)

  const [draggedId, setDraggedId] = useState<number | null>(null)

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-app-border bg-app-surface">
      <div className="flex items-center justify-between border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-app-text">{t('player.queue')}</h2>
        <div className="flex items-center gap-2">
          {queue.items.length > 0 && (
            <button
              type="button"
              onClick={() => void clearQueue()}
              className="text-xs text-app-text-muted hover:text-app-text"
            >
              {t('player.clearQueue')}
            </button>
          )}
          <button
            type="button"
            onClick={onClose}
            className="text-app-text-muted hover:text-app-text"
          >
            <CloseIcon className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {queue.items.length === 0 ? (
          <p className="px-4 py-6 text-center text-sm text-app-text-muted">
            {t('player.queueEmpty')}
          </p>
        ) : (
          queue.items.map((item) => {
            const isCurrent = item.id === queue.current_id
            return (
              <QueueRow
                key={item.id}
                item={item}
                isCurrent={isCurrent}
                isPlaying={isCurrent && isPlaying}
                isDragging={draggedId === item.id}
                onPlay={() => {
                  if (isCurrent) void togglePlayPause()
                  else void playQueueItem(item.id)
                }}
                onRemove={() => void removeFromQueue(item.id)}
                onDragStart={() => setDraggedId(item.id)}
                onDragEnd={() => setDraggedId(null)}
                onDropOn={() => {
                  if (draggedId != null && draggedId !== item.id) {
                    const targetIndex = queue.items.findIndex((i) => i.id === item.id)
                    void reorderQueue(draggedId, targetIndex)
                  }
                  setDraggedId(null)
                }}
              />
            )
          })
        )}
      </div>
    </aside>
  )
}

function QueueRow({
  item,
  isCurrent,
  isPlaying,
  isDragging,
  onPlay,
  onRemove,
  onDragStart,
  onDragEnd,
  onDropOn,
}: {
  item: QueueTrack
  isCurrent: boolean
  isPlaying: boolean
  isDragging: boolean
  onPlay: () => void
  onRemove: () => void
  onDragStart: () => void
  onDragEnd: () => void
  onDropOn: () => void
}) {
  const { t } = useTranslation()
  const display = useQueueItemDisplay(item)

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDropOn}
      className={`group flex items-center gap-2 border-b border-app-border/60 px-3 py-2 ${
        isCurrent ? 'bg-app-accent/10' : 'hover:bg-app-surface-hover'
      } ${isDragging ? 'opacity-40' : ''}`}
    >
      <span className="cursor-grab text-app-text-muted opacity-0 group-hover:opacity-100">
        <GripIcon className="h-3.5 w-3.5" />
      </span>
      <button
        type="button"
        onClick={onPlay}
        className="flex min-w-0 flex-1 items-center gap-2 text-left"
      >
        <span className="flex h-6 w-6 shrink-0 items-center justify-center text-app-text-muted">
          {isCurrent ? (
            isPlaying ? (
              <PauseIcon className="h-3.5 w-3.5 text-app-accent" />
            ) : (
              <PlayIcon className="h-3.5 w-3.5 text-app-accent" />
            )
          ) : (
            <PlayIcon className="h-3.5 w-3.5 opacity-0 group-hover:opacity-100" />
          )}
        </span>
        <span className="min-w-0 flex-1">
          <p
            className={`truncate text-sm ${isCurrent ? 'font-medium text-app-accent' : 'text-app-text'}`}
          >
            {display.title}
          </p>
          {display.artist && (
            <p className="truncate text-xs text-app-text-muted">{display.artist}</p>
          )}
        </span>
      </button>
      <button
        type="button"
        onClick={onRemove}
        title={t('common.remove')}
        className="text-app-text-muted opacity-0 hover:text-app-text group-hover:opacity-100"
      >
        <TrashIcon className="h-3.5 w-3.5" />
      </button>
    </div>
  )
}

function useQueueItemDisplay(item: QueueTrack): { title: string; artist: string | null } {
  const tracks = useLibraryStore((s) => s.tracks)

  return useMemo(() => {
    const match = item.track_id != null ? tracks.find((t) => t.id === item.track_id) : undefined
    if (match) return { title: match.title, artist: match.artist }
    const filename = item.path.split(/[/\\]/).pop() ?? item.path
    return { title: filename, artist: null }
  }, [item.path, item.track_id, tracks])
}
