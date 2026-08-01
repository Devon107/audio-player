use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};
use symphonia::core::audio::Channels;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Time, Timestamp};

/// Decodifica un archivo de audio con Symphonia y lo expone como una fuente reproducible por rodio.
pub struct TrackDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Option<StdDuration>,
    buffer: Vec<f32>,
    buffer_pos: usize,
}

impl TrackDecoder {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file =
            File::open(path).map_err(|e| format!("No se pudo abrir '{}': {e}", path.display()))?;
        let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| format!("No se pudo detectar el formato de audio: {e}"))?;

        Self::from_format(format)
    }

    fn from_format(format: Box<dyn FormatReader>) -> Result<Self, String> {
        let track = format
            .first_track_known_codec(TrackType::Audio)
            .ok_or("El archivo no contiene una pista de audio soportada")?;

        let track_id = track.id;

        let audio_params = track
            .codec_params
            .as_ref()
            .and_then(CodecParameters::audio)
            .ok_or("No se encontraron parámetros de códec de audio")?
            .clone();

        let duration = track
            .time_base
            .zip(track.duration)
            .and_then(|(tb, dur)| tb.calc_time(Timestamp::new(dur.get() as i64)))
            .map(|t| StdDuration::from_secs_f64(t.as_secs_f64().max(0.0)));

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())
            .map_err(|e| format!("Códec de audio no soportado: {e}"))?;

        let channel_count = audio_params
            .channels
            .as_ref()
            .map(Channels::count)
            .filter(|&c| c > 0)
            .unwrap_or(2) as u16;

        let sample_rate_hz = audio_params
            .sample_rate
            .filter(|&r| r > 0)
            .unwrap_or(44_100);

        let channels = ChannelCount::new(channel_count).ok_or("Número de canales inválido")?;
        let sample_rate =
            SampleRate::new(sample_rate_hz).ok_or("Frecuencia de muestreo inválida")?;

        Ok(Self {
            format,
            decoder,
            track_id,
            channels,
            sample_rate,
            duration,
            buffer: Vec::new(),
            buffer_pos: 0,
        })
    }

    /// Duración total de la pista, si el contenedor la reporta.
    pub fn total_duration_hint(&self) -> Option<StdDuration> {
        self.duration
    }

    /// Decodifica el siguiente paquete útil en `self.buffer`. Devuelve `false` al llegar al final
    /// del flujo o si ocurre un error irrecuperable.
    fn decode_next_packet(&mut self) -> bool {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return false,
                Err(SymphoniaError::IoError(_) | SymphoniaError::ResetRequired) => return false,
                Err(_) => return false,
            };

            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    decoded.copy_to_vec_interleaved(&mut self.buffer);
                    self.buffer_pos = 0;
                    if self.buffer.is_empty() {
                        continue;
                    }
                    return true;
                }
                // Paquete corrupto: se descarta y se continúa con el siguiente.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(_) => return false,
            }
        }
    }
}

impl Iterator for TrackDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            if self.buffer_pos < self.buffer.len() {
                let sample = self.buffer[self.buffer_pos];
                self.buffer_pos += 1;
                return Some(sample);
            }

            if !self.decode_next_packet() {
                return None;
            }
        }
    }
}

impl Source for TrackDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<StdDuration> {
        self.duration
    }

    fn try_seek(&mut self, pos: StdDuration) -> Result<(), SeekError> {
        let time = Time::try_from_secs_f64(pos.as_secs_f64())
            .ok_or_else(|| seek_error("posición de búsqueda inválida"))?;

        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| seek_error(&format!("no se pudo buscar la posición: {e}")))?;

        self.decoder.reset();
        self.buffer.clear();
        self.buffer_pos = 0;

        Ok(())
    }
}

fn seek_error(message: &str) -> SeekError {
    SeekError::Other(Arc::new(std::io::Error::other(message.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-tone.mp3"
        ))
    }

    #[test]
    fn opens_and_reports_stream_specs() {
        let decoder = TrackDecoder::open(&fixture_path()).expect("debería abrir el mp3 de prueba");

        assert_eq!(decoder.channels().get(), 1, "el tono de prueba es mono");
        assert_eq!(decoder.sample_rate().get(), 44_100);

        let duration = decoder
            .total_duration_hint()
            .expect("el mp3 debería reportar duración");
        let diff = (duration.as_secs_f64() - 8.0).abs();
        assert!(diff < 0.2, "duración esperada ~8s, obtenida {duration:?}");
    }

    #[test]
    fn decodes_the_expected_number_of_samples() {
        let decoder = TrackDecoder::open(&fixture_path()).expect("debería abrir el mp3 de prueba");
        let channels = decoder.channels().get() as usize;
        let sample_rate = decoder.sample_rate().get() as usize;

        let samples: Vec<f32> = decoder.collect();

        // Un mp3 tiene algo de "encoder delay/padding"; toleramos ±10% del total esperado.
        let expected = 8 * sample_rate * channels;
        let lower = expected * 9 / 10;
        let upper = expected * 12 / 10;
        assert!(
            samples.len() >= lower && samples.len() <= upper,
            "cantidad de muestras fuera de rango: {} (esperado ~{expected})",
            samples.len()
        );

        // No debería ser silencio total: el tono de 440Hz produce amplitud no nula.
        let has_signal = samples.iter().any(|s| s.abs() > 0.01);
        assert!(has_signal, "se esperaba señal de audio, no silencio");
    }

    #[test]
    fn seeking_moves_the_decode_position_forward() {
        let mut decoder =
            TrackDecoder::open(&fixture_path()).expect("debería abrir el mp3 de prueba");

        decoder
            .try_seek(Duration::from_secs(4))
            .expect("la búsqueda debería funcionar");

        // Tras buscar a los 4s, deberían quedar aproximadamente 4s de muestras por decodificar.
        let remaining: Vec<f32> = decoder.collect();
        let sample_rate = 44_100usize;
        let expected_remaining = 4 * sample_rate;
        let lower = expected_remaining * 7 / 10;
        let upper = expected_remaining * 13 / 10;
        assert!(
            remaining.len() >= lower && remaining.len() <= upper,
            "muestras restantes fuera de rango tras seek: {} (esperado ~{expected_remaining})",
            remaining.len()
        );
    }

    #[test]
    fn rejects_missing_file() {
        let result = TrackDecoder::open(std::path::Path::new("/no/existe/archivo.mp3"));
        assert!(result.is_err());
    }
}
