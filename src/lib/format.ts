import type { CSSProperties } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'

export function formatDuration(totalSeconds: number | null | undefined): string {
  if (totalSeconds == null || !Number.isFinite(totalSeconds) || totalSeconds < 0) {
    return '--:--'
  }

  const seconds = Math.floor(totalSeconds)
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
  }
  return `${minutes}:${String(secs).padStart(2, '0')}`
}

export function coverAssetUrl(coverPath: string | null | undefined): string | null {
  if (!coverPath) return null
  return convertFileSrc(coverPath)
}

/// Trunca texto a `maxLength` caracteres, agregando "…" si lo corta. Los `<option>` nativos de
/// un `<select>` no respetan `text-overflow: ellipsis` (limitación del control nativo, no algo
/// que se pueda arreglar solo con CSS) — así que con nombres muy largos (artista/álbum/género)
/// el popup desplegado se sale de la pantalla a menos que el texto se corte a mano antes de
/// renderizarlo como opción.
export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text
  return `${text.slice(0, maxLength - 1).trimEnd()}…`
}

/// El backend aplica la ganancia de volumen linealmente (multiplica cada muestra), pero el oído
/// humano percibe el volumen logarítmicamente: con una ganancia lineal, un slider 0-1 se siente
/// "sin cambios" en su primer tramo y luego se dispara de golpe cerca del máximo. Una curva
/// cuadrática (aproximación estándar de "audio taper") hace que la posición del slider se sienta
/// aproximadamente lineal con el volumen percibido. Se probó primero con una curva cúbica
/// (posición³), pero resultó demasiado agresiva — a 50% el volumen real caía a 12.5% de la
/// ganancia máxima, notablemente más bajo que la referencia (p. ej. YouTube Music) al mismo
/// punto del slider. Con el cuadrado, 50% cae a 25%, más cerca de lo esperado.
export function volumePositionToGain(position: number): number {
  const clamped = Math.min(1, Math.max(0, position))
  return clamped ** 2
}

/// Estilo inline con la variable CSS que pinta de color de acento el tramo ya "recorrido" de un
/// `<input type="range">` (ver `--range-progress` en index.css).
export function rangeProgressStyle(value: number, min: number, max: number): CSSProperties {
  const percent = max > min ? ((value - min) / (max - min)) * 100 : 0
  return { '--range-progress': `${Math.min(100, Math.max(0, percent))}%` } as CSSProperties
}
