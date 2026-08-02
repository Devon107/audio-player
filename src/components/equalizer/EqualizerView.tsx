import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  EQ_BAND_FREQUENCIES,
  EQ_MAX_GAIN_DB,
  EQ_MIN_GAIN_DB,
  useEqualizerStore,
} from '../../store/equalizerStore'
import { rangeProgressStyle } from '../../lib/format'
import type { EqPreset } from '../../lib/types'

const NAMED_PRESETS: Exclude<EqPreset, 'custom'>[] = ['flat', 'rock', 'pop', 'jazz']

export function EqualizerView() {
  const { t } = useTranslation()
  const init = useEqualizerStore((s) => s.init)
  const gainsDb = useEqualizerStore((s) => s.gainsDb)
  const preset = useEqualizerStore((s) => s.preset)
  const setBandGain = useEqualizerStore((s) => s.setBandGain)
  const applyPreset = useEqualizerStore((s) => s.applyPreset)

  useEffect(() => {
    void init()
  }, [init])

  return (
    <div className="flex h-full flex-col overflow-y-auto p-6">
      <h1 className="mb-4 text-lg font-semibold text-app-text">{t('equalizer.title')}</h1>

      <div className="mb-8 flex flex-wrap gap-2">
        {NAMED_PRESETS.map((p) => (
          <button
            key={p}
            type="button"
            onClick={() => void applyPreset(p)}
            className={`rounded-full px-4 py-1.5 text-sm font-medium transition-colors ${
              preset === p
                ? 'bg-app-accent text-white'
                : 'border border-app-border text-app-text hover:bg-app-surface-hover'
            }`}
          >
            {t(`equalizer.presets.${p}`)}
          </button>
        ))}
        {preset === 'custom' && (
          <span className="rounded-full bg-app-surface-hover px-4 py-1.5 text-sm font-medium text-app-text-muted">
            {t('equalizer.presets.custom')}
          </span>
        )}
      </div>

      <div className="flex flex-1 items-end justify-center gap-6 rounded-2xl border border-app-border bg-app-surface p-8">
        {EQ_BAND_FREQUENCIES.map((freq, index) => (
          <BandSlider
            key={freq}
            frequency={freq}
            gainDb={gainsDb[index] ?? 0}
            onChange={(value) => void setBandGain(index, value)}
          />
        ))}
      </div>
    </div>
  )
}

function BandSlider({
  frequency,
  gainDb,
  onChange,
}: {
  frequency: number
  gainDb: number
  onChange: (value: number) => void
}) {
  const label = frequency >= 1000 ? `${frequency / 1000}k` : `${frequency}`

  return (
    <div className="flex flex-col items-center gap-2">
      <span className="w-10 text-center text-xs tabular-nums text-app-text-muted">
        {gainDb > 0 ? '+' : ''}
        {gainDb.toFixed(1)}
      </span>
      {/* Los navegadores no estilizan de forma consistente `-webkit-appearance: slider-vertical`
          (en WebKitGTK cae a un thumb blanco nativo que ignora el tema). En cambio, se rota -90°
          un slider horizontal normal, que sí respeta el estilo custom vía `::-webkit-slider-*`. */}
      <div className="flex h-40 w-6 items-center justify-center">
        <input
          type="range"
          min={EQ_MIN_GAIN_DB}
          max={EQ_MAX_GAIN_DB}
          step={0.5}
          value={gainDb}
          onChange={(e) => onChange(Number(e.target.value))}
          className="w-40"
          style={{
            transform: 'rotate(-90deg)',
            ...rangeProgressStyle(gainDb, EQ_MIN_GAIN_DB, EQ_MAX_GAIN_DB),
          }}
        />
      </div>
      <span className="text-xs text-app-text-muted">{label}</span>
    </div>
  )
}
