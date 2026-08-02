import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { confirm, open, save } from '@tauri-apps/plugin-dialog'
import { usePlaylistStore } from '../../store/playlistStore'
import { usePlayerStore } from '../../store/playerStore'
import { playlists as playlistsApi } from '../../lib/tauri'
import { fromPlaylistTrackRecord, toQueueInput } from '../../lib/displayTrack'
import { formatDuration } from '../../lib/format'
import type { PlaylistTrackRecord } from '../../lib/types'
import { CoverArt } from '../common/CoverArt'
import { DownloadIcon, GripIcon, PauseIcon, PlayIcon, TrashIcon, UploadIcon } from '../common/Icons'

interface PlaylistDetailProps {
  playlistId: number
}

export function PlaylistDetail({ playlistId }: PlaylistDetailProps) {
  const { t } = useTranslation()
  const playlists = usePlaylistStore((s) => s.playlists)
  const tracks = usePlaylistStore((s) => s.selectedPlaylistTracks)
  const selectPlaylist = usePlaylistStore((s) => s.selectPlaylist)
  const renamePlaylist = usePlaylistStore((s) => s.renamePlaylist)
  const deletePlaylist = usePlaylistStore((s) => s.deletePlaylist)
  const removeTrack = usePlaylistStore((s) => s.removeTrack)
  const reorderTrack = usePlaylistStore((s) => s.reorderTrack)
  const refreshPlaylists = usePlaylistStore((s) => s.refreshPlaylists)

  const playTracks = usePlayerStore((s) => s.playTracks)
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause)
  const currentPath = usePlayerStore(
    (s) => s.queue.items.find((i) => i.id === s.queue.current_id)?.path,
  )
  const isPlaying = usePlayerStore((s) => s.isPlaying)

  const playlist = playlists.find((p) => p.id === playlistId)
  const [draggedTrackId, setDraggedTrackId] = useState<number | null>(null)
  const [renaming, setRenaming] = useState(false)
  const [nameDraft, setNameDraft] = useState('')

  useEffect(() => {
    void selectPlaylist(playlistId)
  }, [playlistId, selectPlaylist])

  if (!playlist) return null

  const startRenaming = () => {
    setNameDraft(playlist.name)
    setRenaming(true)
  }

  const submitRename = async () => {
    const trimmed = nameDraft.trim()
    setRenaming(false)
    if (trimmed && trimmed !== playlist.name) {
      await renamePlaylist(playlistId, trimmed)
    } else {
      setNameDraft(playlist.name)
    }
  }

  const handleDelete = async () => {
    const confirmed = await confirm(t('common.confirmDeleteBody'), {
      title: t('common.confirmDeleteTitle', { name: playlist.name }),
      kind: 'warning',
    })
    if (confirmed) await deletePlaylist(playlistId)
  }

  const handlePlayAll = async () => {
    if (tracks.length === 0) return
    await playTracks(
      tracks.map((track) => toQueueInput(fromPlaylistTrackRecord(track))),
      0,
    )
  }

  const handleExport = async () => {
    const targetPath = await save({
      title: t('playlists.exportM3u'),
      defaultPath: `${playlist.name}.m3u`,
      filters: [{ name: 'M3U', extensions: ['m3u', 'm3u8'] }],
    })
    if (targetPath) await playlistsApi.exportM3u(playlistId, targetPath)
  }

  const handleImport = async () => {
    const sourcePath = await open({
      title: t('playlists.importM3u'),
      multiple: false,
      filters: [{ name: 'M3U', extensions: ['m3u', 'm3u8'] }],
    })
    if (typeof sourcePath === 'string') {
      await playlistsApi.importM3u(sourcePath, null)
      await refreshPlaylists()
    }
  }

  return (
    <div className="flex h-full flex-1 flex-col overflow-hidden">
      <div className="flex flex-wrap items-center gap-2 border-b border-app-border px-4 py-3">
        {renaming ? (
          <input
            autoFocus
            value={nameDraft}
            onChange={(e) => setNameDraft(e.target.value)}
            onBlur={() => void submitRename()}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void submitRename()
              if (e.key === 'Escape') {
                setRenaming(false)
                setNameDraft(playlist.name)
              }
            }}
            className="rounded border border-app-accent bg-app-bg px-2 py-1 text-lg font-semibold text-app-text focus:outline-none"
          />
        ) : (
          <h1
            onDoubleClick={startRenaming}
            title={t('common.rename')}
            className="cursor-text truncate text-lg font-semibold text-app-text"
          >
            {playlist.name}
          </h1>
        )}
        <span className="text-sm text-app-text-muted">
          {t('playlists.trackCount', { count: tracks.length })}
        </span>

        <div className="ml-auto flex items-center gap-2">
          <button
            type="button"
            onClick={() => void handlePlayAll()}
            disabled={tracks.length === 0}
            className="flex items-center gap-1.5 rounded-lg bg-app-accent px-3 py-1.5 text-sm font-medium text-white hover:bg-app-accent-hover disabled:opacity-40"
          >
            <PlayIcon className="h-3.5 w-3.5" />
            {t('playlists.playAll')}
          </button>
          <button
            type="button"
            onClick={() => void handleImport()}
            title={t('playlists.importM3u')}
            className="rounded-lg border border-app-border p-2 text-app-text-muted hover:bg-app-surface-hover hover:text-app-text"
          >
            <UploadIcon className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={() => void handleExport()}
            disabled={tracks.length === 0}
            title={t('playlists.exportM3u')}
            className="rounded-lg border border-app-border p-2 text-app-text-muted hover:bg-app-surface-hover hover:text-app-text disabled:opacity-40"
          >
            <DownloadIcon className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={() => void handleDelete()}
            title={t('common.delete')}
            className="rounded-lg border border-app-border p-2 text-app-text-muted hover:bg-red-500/10 hover:text-red-400"
          >
            <TrashIcon className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {tracks.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1 px-4 text-center">
            <p className="text-sm text-app-text">{t('playlists.emptyPlaylist')}</p>
            <p className="text-xs text-app-text-muted">{t('playlists.emptyPlaylistHint')}</p>
          </div>
        ) : (
          tracks.map((track, index) => (
            <PlaylistTrackRow
              key={track.track_id}
              track={track}
              isCurrent={currentPath === track.path}
              isPlaying={isPlaying}
              isDragging={draggedTrackId === track.track_id}
              onPlay={() => {
                if (currentPath === track.path) void togglePlayPause()
                else
                  void playTracks(
                    tracks.map((t) => toQueueInput(fromPlaylistTrackRecord(t))),
                    index,
                  )
              }}
              onRemove={() => void removeTrack(playlistId, track.track_id)}
              onDragStart={() => setDraggedTrackId(track.track_id)}
              onDragEnd={() => setDraggedTrackId(null)}
              onDropOn={() => {
                if (draggedTrackId != null && draggedTrackId !== track.track_id) {
                  void reorderTrack(playlistId, draggedTrackId, index)
                }
                setDraggedTrackId(null)
              }}
            />
          ))
        )}
      </div>
    </div>
  )
}

function PlaylistTrackRow({
  track,
  isCurrent,
  isPlaying,
  isDragging,
  onPlay,
  onRemove,
  onDragStart,
  onDragEnd,
  onDropOn,
}: {
  track: PlaylistTrackRecord
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

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDropOn}
      className={`group flex items-center gap-3 border-b border-app-border/50 px-4 py-2 ${
        isCurrent ? 'bg-app-accent/10' : 'hover:bg-app-surface-hover'
      } ${isDragging ? 'opacity-40' : ''}`}
    >
      <span className="cursor-grab text-app-text-muted opacity-0 group-hover:opacity-100">
        <GripIcon className="h-3.5 w-3.5" />
      </span>
      <button type="button" onClick={onPlay} className="relative h-9 w-9 shrink-0">
        <CoverArt
          coverPath={track.cover_path}
          alt={track.title}
          className="h-9 w-9 rounded"
          iconClassName="h-4 w-4"
        />
        <span className="absolute inset-0 flex items-center justify-center rounded bg-black/50 opacity-0 group-hover:opacity-100">
          {isCurrent && isPlaying ? (
            <PauseIcon className="h-4 w-4 text-white" />
          ) : (
            <PlayIcon className="h-4 w-4 text-white" />
          )}
        </span>
      </button>
      <button type="button" onClick={onPlay} className="min-w-0 flex-1 text-left">
        <p
          className={`truncate text-sm ${isCurrent ? 'font-medium text-app-accent' : 'text-app-text'}`}
        >
          {track.title}
        </p>
        <p className="truncate text-xs text-app-text-muted">
          {track.artist ?? t('common.unknownArtist')}
          {track.album ? ` · ${track.album}` : ''}
        </p>
      </button>
      <span className="text-xs tabular-nums text-app-text-muted">
        {formatDuration(track.duration_secs)}
      </span>
      <button
        type="button"
        onClick={onRemove}
        title={t('playlists.removeFromPlaylist')}
        className="text-app-text-muted opacity-0 hover:text-app-text group-hover:opacity-100"
      >
        <TrashIcon className="h-3.5 w-3.5" />
      </button>
    </div>
  )
}
