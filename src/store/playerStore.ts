import { create } from 'zustand'
import { audio, settings } from '../lib/tauri'
import {
  onPlayerError,
  onPlayerLoaded,
  onPlayerProgress,
  onPlayerQueueChanged,
  onPlayerTrackEnded,
} from '../lib/events'
import { volumePositionToGain } from '../lib/format'
import type { QueueSnapshot, QueueTrackInput, RepeatMode } from '../lib/types'

const EMPTY_QUEUE: QueueSnapshot = {
  items: [],
  current_id: null,
  shuffle: false,
  repeat: 'off',
  has_previous: false,
  duration_secs: null,
}

// Arrastrar el slider dispara `onChange` (y por lo tanto `setVolume`) muy seguido. La ganancia
// real se aplica al toque para que se sienta responsivo, pero guardar en SQLite en cada evento
// generaría el mismo lag que ya se vio con el ecualizador — así que solo la persistencia se
// debounce, no el cambio de volumen en sí.
const VOLUME_PERSIST_DEBOUNCE_MS = 300
let pendingVolumeSave: ReturnType<typeof setTimeout> | undefined

interface PlayerStore {
  queue: QueueSnapshot
  positionSecs: number
  durationSecs: number | null
  isPlaying: boolean
  /** Posición del slider de volumen, 0 a 1 (lineal). Ver `volumePositionToGain` para cómo se
   * convierte a la ganancia real que recibe el motor de audio. */
  volume: number
  lastError: string | null
  initialized: boolean

  init: () => Promise<void>
  playTracks: (items: QueueTrackInput[], startIndex?: number | null) => Promise<void>
  addToQueue: (items: QueueTrackInput[]) => Promise<void>
  playQueueItem: (itemId: number) => Promise<void>
  removeFromQueue: (itemId: number) => Promise<void>
  reorderQueue: (itemId: number, newIndex: number) => Promise<void>
  clearQueue: () => Promise<void>
  togglePlayPause: () => Promise<void>
  next: () => Promise<void>
  previous: () => Promise<void>
  seek: (positionSecs: number) => Promise<void>
  setVolume: (volume: number) => Promise<void>
  setShuffle: (enabled: boolean) => Promise<void>
  setRepeatMode: (mode: RepeatMode) => Promise<void>
  dismissError: () => void
}

export const usePlayerStore = create<PlayerStore>((set, get) => ({
  queue: EMPTY_QUEUE,
  positionSecs: 0,
  durationSecs: null,
  isPlaying: false,
  volume: 1,
  lastError: null,
  initialized: false,

  init: async () => {
    if (get().initialized) return
    set({ initialized: true })

    onPlayerLoaded((payload) => {
      set({ durationSecs: payload.duration_secs, positionSecs: 0 })
    })
    onPlayerProgress((payload) => {
      set({
        positionSecs: payload.position_secs,
        durationSecs: payload.duration_secs,
        isPlaying: payload.is_playing,
      })
    })
    onPlayerTrackEnded(() => {
      set({ isPlaying: false, positionSecs: 0 })
    })
    onPlayerQueueChanged((snapshot) => {
      set({ queue: snapshot, durationSecs: snapshot.duration_secs })
      if (snapshot.current_id == null) {
        set({ isPlaying: false, positionSecs: 0, durationSecs: null })
      }
    })
    onPlayerError((message) => {
      set({ lastError: message })
    })

    try {
      // Si el motor restauró una cola guardada al arrancar, esto ya trae la pista actual y su
      // duración — así la barra de reproducción muestra el último track escuchado, listo para
      // reproducir, sin esperar a que el usuario toque nada.
      const snapshot = await audio.getQueueState()
      set({ queue: snapshot, durationSecs: snapshot.duration_secs })
    } catch {
      // El motor de audio no respondió a tiempo: se sigue con la cola vacía por defecto.
    }

    try {
      const savedVolume = await settings.getVolume()
      if (savedVolume != null) {
        set({ volume: savedVolume })
        await audio.setVolume(volumePositionToGain(savedVolume))
      }
    } catch {
      // Sin volumen guardado todavía (o falló la lectura): se queda con el valor por defecto.
    }
  },

  playTracks: async (items, startIndex = 0) => {
    await audio.setQueue(items, startIndex ?? 0, true)
    set({ isPlaying: true })
  },

  addToQueue: async (items) => {
    await audio.addToQueue(items)
  },

  playQueueItem: async (itemId) => {
    await audio.playQueueItem(itemId)
    set({ isPlaying: true })
  },

  removeFromQueue: async (itemId) => {
    await audio.removeFromQueue(itemId)
  },

  reorderQueue: async (itemId, newIndex) => {
    await audio.reorderQueue(itemId, newIndex)
  },

  clearQueue: async () => {
    await audio.clearQueue()
    set({ isPlaying: false, positionSecs: 0, durationSecs: null })
  },

  togglePlayPause: async () => {
    const { isPlaying, queue } = get()
    if (queue.current_id == null) return
    if (isPlaying) {
      await audio.pause()
      set({ isPlaying: false })
    } else {
      await audio.play()
      set({ isPlaying: true })
    }
  },

  next: async () => {
    await audio.nextTrack()
    set({ isPlaying: true })
  },

  previous: async () => {
    await audio.previousTrack()
    set({ isPlaying: true })
  },

  seek: async (positionSecs) => {
    await audio.seek(positionSecs)
    set({ positionSecs })
  },

  setVolume: async (volume) => {
    set({ volume })
    await audio.setVolume(volumePositionToGain(volume))

    clearTimeout(pendingVolumeSave)
    pendingVolumeSave = setTimeout(() => {
      void settings.setVolume(volume)
    }, VOLUME_PERSIST_DEBOUNCE_MS)
  },

  setShuffle: async (enabled) => {
    await audio.setShuffle(enabled)
  },

  setRepeatMode: async (mode) => {
    await audio.setRepeatMode(mode)
  },

  dismissError: () => set({ lastError: null }),
}))
