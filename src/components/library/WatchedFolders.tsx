import { useTranslation } from 'react-i18next'
import { useLibraryStore } from '../../store/libraryStore'
import { CloseIcon, FolderIcon } from '../common/Icons'

export function WatchedFolders() {
  const { t } = useTranslation()
  const folders = useLibraryStore((s) => s.watchedFolders)
  const removeFolder = useLibraryStore((s) => s.removeFolder)

  if (folders.length === 0) return null

  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-app-border px-4 py-2">
      <span className="text-xs font-medium text-app-text-muted">
        {t('library.watchedFolders')}:
      </span>
      {folders.map((folder) => (
        <span
          key={folder}
          className="flex items-center gap-1.5 rounded-full border border-app-border bg-app-surface-hover px-2.5 py-1 text-xs text-app-text"
          title={folder}
        >
          <FolderIcon className="h-3 w-3 shrink-0 text-app-text-muted" />
          <span className="max-w-48 truncate">{folder}</span>
          <button
            type="button"
            onClick={() => void removeFolder(folder)}
            className="text-app-text-muted hover:text-app-text"
          >
            <CloseIcon className="h-3 w-3" />
          </button>
        </span>
      ))}
    </div>
  )
}
