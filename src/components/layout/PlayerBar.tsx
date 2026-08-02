import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { usePlayerStore } from '../../store/playerStore'
import { useCurrentTrackInfo } from '../../lib/useCurrentTrackInfo'
import { formatDuration, rangeProgressStyle } from '../../lib/format'
import { CoverArt } from '../common/CoverArt'
import {
  ListIcon,
  NextIcon,
  PauseIcon,
  PlayIcon,
  PreviousIcon,
  RepeatIcon,
  RepeatOneIcon,
  ShuffleIcon,
  VolumeIcon,
  VolumeMuteIcon,
} from '../common/Icons'
import type { RepeatMode } from '../../lib/types'

const REPEAT_CYCLE: RepeatMode[] = ['off', 'queue', 'track']

interface PlayerBarProps {
  onToggleQueue: () => void
  queueOpen: boolean
}

export function PlayerBar({ onToggleQueue, queueOpen }: PlayerBarProps) {
  const { t } = useTranslation()
  const queue = usePlayerStore((s) => s.queue)
  const isPlaying = usePlayerStore((s) => s.isPlaying)
  const positionSecs = usePlayerStore((s) => s.positionSecs)
  const durationSecs = usePlayerStore((s) => s.durationSecs)
  const volume = usePlayerStore((s) => s.volume)
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause)
  const next = usePlayerStore((s) => s.next)
  const previous = usePlayerStore((s) => s.previous)
  const seek = usePlayerStore((s) => s.seek)
  const setVolume = usePlayerStore((s) => s.setVolume)
  const setShuffle = usePlayerStore((s) => s.setShuffle)
  const setRepeatMode = usePlayerStore((s) => s.setRepeatMode)

  const [seekValue, setSeekValue] = useState(0)
  const [isSeeking, setIsSeeking] = useState(false)

  const info = useCurrentTrackInfo()
  const hasTrack = queue.current_id != null
  const displayPosition = isSeeking ? seekValue : positionSecs

  const cycleRepeat = () => {
    const nextIndex = (REPEAT_CYCLE.indexOf(queue.repeat) + 1) % REPEAT_CYCLE.length
    void setRepeatMode(REPEAT_CYCLE[nextIndex])
  }

  return (
    <footer className="flex h-20 shrink-0 items-center gap-4 border-t border-app-border bg-app-surface px-4">
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <CoverArt
          coverPath={info?.coverPath}
          alt={info?.title ?? ''}
          className="h-12 w-12 shrink-0 rounded-md"
          iconClassName="h-5 w-5"
        />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-app-text">
            {hasTrack ? (info?.title ?? '…') : t('player.noTrack')}
          </p>
          <p className="truncate text-xs text-app-text-muted">
            {info?.artist ?? (hasTrack ? t('common.unknownArtist') : '')}
          </p>
        </div>
      </div>

      <div className="flex w-full max-w-xl flex-1 flex-col items-center gap-1">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void setShuffle(!queue.shuffle)}
            title={t('player.shuffle')}
            className={`rounded p-1.5 transition-colors ${queue.shuffle ? 'text-app-accent' : 'text-app-text-muted hover:text-app-text'}`}
          >
            <ShuffleIcon className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={previous}
            disabled={!hasTrack || !queue.has_previous}
            title={t('player.previous')}
            className="rounded p-1.5 text-app-text hover:text-app-accent disabled:opacity-30"
          >
            <PreviousIcon className="h-5 w-5" />
          </button>
          <button
            type="button"
            onClick={() => void togglePlayPause()}
            disabled={!hasTrack}
            title={isPlaying ? t('player.pause') : t('player.play')}
            className="flex h-9 w-9 items-center justify-center rounded-full bg-app-accent text-white transition-colors hover:bg-app-accent-hover disabled:opacity-40"
          >
            {isPlaying ? (
              <PauseIcon className="h-4.5 w-4.5" />
            ) : (
              <PlayIcon className="h-4.5 w-4.5 pl-0.5" />
            )}
          </button>
          <button
            type="button"
            onClick={next}
            disabled={!hasTrack}
            title={t('player.next')}
            className="rounded p-1.5 text-app-text hover:text-app-accent disabled:opacity-30"
          >
            <NextIcon className="h-5 w-5" />
          </button>
          <button
            type="button"
            onClick={cycleRepeat}
            title={
              queue.repeat === 'off'
                ? t('player.repeatOff')
                : queue.repeat === 'track'
                  ? t('player.repeatTrack')
                  : t('player.repeatQueue')
            }
            className={`rounded p-1.5 transition-colors ${queue.repeat !== 'off' ? 'text-app-accent' : 'text-app-text-muted hover:text-app-text'}`}
          >
            {queue.repeat === 'track' ? (
              <RepeatOneIcon className="h-4 w-4" />
            ) : (
              <RepeatIcon className="h-4 w-4" />
            )}
          </button>
        </div>

        <div className="flex w-full items-center gap-2">
          <span className="w-10 shrink-0 text-right text-[11px] tabular-nums text-app-text-muted">
            {formatDuration(displayPosition)}
          </span>
          <input
            type="range"
            min={0}
            max={durationSecs || 0}
            step={0.1}
            value={displayPosition}
            disabled={!hasTrack}
            onChange={(e) => {
              setIsSeeking(true)
              setSeekValue(Number(e.target.value))
            }}
            onPointerUp={() => {
              if (isSeeking) {
                void seek(seekValue)
                setIsSeeking(false)
              }
            }}
            style={rangeProgressStyle(displayPosition, 0, durationSecs || 0)}
            className="h-4 flex-1"
          />
          <span className="w-10 shrink-0 text-[11px] tabular-nums text-app-text-muted">
            {formatDuration(durationSecs)}
          </span>
        </div>
      </div>

      <div className="flex flex-1 items-center justify-end gap-3">
        <VolumeControl volume={volume} onChange={(v) => void setVolume(v)} />
        <button
          type="button"
          onClick={onToggleQueue}
          title={t('player.queue')}
          className={`rounded p-1.5 transition-colors ${queueOpen ? 'text-app-accent' : 'text-app-text-muted hover:text-app-text'}`}
        >
          <ListIcon className="h-4.5 w-4.5" />
        </button>
      </div>
    </footer>
  )
}

function VolumeControl({ volume, onChange }: { volume: number; onChange: (v: number) => void }) {
  const [previousVolume, setPreviousVolume] = useState(1)

  const toggleMute = () => {
    if (volume > 0) {
      setPreviousVolume(volume)
      onChange(0)
    } else {
      onChange(previousVolume || 1)
    }
  }

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={toggleMute}
        className="text-app-text-muted hover:text-app-text"
      >
        {volume > 0 ? (
          <VolumeIcon className="h-4.5 w-4.5" />
        ) : (
          <VolumeMuteIcon className="h-4.5 w-4.5" />
        )}
      </button>
      <input
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={volume}
        onChange={(e) => onChange(Number(e.target.value))}
        style={rangeProgressStyle(volume, 0, 1)}
        className="h-1 w-20"
      />
    </div>
  )
}
