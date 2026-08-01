use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rodio::{DeviceSinkBuilder, Player};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::decoder::TrackDecoder;

/// Intervalo al que se emite el progreso de reproducción y se revisa si la pista terminó.
const TICK_INTERVAL: Duration = Duration::from_millis(250);

pub enum AudioCommand {
    Load { path: PathBuf, autoplay: bool },
    Play,
    Pause,
    Stop,
    Seek(Duration),
    SetVolume(f32),
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    position_secs: f64,
    duration_secs: Option<f64>,
}

#[derive(Clone, Serialize)]
struct LoadedPayload {
    duration_secs: Option<f64>,
}

/// Asa liviana (clonable vía `State`) para enviar comandos al hilo dedicado de audio.
pub struct AudioEngineHandle {
    tx: mpsc::Sender<AudioCommand>,
}

impl AudioEngineHandle {
    pub fn spawn(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || run_engine(rx, app));
        Self { tx }
    }

    pub fn send(&self, command: AudioCommand) -> Result<(), String> {
        self.tx
            .send(command)
            .map_err(|_| "El motor de audio no está disponible".to_string())
    }
}

/// Bucle principal del motor de audio. Vive en su propio hilo del sistema operativo porque el
/// `cpal::Stream` que mantiene abierto el dispositivo de salida no es `Send`/`Sync` en todas las
/// plataformas, por lo que no puede vivir directamente en el estado gestionado por Tauri.
fn run_engine(rx: mpsc::Receiver<AudioCommand>, app: AppHandle) {
    let sink_handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(handle) => handle,
        Err(e) => {
            let _ = app.emit(
                "player://error",
                format!("No se pudo abrir el dispositivo de audio: {e}"),
            );
            return;
        }
    };

    let player = Player::connect_new(sink_handle.mixer());
    let mut duration: Option<Duration> = None;
    let mut has_track = false;

    loop {
        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(AudioCommand::Load { path, autoplay }) => match TrackDecoder::open(&path) {
                Ok(decoder) => {
                    duration = decoder.total_duration_hint();
                    player.clear();
                    player.append(decoder);
                    has_track = true;

                    if autoplay {
                        player.play();
                    } else {
                        player.pause();
                    }

                    let _ = app.emit(
                        "player://loaded",
                        LoadedPayload {
                            duration_secs: duration.map(|d| d.as_secs_f64()),
                        },
                    );
                }
                Err(e) => {
                    has_track = false;
                    duration = None;
                    let _ = app.emit("player://error", e);
                }
            },
            Ok(AudioCommand::Play) => player.play(),
            Ok(AudioCommand::Pause) => player.pause(),
            Ok(AudioCommand::Stop) => {
                player.stop();
                has_track = false;
                duration = None;
            }
            Ok(AudioCommand::Seek(pos)) => {
                if let Err(e) = player.try_seek(pos) {
                    let _ = app.emit(
                        "player://error",
                        format!("No se pudo buscar la posición: {e}"),
                    );
                }
            }
            Ok(AudioCommand::SetVolume(volume)) => player.set_volume(volume.clamp(0.0, 2.0)),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if has_track {
            if player.empty() {
                has_track = false;
                let _ = app.emit("player://track-ended", ());
            } else {
                let _ = app.emit(
                    "player://progress",
                    ProgressPayload {
                        position_secs: player.get_pos().as_secs_f64(),
                        duration_secs: duration.map(|d| d.as_secs_f64()),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    /// Prueba de humo manual: abre el dispositivo de audio real y reproduce el tono de prueba
    /// por unos segundos para confirmar audiblemente que la tubería decode -> rodio -> altavoces
    /// funciona de punta a punta. No corre en `cargo test` normal porque depende de hardware de
    /// audio real; se ejecuta a propósito con `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn plays_test_tone_through_default_device() {
        let fixture = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-tone.mp3"
        ));

        let sink_handle =
            DeviceSinkBuilder::open_default_sink().expect("debería abrir el dispositivo de audio");
        let player = Player::connect_new(sink_handle.mixer());

        let decoder = TrackDecoder::open(&fixture).expect("debería decodificar el mp3 de prueba");
        let duration = decoder.total_duration_hint();
        println!("Duración reportada: {duration:?}");

        player.append(decoder);
        player.play();

        println!("Reproduciendo 2s desde el inicio (deberías escuchar un tono de 440Hz)...");
        for _ in 0..4 {
            sleep(Duration::from_millis(500));
            println!("  posición: {:.2}s", player.get_pos().as_secs_f64());
        }
        assert!(
            player.get_pos() >= Duration::from_millis(1500),
            "la posición debería avanzar"
        );

        println!("Pausando 1s...");
        player.pause();
        let pos_before_pause = player.get_pos();
        sleep(Duration::from_millis(1000));
        let pos_after_pause = player.get_pos();
        assert!(
            (pos_after_pause.as_secs_f64() - pos_before_pause.as_secs_f64()).abs() < 0.1,
            "la posición no debería avanzar en pausa"
        );

        println!("Buscando a 6s y reanudando 2s (deberías escuchar el final del tono)...");
        player.play();
        player
            .try_seek(Duration::from_secs(6))
            .expect("la búsqueda debería funcionar");
        sleep(Duration::from_millis(2000));
        println!("  posición final: {:.2}s", player.get_pos().as_secs_f64());

        player.stop();
        println!("Prueba de reproducción completada.");
    }
}
