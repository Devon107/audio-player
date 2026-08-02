import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import en from './locales/en.json'
import es from './locales/es.json'
import { settings } from '../lib/tauri'

export const SUPPORTED_LANGUAGES = ['en', 'es'] as const
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number]

function detectSystemLanguage(): SupportedLanguage {
  const browserLang = navigator.language?.slice(0, 2).toLowerCase()
  return browserLang === 'es' ? 'es' : 'en'
}

// Arranca con el idioma del sistema como mejor estimación síncrona (i18next necesita un idioma
// inicial de inmediato); si el usuario ya había elegido uno explícitamente, se aplica después de
// leerlo del backend (ver `initI18n`) para no bloquear el primer render con una llamada async.
void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    es: { translation: es },
  },
  lng: detectSystemLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
})

/// Aplica la preferencia de idioma guardada en el backend (Fase 7), si existe. Se llama una vez
/// al arrancar la app.
export async function initI18n(): Promise<void> {
  try {
    const saved = await settings.getLanguage()
    if (saved && SUPPORTED_LANGUAGES.includes(saved as SupportedLanguage)) {
      await i18n.changeLanguage(saved)
    }
  } catch {
    // Sin preferencia guardada o backend no disponible todavía: se queda con el idioma del
    // sistema ya aplicado en la inicialización síncrona de arriba.
  }
}

export async function changeLanguage(language: SupportedLanguage): Promise<void> {
  await i18n.changeLanguage(language)
  try {
    await settings.setLanguage(language)
  } catch {
    // La preferencia no se pudo persistir; el idioma igual queda aplicado en esta sesión.
  }
}

export default i18n
