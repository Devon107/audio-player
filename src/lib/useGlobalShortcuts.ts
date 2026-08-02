import { useEffect } from 'react'
import { usePlayerStore } from '../store/playerStore'

// Elementos donde las flechas/espacio tienen un significado nativo propio (escribir texto,
// mover el valor de un <select>/slider) que no debe pisarse con los atajos globales.
const TEXT_ENTRY_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT'])

/// Atajos de teclado globales del reproductor: espacio para play/pause, flechas izq/der para
/// pista anterior/siguiente. Se ignoran mientras el foco está en un campo de texto, un <select>
/// o un slider, para no interferir con su interacción nativa por teclado.
export function useGlobalShortcuts() {
  const togglePlayPause = usePlayerStore((s) => s.togglePlayPause)
  const next = usePlayerStore((s) => s.next)
  const previous = usePlayerStore((s) => s.previous)

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      if (target && (TEXT_ENTRY_TAGS.has(target.tagName) || target.isContentEditable)) return

      if (event.code === 'Space') {
        event.preventDefault()
        void togglePlayPause()
        return
      }

      if (event.code === 'ArrowRight') {
        if (usePlayerStore.getState().queue.current_id != null) void next()
        return
      }

      if (event.code === 'ArrowLeft') {
        const { queue } = usePlayerStore.getState()
        if (queue.current_id != null && queue.has_previous) void previous()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [togglePlayPause, next, previous])
}
