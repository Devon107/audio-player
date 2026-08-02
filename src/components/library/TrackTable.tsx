import { useTranslation } from 'react-i18next'
import { usePlayerStore } from '../../store/playerStore'
import { usePlaylistStore } from '../../store/playlistStore'
import { fromTrackRecord, toQueueInput } from '../../lib/displayTrack'
import { formatDuration } from '../../lib/format'
import type { TrackRecord } from '../../lib/types'
import { CoverArt } from '../common/CoverArt'
import { PauseIcon, PlayIcon } from '../common/Icons'
import { TrackRowMenu } from '../common/TrackRowMenu'

interface TrackTableProps {
  tracks: TrackRecord[]
  emptyMessage: string
}

export function TrackTable({ tracks, emptyMessage }: TrackTableProps) {
  const { t } = useTranslation()
  const currentPath = usePlayerStore(
    (s) => s.queue.items.find((i) => i.id === s.queue.current_id)?.path,
  )
  const isPlaying = usePlayerStore((s) => s.isPlaying)
  const playTracks = usePlayerStore((s) => s.playTracks)
  const addToQueue = usePlayerStore((s) => s.addToQueue)
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause)
  const addTracksToPlaylist = usePlaylistStore((s) => s.addTracks)

  if (tracks.length === 0) {
    return <p className="px-4 py-10 text-center text-sm text-app-text-muted">{emptyMessage}</p>
  }

  const allQueueInputs = tracks.map((track) => toQueueInput(fromTrackRecord(track)))

  return (
    <table className="w-full border-collapse text-left text-sm">
      <thead className="sticky top-0 z-10 bg-app-bg text-xs text-app-text-muted">
        <tr className="border-b border-app-border">
          <th className="w-12 px-3 py-2 font-medium" />
          <th className="px-2 py-2 font-medium">{t('library.columns.title')}</th>
          <th className="px-2 py-2 font-medium">{t('library.columns.artist')}</th>
          <th className="px-2 py-2 font-medium">{t('library.columns.album')}</th>
          <th className="px-2 py-2 font-medium">{t('library.columns.genre')}</th>
          <th className="px-2 py-2 text-right font-medium">{t('library.columns.duration')}</th>
          <th className="w-10 px-2 py-2" />
        </tr>
      </thead>
      <tbody>
        {tracks.map((track, index) => {
          const isCurrent = currentPath === track.path
          return (
            <tr
              key={track.id}
              onClick={() => {
                if (isCurrent) void togglePlayPause()
                else void playTracks(allQueueInputs, index)
              }}
              className={`group cursor-pointer border-b border-app-border/50 ${
                isCurrent ? 'bg-app-accent/10' : 'hover:bg-app-surface-hover'
              }`}
            >
              <td className="px-3 py-2">
                <div className="relative h-9 w-9">
                  <CoverArt
                    coverPath={track.cover_path}
                    alt={track.title}
                    className="h-9 w-9 rounded"
                    iconClassName="h-4 w-4"
                  />
                  {isCurrent && (
                    <span className="absolute inset-0 flex items-center justify-center rounded bg-black/50 text-app-accent">
                      {isPlaying ? (
                        <PauseIcon className="h-4 w-4" />
                      ) : (
                        <PlayIcon className="h-4 w-4" />
                      )}
                    </span>
                  )}
                </div>
              </td>
              <td className="max-w-56 truncate px-2 py-2">
                <span className={isCurrent ? 'font-medium text-app-accent' : 'text-app-text'}>
                  {track.title}
                </span>
              </td>
              <td className="max-w-40 truncate px-2 py-2 text-app-text-muted">
                {track.artist ?? t('common.unknownArtist')}
              </td>
              <td className="max-w-40 truncate px-2 py-2 text-app-text-muted">
                {track.album ?? t('common.unknownAlbum')}
              </td>
              <td className="max-w-32 truncate px-2 py-2 text-app-text-muted">
                {track.genre ?? '—'}
              </td>
              <td className="px-2 py-2 text-right tabular-nums text-app-text-muted">
                {formatDuration(track.duration_secs)}
              </td>
              <td className="px-2 py-2" onClick={(e) => e.stopPropagation()}>
                <TrackRowMenu
                  onPlayNow={() => void playTracks(allQueueInputs, index)}
                  onAddToQueue={() => void addToQueue([toQueueInput(fromTrackRecord(track))])}
                  onAddToPlaylist={(playlistId) => void addTracksToPlaylist(playlistId, [track.id])}
                />
              </td>
            </tr>
          )
        })}
      </tbody>
    </table>
  )
}
