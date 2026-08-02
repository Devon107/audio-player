import { useTranslation } from 'react-i18next'
import { changeLanguage, SUPPORTED_LANGUAGES } from '../../i18n'

export function SettingsView() {
  const { t, i18n } = useTranslation()

  return (
    <div className="h-full overflow-y-auto p-6">
      <h1 className="mb-6 text-lg font-semibold text-app-text">{t('settings.title')}</h1>

      <div className="max-w-sm">
        <label className="mb-2 block text-sm font-medium text-app-text">
          {t('settings.language')}
        </label>
        <div className="flex gap-2">
          {SUPPORTED_LANGUAGES.map((lang) => (
            <button
              key={lang}
              type="button"
              onClick={() => void changeLanguage(lang)}
              className={`flex-1 rounded-lg border px-4 py-2 text-sm font-medium transition-colors ${
                i18n.language === lang
                  ? 'border-app-accent bg-app-accent/10 text-app-accent'
                  : 'border-app-border text-app-text hover:bg-app-surface-hover'
              }`}
            >
              {t(lang === 'en' ? 'settings.languageEnglish' : 'settings.languageSpanish')}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
