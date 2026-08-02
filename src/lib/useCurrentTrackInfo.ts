import { useEffect, useState } from 'react'
import { metadata } from './tauri'
import { usePlayerStore } from '../store/playerStore'

export interface CurrentTrackInfo {
  title: string
  artist: string | null
  album: string | null
  coverPath: string | null
}

/// Resuelve título/artista/carátula de la pista actual leyendo el archivo directamente (no
/// depende de que la biblioteca esté cargada): así el reproductor siempre puede mostrar algo
/// razonable, incluso para archivos sueltos que no están en la biblioteca.
export function useCurrentTrackInfo(): CurrentTrackInfo | null {
  const currentPath = usePlayerStore((s) => {
    const currentId = s.queue.current_id
    return s.queue.items.find((item) => item.id === currentId)?.path ?? null
  })

  const [info, setInfo] = useState<CurrentTrackInfo | null>(null)

  useEffect(() => {
    let cancelled = false

    void (async () => {
      if (!currentPath) {
        if (!cancelled) setInfo(null)
        return
      }

      try {
        const meta = await metadata.readTrackMetadata(currentPath)
        const coverPath = meta.has_cover_art
          ? await metadata.getCoverArt(currentPath).catch(() => null)
          : null
        if (!cancelled) {
          setInfo({ title: meta.title, artist: meta.artist, album: meta.album, coverPath })
        }
      } catch {
        if (!cancelled) {
          setInfo({
            title: currentPath.split(/[/\\]/).pop() ?? currentPath,
            artist: null,
            album: null,
            coverPath: null,
          })
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [currentPath])

  return info
}
