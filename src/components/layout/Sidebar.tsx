import { useTranslation } from 'react-i18next'
import type { ViewId } from '../../App'
import { MusicNoteIcon } from '../common/Icons'

interface SidebarProps {
  active: ViewId
  onSelect: (view: ViewId) => void
}

const NAV_ITEMS: { id: ViewId; labelKey: string }[] = [
  { id: 'library', labelKey: 'nav.library' },
  { id: 'playlists', labelKey: 'nav.playlists' },
  { id: 'equalizer', labelKey: 'nav.equalizer' },
  { id: 'settings', labelKey: 'nav.settings' },
]

export function Sidebar({ active, onSelect }: SidebarProps) {
  const { t } = useTranslation()

  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-app-border bg-app-surface">
      <div className="flex items-center gap-2 px-5 py-5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-app-accent text-white">
          <MusicNoteIcon className="h-4.5 w-4.5" />
        </div>
        <span className="truncate text-sm font-semibold text-app-text">{t('app.title')}</span>
      </div>

      <nav className="flex flex-col gap-1 px-3">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onSelect(item.id)}
            className={`rounded-lg px-3 py-2 text-left text-sm font-medium transition-colors ${
              active === item.id
                ? 'bg-app-accent text-white'
                : 'text-app-text-muted hover:bg-app-surface-hover hover:text-app-text'
            }`}
          >
            {t(item.labelKey)}
          </button>
        ))}
      </nav>
    </aside>
  )
}
