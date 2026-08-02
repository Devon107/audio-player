import { useTranslation } from 'react-i18next'
import { usePlayerStore } from '../../store/playerStore'
import { useCurrentTrackInfo } from '../../lib/useCurrentTrackInfo'
import { CoverArt } from '../common/CoverArt'
import { ChevronLeftIcon, MusicNoteIcon } from '../common/Icons'

interface NowPlayingViewProps {
  onBack: () => void
}

export function NowPlayingView({ onBack }: NowPlayingViewProps) {
  const { t } = useTranslation()
  const hasTrack = usePlayerStore((s) => s.queue.current_id != null)
  const info = useCurrentTrackInfo()

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-app-border px-4 py-3">
        <button
          type="button"
          onClick={onBack}
          className="rounded p-1.5 text-app-text-muted hover:bg-app-surface-hover hover:text-app-text"
          aria-label={t('common.back')}
        >
          <ChevronLeftIcon className="h-4.5 w-4.5" />
        </button>
        <h1 className="text-sm font-semibold text-app-text">{t('player.nowPlaying')}</h1>
      </div>

      <div className="flex flex-1 items-center justify-center overflow-y-auto p-8">
        {!hasTrack ? (
          <div className="flex flex-col items-center gap-2 text-center">
            <MusicNoteIcon className="h-10 w-10 text-app-text-muted" />
            <p className="text-sm text-app-text-muted">{t('player.noTrack')}</p>
          </div>
        ) : (
          <div className="flex w-full max-w-sm flex-col items-center gap-6 text-center">
            <CoverArt
              coverPath={info?.coverPath}
              alt={info?.title ?? ''}
              className="aspect-square w-full rounded-2xl shadow-2xl"
              iconClassName="h-20 w-20"
            />
            <div className="min-w-0">
              <p className="truncate text-xl font-semibold text-app-text">{info?.title ?? '…'}</p>
              <p className="mt-1 truncate text-base text-app-text-muted">
                {info?.artist ?? t('common.unknownArtist')}
              </p>
              <p className="mt-4 truncate text-sm text-app-text-muted">
                {info?.album ?? t('common.unknownAlbum')} ·{' '}
                {info?.genre ?? t('common.unknownGenre')}
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
