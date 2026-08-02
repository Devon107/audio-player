use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};
use serde::{Deserialize, Serialize};

/// Ecualizador gráfico de 10 bandas (frecuencias ISO estándar, ~1 octava entre bandas).
pub const NUM_BANDS: usize = 10;
pub const BAND_FREQUENCIES: [f32; NUM_BANDS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub const MIN_GAIN_DB: f32 = -12.0;
pub const MAX_GAIN_DB: f32 = 12.0;

/// Factor de calidad de cada banda: ~1 octava de ancho, un valor típico para ecualizadores
/// gráficos con bandas espaciadas así.
const BAND_Q: f32 = std::f32::consts::SQRT_2;

#[derive(Clone, Copy, Default)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    /// Filtro "peaking EQ" según el Audio EQ Cookbook (Robert Bristow-Johnson). A `gain_db == 0`
    /// el filtro es analíticamente la identidad (b0/a0 = 1, b1/a0 = a1/a0, b2/a0 = a2/a0).
    fn peaking(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    #[inline]
    fn process(&mut self, c: &BiquadCoeffs, x0: f32) -> f32 {
        let y0 = c.b0 * x0 + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }
}

/// Asa compartida y clonable (barata: solo `Arc` + atómicos) con las ganancias actuales del
/// ecualizador. Se lee sin bloquear desde el hilo de audio en tiempo real, y se escribe desde los
/// comandos Tauri sin pasar por el canal de comandos del motor de audio.
#[derive(Clone)]
pub struct EqualizerControl {
    gains_centidb: Arc<[AtomicI32; NUM_BANDS]>,
}

impl EqualizerControl {
    pub fn new() -> Self {
        Self {
            gains_centidb: Arc::new(std::array::from_fn(|_| AtomicI32::new(0))),
        }
    }

    pub fn set_gain(&self, band: usize, gain_db: f32) {
        if let Some(slot) = self.gains_centidb.get(band) {
            let clamped = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
            slot.store((clamped * 100.0).round() as i32, Ordering::Relaxed);
        }
    }

    pub fn set_gains(&self, gains_db: &[f32; NUM_BANDS]) {
        for (band, gain) in gains_db.iter().enumerate() {
            self.set_gain(band, *gain);
        }
    }

    pub fn gains_db(&self) -> [f32; NUM_BANDS] {
        std::array::from_fn(|i| self.gains_centidb[i].load(Ordering::Relaxed) as f32 / 100.0)
    }
}

impl Default for EqualizerControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Envuelve una fuente de audio `f32` aplicándole las 10 bandas del ecualizador en cadena, con
/// estado de filtro independiente por canal (para no mezclar izquierda/derecha). Revisa si las
/// ganancias cambiaron una vez por frame (no por muestra) para no golpear los atómicos
/// innecesariamente.
pub struct EqualizerSource<S: Source<Item = f32>> {
    inner: S,
    control: EqualizerControl,
    sample_rate: f32,
    channels: usize,
    channel_index: usize,
    coeffs: [BiquadCoeffs; NUM_BANDS],
    cached_gains_centidb: [i32; NUM_BANDS],
    states: Vec<[BiquadState; NUM_BANDS]>,
}

impl<S: Source<Item = f32>> EqualizerSource<S> {
    pub fn new(inner: S, control: EqualizerControl) -> Self {
        let sample_rate = inner.sample_rate().get() as f32;
        let channels = inner.channels().get() as usize;
        let cached_gains_centidb =
            std::array::from_fn(|i| (control.gains_db()[i] * 100.0).round() as i32);
        let coeffs = Self::compute_coeffs(sample_rate, &cached_gains_centidb);

        Self {
            inner,
            control,
            sample_rate,
            channels,
            channel_index: 0,
            coeffs,
            cached_gains_centidb,
            states: vec![[BiquadState::default(); NUM_BANDS]; channels.max(1)],
        }
    }

    fn compute_coeffs(
        sample_rate: f32,
        gains_centidb: &[i32; NUM_BANDS],
    ) -> [BiquadCoeffs; NUM_BANDS] {
        std::array::from_fn(|i| {
            let gain_db = gains_centidb[i] as f32 / 100.0;
            BiquadCoeffs::peaking(sample_rate, BAND_FREQUENCIES[i], gain_db, BAND_Q)
        })
    }

    fn current_gains_centidb(&self) -> [i32; NUM_BANDS] {
        std::array::from_fn(|i| (self.control.gains_db()[i] * 100.0).round() as i32)
    }
}

impl<S: Source<Item = f32>> Iterator for EqualizerSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.channel_index == 0 {
            let current = self.current_gains_centidb();
            if current != self.cached_gains_centidb {
                self.coeffs = Self::compute_coeffs(self.sample_rate, &current);
                self.cached_gains_centidb = current;
            }
        }

        let sample = self.inner.next()?;
        let channel = self.channel_index;
        self.channel_index = (self.channel_index + 1) % self.channels.max(1);

        let state = &mut self.states[channel];
        let mut x = sample;
        for (band_state, coeffs) in state.iter_mut().zip(self.coeffs.iter()) {
            x = band_state.process(coeffs, x);
        }
        Some(x)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<S: Source<Item = f32>> Source for EqualizerSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let result = self.inner.try_seek(pos);
        if result.is_ok() {
            // Evita clics/artefactos causados por arrastrar estado de filtro de antes del salto.
            for state in &mut self.states {
                *state = [BiquadState::default(); NUM_BANDS];
            }
            self.channel_index = 0;
        }
        result
    }
}

/// Presets predefinidos. `Custom` no tiene curva propia: representa que el usuario ajustó las
/// bandas a mano y ya no coinciden con ninguno de los presets con nombre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqPreset {
    Flat,
    Rock,
    Pop,
    Jazz,
    Custom,
}

impl EqPreset {
    /// Curva de ganancias (dB) para cada banda, en el mismo orden que `BAND_FREQUENCIES`.
    /// `Custom` no tiene curva propia: llamar con `Custom` no debería usarse para aplicar
    /// ganancias, solo para etiquetar el estado actual en la UI.
    pub fn gains_db(self) -> Option<[f32; NUM_BANDS]> {
        match self {
            EqPreset::Flat => Some([0.0; NUM_BANDS]),
            EqPreset::Rock => Some([4.0, 3.0, -1.0, -2.0, -1.0, 1.0, 3.0, 4.0, 4.0, 4.0]),
            EqPreset::Pop => Some([-1.0, 1.0, 3.0, 3.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0]),
            EqPreset::Jazz => Some([2.0, 1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            EqPreset::Custom => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EqStateSnapshot {
    pub gains_db: [f32; NUM_BANDS],
    pub preset: EqPreset,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44_100.0;

    /// Magnitud analítica |H(e^{jw})| del biquad a una frecuencia dada, evaluando la función de
    /// transferencia directamente (sin necesidad de simular audio ni depender de una crate de
    /// números complejos).
    fn magnitude_at(c: &BiquadCoeffs, sample_rate: f32, freq: f32) -> f32 {
        let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();

        let num_re = c.b0 + c.b1 * c1 + c.b2 * c2;
        let num_im = -(c.b1 * s1 + c.b2 * s2);
        let den_re = 1.0 + c.a1 * c1 + c.a2 * c2;
        let den_im = -(c.a1 * s1 + c.a2 * s2);

        (num_re * num_re + num_im * num_im).sqrt() / (den_re * den_re + den_im * den_im).sqrt()
    }

    #[test]
    fn peaking_at_zero_db_is_the_identity_filter() {
        let coeffs = BiquadCoeffs::peaking(SAMPLE_RATE, 1000.0, 0.0, BAND_Q);
        let mut state = BiquadState::default();

        // Señal pseudoaleatoria simple, suficiente para ejercitar el estado del filtro.
        let mut seed = 12345u32;
        for _ in 0..2000 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let x = ((seed >> 8) as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let y = state.process(&coeffs, x);
            assert!(
                (y - x).abs() < 1e-4,
                "a 0dB la salida debería ser ~igual a la entrada"
            );
        }
    }

    #[test]
    fn peaking_boosts_center_frequency_more_than_distant_frequencies() {
        let center = 1000.0;
        let coeffs = BiquadCoeffs::peaking(SAMPLE_RATE, center, 6.0, BAND_Q);

        let mag_center = magnitude_at(&coeffs, SAMPLE_RATE, center);
        let mag_low = magnitude_at(&coeffs, SAMPLE_RATE, 50.0);
        let mag_high = magnitude_at(&coeffs, SAMPLE_RATE, 18_000.0);

        // +6dB de ganancia ~ factor 2x en amplitud en el centro de la banda.
        assert!(
            (mag_center - 1.995).abs() < 0.05,
            "se esperaba ~+6dB (x2) en el centro, se obtuvo {mag_center}"
        );
        assert!(
            mag_low < 1.1,
            "no debería haber boost apreciable lejos de la banda (grave)"
        );
        assert!(
            mag_high < 1.1,
            "no debería haber boost apreciable lejos de la banda (agudo)"
        );
    }

    #[test]
    fn cutting_reduces_magnitude_at_center() {
        let coeffs = BiquadCoeffs::peaking(SAMPLE_RATE, 1000.0, -6.0, BAND_Q);
        let mag_center = magnitude_at(&coeffs, SAMPLE_RATE, 1000.0);
        assert!(
            (mag_center - 0.501).abs() < 0.03,
            "se esperaba ~-6dB (x0.5) en el centro, se obtuvo {mag_center}"
        );
    }

    #[test]
    fn equalizer_control_clamps_gain_to_valid_range() {
        let control = EqualizerControl::new();
        control.set_gain(0, 999.0);
        control.set_gain(1, -999.0);
        let gains = control.gains_db();
        assert_eq!(gains[0], MAX_GAIN_DB);
        assert_eq!(gains[1], MIN_GAIN_DB);
    }

    #[test]
    fn equalizer_control_round_trips_gains() {
        let control = EqualizerControl::new();
        let mut gains = [0.0; NUM_BANDS];
        gains[3] = 5.5;
        gains[7] = -3.25;
        control.set_gains(&gains);

        let read_back = control.gains_db();
        assert!((read_back[3] - 5.5).abs() < 0.01);
        assert!((read_back[7] - (-3.25)).abs() < 0.01);
    }

    #[test]
    fn out_of_range_band_index_is_ignored_not_a_panic() {
        let control = EqualizerControl::new();
        control.set_gain(NUM_BANDS + 5, 10.0); // no debe entrar en pánico
    }

    #[test]
    fn presets_have_the_expected_band_count_and_are_within_range() {
        for preset in [
            EqPreset::Flat,
            EqPreset::Rock,
            EqPreset::Pop,
            EqPreset::Jazz,
        ] {
            let gains = preset
                .gains_db()
                .expect("los presets con nombre tienen curva propia");
            assert_eq!(gains.len(), NUM_BANDS);
            for gain in gains {
                assert!(
                    (MIN_GAIN_DB..=MAX_GAIN_DB).contains(&gain),
                    "ganancia fuera de rango en preset {preset:?}: {gain}"
                );
            }
        }
        assert_eq!(EqPreset::Custom.gains_db(), None);
    }

    /// Fuente sintética mínima para probar `EqualizerSource` sin depender de archivos reales.
    struct TestSource {
        samples: std::vec::IntoIter<f32>,
        channels: u16,
        sample_rate: u32,
    }

    impl TestSource {
        fn new(samples: Vec<f32>, channels: u16, sample_rate: u32) -> Self {
            Self {
                samples: samples.into_iter(),
                channels,
                sample_rate,
            }
        }
    }

    impl Iterator for TestSource {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            self.samples.next()
        }
    }

    impl Source for TestSource {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> ChannelCount {
            ChannelCount::new(self.channels).unwrap()
        }
        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(self.sample_rate).unwrap()
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn equalizer_source_passes_audio_through_unchanged_when_flat() {
        let samples: Vec<f32> = (0..200).map(|i| (i as f32 * 0.37).sin() * 0.8).collect();
        let source = TestSource::new(samples.clone(), 2, SAMPLE_RATE as u32);
        let control = EqualizerControl::new(); // todas las ganancias en 0 por defecto

        let output: Vec<f32> = EqualizerSource::new(source, control).collect();

        assert_eq!(output.len(), samples.len());
        for (original, processed) in samples.iter().zip(output.iter()) {
            assert!(
                (original - processed).abs() < 1e-3,
                "con todas las bandas en 0dB la salida debería ser ~igual a la entrada"
            );
        }
    }

    #[test]
    fn equalizer_source_reports_inner_specs() {
        let source = TestSource::new(vec![0.0; 10], 2, 48_000);
        let eq = EqualizerSource::new(source, EqualizerControl::new());
        assert_eq!(eq.channels().get(), 2);
        assert_eq!(eq.sample_rate().get(), 48_000);
    }

    /// Prueba de humo manual: reproduce un tono de 1000Hz (justo el centro de una banda del EQ)
    /// por el dispositivo de audio real, alternando esa banda entre -12dB y +12dB, para
    /// confirmar audiblemente que el ecualizador cambia el volumen del tono en tiempo real. Se
    /// ejecuta a propósito con `cargo test -- --ignored --nocapture` porque depende de hardware
    /// de audio real.
    #[test]
    #[ignore]
    fn eq_band_gain_is_audible_on_real_playback() {
        use rodio::{DeviceSinkBuilder, Player};

        let fixture = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures_eq/test-tone-1khz.mp3"
        ));

        let sink_handle =
            DeviceSinkBuilder::open_default_sink().expect("debería abrir el dispositivo de audio");
        let player = Player::connect_new(sink_handle.mixer());

        let decoder = super::super::decoder::TrackDecoder::open(&fixture)
            .expect("debería decodificar el tono de 1kHz");
        let control = EqualizerControl::new();
        player.append(EqualizerSource::new(decoder, control.clone()));
        player.play();

        let band_1khz = BAND_FREQUENCIES.iter().position(|f| *f == 1000.0).unwrap();

        println!("Volumen normal (0dB) por 2s...");
        std::thread::sleep(Duration::from_secs(2));

        println!("Banda de 1kHz en -12dB por 2s (debería sonar mucho más bajo)...");
        control.set_gain(band_1khz, MIN_GAIN_DB);
        std::thread::sleep(Duration::from_secs(2));

        println!("Banda de 1kHz en +12dB por 2s (debería sonar mucho más fuerte)...");
        control.set_gain(band_1khz, MAX_GAIN_DB);
        std::thread::sleep(Duration::from_secs(2));

        control.set_gain(band_1khz, 0.0);
        player.stop();
        println!("Prueba de ecualizador completada.");
    }
}
