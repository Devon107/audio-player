import { create } from 'zustand'
import { audio } from '../lib/tauri'
import type { EqPreset } from '../lib/types'

export const EQ_BAND_FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
export const EQ_MIN_GAIN_DB = -12
export const EQ_MAX_GAIN_DB = 12

// Al arrastrar un slider, `onChange` dispara muy seguido (varias veces por frame). Enviar cada
// evento al backend implica una escritura sincrónica a SQLite (persistencia del EQ), lo que
// genera lag notorio en el arrastre. Se debounce el envío real por banda, mientras el valor en
// pantalla se actualiza al instante para que el slider se sienta fluido.
const EQ_SEND_DEBOUNCE_MS = 120
const pendingSends: Record<number, ReturnType<typeof setTimeout>> = {}

interface EqualizerStore {
  gainsDb: number[]
  preset: EqPreset
  initialized: boolean

  init: () => Promise<void>
  setBandGain: (band: number, gainDb: number) => void
  applyPreset: (preset: Exclude<EqPreset, 'custom'>) => Promise<void>
}

export const useEqualizerStore = create<EqualizerStore>((set, get) => ({
  gainsDb: new Array(EQ_BAND_FREQUENCIES.length).fill(0),
  preset: 'flat',
  initialized: false,

  init: async () => {
    if (get().initialized) return
    set({ initialized: true })
    try {
      const state = await audio.getEqState()
      set({ gainsDb: state.gains_db, preset: state.preset })
    } catch {
      // Sin estado guardado todavía: se queda en "plano" por defecto.
    }
  },

  setBandGain: (band, gainDb) => {
    const clamped = Math.min(EQ_MAX_GAIN_DB, Math.max(EQ_MIN_GAIN_DB, gainDb))
    set((state) => {
      const gainsDb = [...state.gainsDb]
      gainsDb[band] = clamped
      return { gainsDb, preset: 'custom' }
    })

    clearTimeout(pendingSends[band])
    pendingSends[band] = setTimeout(() => {
      void audio.setEqBandGain(band, clamped)
    }, EQ_SEND_DEBOUNCE_MS)
  },

  applyPreset: async (preset) => {
    await audio.setEqPreset(preset)
    const state = await audio.getEqState()
    set({ gainsDb: state.gains_db, preset: state.preset })
  },
}))
