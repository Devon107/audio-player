use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rodio::{DeviceSinkBuilder, Player};
use serde::{Deserialize, Serialize};
use souvlaki::{MediaControls, MediaPlayback, MediaPosition};
use tauri::{AppHandle, Emitter, Runtime};

use crate::db::{settings, DbHandle};

use super::decoder::TrackDecoder;
use super::equalizer::{EqualizerControl, EqualizerSource};
use super::media_controls;
use super::queue::{QueueState, QueueTrack, QueueTrackInput, RepeatMode};

/// Intervalo al que se emite el progreso de reproducción y se revisa si la pista terminó.
const TICK_INTERVAL: Duration = Duration::from_millis(250);

/// Clave en `settings` bajo la que se guarda la cola de reproducción, para poder restaurarla
/// (última pista + resto de la cola) la próxima vez que arranque la app.
const QUEUE_STATE_KEY: &str = "queue_state";

/// Cada cuánto se guarda la posición de reproducción mientras suena una pista, aparte de los
/// momentos puntuales (pausar, buscar, cambiar de pista) donde se guarda al toque. Guardar en
/// cada tick de progreso (cada 250ms) sería una escritura a SQLite innecesariamente seguida; con
/// este intervalo, en el peor caso (cerrar la app mientras reproduce, sin pausar) se pierden a lo
/// sumo unos segundos de posición.
const POSITION_SAVE_INTERVAL: Duration = Duration::from_secs(5);

pub enum AudioCommand {
    /// Carga un archivo suelto y lo reproduce, sin pasar por la cola (vacía la cola actual).
    Load {
        path: PathBuf,
        autoplay: bool,
    },
    Play,
    Pause,
    Stop,
    Seek(Duration),
    SetVolume(f32),

    /// Reemplaza toda la cola y, si se indica `start_index`, empieza a reproducir desde ahí (si
    /// no, empieza desde el principio según el modo aleatorio/secuencial).
    SetQueue {
        items: Vec<QueueTrackInput>,
        start_index: Option<usize>,
        autoplay: bool,
    },
    AddToQueue(Vec<QueueTrackInput>),
    RemoveFromQueue(u64),
    /// Quita de la cola cualquier pista cuyo archivo esté bajo esta carpeta (usado cuando se deja
    /// de vigilar una carpeta de la biblioteca: sus pistas ya no deberían seguir en la cola ni
    /// seguir sonando en segundo plano aunque el archivo siga físicamente en disco).
    RemoveQueueItemsUnder(PathBuf),
    ReorderQueue {
        item_id: u64,
        new_index: usize,
    },
    ClearQueue,
    PlayQueueItem(u64),
    NextTrack,
    PreviousTrack,
    /// Alterna play/pause. Separado de `Play`/`Pause` porque los controles nativos de medios del
    /// SO (tecla multimedia de "toggle", botón único de un parlante) no saben si está sonando o
    /// pausado — quien sí lo sabe es el motor, vía `player.is_paused()`.
    TogglePlayPause,
    SetShuffle(bool),
    SetRepeatMode(RepeatMode),
    GetQueueState(mpsc::Sender<QueueSnapshot>),
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    position_secs: f64,
    duration_secs: Option<f64>,
    /// Estado real de reproducción del motor. El frontend no tiene otra forma de enterarse
    /// cuando play/pause se dispara desde afuera de la propia UI (teclas multimedia del teclado,
    /// control remoto Bluetooth vía AVRCP/MPRIS): sin esto, el ícono de play/pause queda
    /// desincronizado porque el frontend solo actualizaba `isPlaying` de forma optimista.
    is_playing: bool,
}

#[derive(Clone, Serialize)]
struct LoadedPayload {
    duration_secs: Option<f64>,
}

#[derive(Clone, Serialize)]
pub struct QueueSnapshot {
    items: Vec<QueueTrack>,
    current_id: Option<u64>,
    shuffle: bool,
    repeat: RepeatMode,
    has_previous: bool,
    /// Duración de la pista actual, si ya se conoce. Se expone acá (y no solo vía el evento
    /// `player://loaded`) para que la pista restaurada al arrancar la app tenga la duración
    /// correcta desde la primera consulta del frontend, sin depender de un evento que puede
    /// emitirse antes de que el frontend termine de registrar sus listeners.
    duration_secs: Option<f64>,
    /// Igual que `duration_secs`: se expone acá además de en `player://progress` para que la
    /// posición restaurada (vía seek) se vea correcta desde la primera consulta, no recién en el
    /// próximo tick de progreso.
    position_secs: f64,
}

fn queue_snapshot(
    queue: &QueueState,
    duration: Option<Duration>,
    position: Duration,
) -> QueueSnapshot {
    QueueSnapshot {
        items: queue.items().to_vec(),
        current_id: queue.current_id(),
        shuffle: queue.shuffle(),
        repeat: queue.repeat(),
        has_previous: queue.has_previous(),
        duration_secs: duration.map(|d| d.as_secs_f64()),
        position_secs: position.as_secs_f64(),
    }
}

/// Forma en la que se persiste la cola en `settings`. No guarda el `id` interno de cada pista
/// (`QueueTrack::id`): se reasigna desde 0 cada vez que arranca el motor, así que no tiene
/// sentido persistirlo — en su lugar se guarda la posición de la pista actual dentro de la
/// lista, y se recupera el id real correspondiente después de recargar los ítems.
#[derive(Serialize, Deserialize)]
struct PersistedQueue {
    items: Vec<QueueTrackInput>,
    current_index: Option<usize>,
    shuffle: bool,
    repeat: RepeatMode,
    /// Dónde iba la reproducción de la pista actual. `#[serde(default)]` para que las colas
    /// guardadas antes de que existiera este campo se sigan pudiendo leer (quedan en 0.0).
    #[serde(default)]
    position_secs: f64,
}

fn persist_queue(db: &DbHandle, queue: &QueueState, position: Duration) {
    let persisted = PersistedQueue {
        items: queue
            .items()
            .iter()
            .map(|t| QueueTrackInput {
                path: t.path.clone(),
                track_id: t.track_id,
            })
            .collect(),
        current_index: queue
            .current_id()
            .and_then(|id| queue.items().iter().position(|t| t.id == id)),
        shuffle: queue.shuffle(),
        repeat: queue.repeat(),
        position_secs: position.as_secs_f64(),
    };

    if let Ok(json) = serde_json::to_string(&persisted) {
        let conn = db.lock();
        let _ = settings::set(&conn, QUEUE_STATE_KEY, &json);
    }
}

fn load_persisted_queue(db: &DbHandle) -> Option<PersistedQueue> {
    let conn = db.lock();
    let json = settings::get(&conn, QUEUE_STATE_KEY).ok().flatten()?;
    serde_json::from_str(&json).ok()
}

/// Asa liviana (clonable vía `State`) para enviar comandos al hilo dedicado de audio.
pub struct AudioEngineHandle {
    tx: mpsc::Sender<AudioCommand>,
}

impl AudioEngineHandle {
    /// Genérico sobre `R: Runtime` para poder probarse con `tauri::test::mock_app()`, que usa
    /// `MockRuntime` en lugar del runtime `Wry` por defecto.
    pub fn spawn<R: Runtime + 'static>(
        app: AppHandle<R>,
        eq: EqualizerControl,
        db: DbHandle,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let media_tx = tx.clone();
        thread::spawn(move || run_engine(rx, media_tx, app, eq, db));
        Self { tx }
    }

    pub fn send(&self, command: AudioCommand) -> Result<(), String> {
        self.tx
            .send(command)
            .map_err(|_| "El motor de audio no está disponible".to_string())
    }
}

/// Intenta cargar y reproducir `path`, pasando la señal decodificada por el ecualizador antes de
/// llegar al `Player`. Devuelve la duración reportada si tuvo éxito.
fn load_and_play(
    player: &Player,
    path: &Path,
    autoplay: bool,
    eq: &EqualizerControl,
) -> Result<Option<Duration>, String> {
    let decoder = TrackDecoder::open(path)?;
    let duration = decoder.total_duration_hint();
    player.clear();
    player.append(EqualizerSource::new(decoder, eq.clone()));
    if autoplay {
        player.play();
    } else {
        player.pause();
    }
    Ok(duration)
}

/// Aplica el resultado de `load_and_play` al estado local del motor y emite el evento
/// correspondiente (`player://loaded` o `player://error`). También reporta la metadata de la
/// pista a los controles nativos de medios del SO, si están disponibles.
fn handle_load_result<R: Runtime>(
    result: Result<Option<Duration>, String>,
    app: &AppHandle<R>,
    duration: &mut Option<Duration>,
    has_track: &mut bool,
    media_controls: Option<&mut MediaControls>,
    path: &Path,
) {
    match result {
        Ok(dur) => {
            *duration = dur;
            *has_track = true;
            let _ = app.emit(
                "player://loaded",
                LoadedPayload {
                    duration_secs: dur.map(|d| d.as_secs_f64()),
                },
            );
            if let Some(controls) = media_controls {
                media_controls::update_metadata(controls, app, path, dur.map(|d| d.as_secs_f64()));
            }
        }
        Err(e) => {
            *has_track = false;
            *duration = None;
            let _ = app.emit("player://error", e);
        }
    }
}

/// Bucle principal del motor de audio. Vive en su propio hilo del sistema operativo porque el
/// `cpal::Stream` que mantiene abierto el dispositivo de salida no es `Send`/`Sync` en todas las
/// plataformas, por lo que no puede vivir directamente en el estado gestionado por Tauri.
fn run_engine<R: Runtime>(
    rx: mpsc::Receiver<AudioCommand>,
    media_tx: mpsc::Sender<AudioCommand>,
    app: AppHandle<R>,
    eq: EqualizerControl,
    db: DbHandle,
) {
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
    let mut queue = QueueState::default();
    let mut media_controls = media_controls::init(&app, media_tx);

    // Restaura la última cola guardada (Fase de persistencia de reproducción): la pista actual
    // queda cargada y lista (con su duración y metadata ya disponibles), pero en pausa — no
    // arranca a reproducir sola al abrir la app.
    if let Some(persisted) = load_persisted_queue(&db) {
        queue.set_items(persisted.items);
        queue.set_shuffle(persisted.shuffle);
        queue.set_repeat(persisted.repeat);

        let restored_track = persisted
            .current_index
            .and_then(|i| queue.items().get(i).cloned())
            .and_then(|track| queue.play_item(track.id));

        if let Some(track) = restored_track {
            handle_load_result(
                load_and_play(&player, Path::new(&track.path), false, &eq),
                &app,
                &mut duration,
                &mut has_track,
                media_controls.as_mut(),
                Path::new(&track.path),
            );
            if has_track && persisted.position_secs > 0.0 {
                let _ = player.try_seek(Duration::from_secs_f64(persisted.position_secs));
            }
        }
    }
    let mut last_position_save = Instant::now();

    // Aplica el resultado de avanzar/retroceder/saltar en la cola (o detiene la reproducción si
    // `$track` es `None`, es decir, no queda nada por reproducir).
    macro_rules! apply_track {
        ($track:expr) => {
            match $track {
                Some(track) => handle_load_result(
                    load_and_play(&player, Path::new(&track.path), true, &eq),
                    &app,
                    &mut duration,
                    &mut has_track,
                    media_controls.as_mut(),
                    Path::new(&track.path),
                ),
                None => {
                    player.stop();
                    has_track = false;
                    duration = None;
                }
            }
        };
    }

    loop {
        let mut queue_changed = false;
        let had_track = has_track;

        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(AudioCommand::Load { path, autoplay }) => {
                queue.clear();
                queue_changed = true;
                handle_load_result(
                    load_and_play(&player, &path, autoplay, &eq),
                    &app,
                    &mut duration,
                    &mut has_track,
                    media_controls.as_mut(),
                    &path,
                );
            }
            Ok(AudioCommand::Play) => player.play(),
            Ok(AudioCommand::Pause) => {
                player.pause();
                persist_queue(&db, &queue, player.get_pos());
                last_position_save = Instant::now();
            }
            Ok(AudioCommand::TogglePlayPause) => {
                if has_track {
                    if player.is_paused() {
                        player.play();
                    } else {
                        player.pause();
                        persist_queue(&db, &queue, player.get_pos());
                        last_position_save = Instant::now();
                    }
                }
            }
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
                } else {
                    persist_queue(&db, &queue, player.get_pos());
                    last_position_save = Instant::now();
                }
            }
            Ok(AudioCommand::SetVolume(volume)) => player.set_volume(volume.clamp(0.0, 2.0)),

            Ok(AudioCommand::SetQueue {
                items,
                start_index,
                autoplay,
            }) => {
                queue.set_items(items);
                queue_changed = true;

                let start_track = start_index
                    .and_then(|i| queue.items().get(i).cloned())
                    .and_then(|track| queue.play_item(track.id))
                    .or_else(|| queue.next());

                match start_track {
                    Some(track) => handle_load_result(
                        load_and_play(&player, Path::new(&track.path), autoplay, &eq),
                        &app,
                        &mut duration,
                        &mut has_track,
                        media_controls.as_mut(),
                        Path::new(&track.path),
                    ),
                    None => {
                        player.stop();
                        has_track = false;
                        duration = None;
                    }
                }
            }
            Ok(AudioCommand::AddToQueue(items)) => {
                queue.add_items(items);
                queue_changed = true;
            }
            Ok(AudioCommand::RemoveFromQueue(item_id)) => {
                queue.remove(item_id);
                queue_changed = true;
            }
            Ok(AudioCommand::RemoveQueueItemsUnder(root)) => {
                queue_changed = true;
                if queue.remove_under(&root) {
                    player.stop();
                    has_track = false;
                    duration = None;
                }
            }
            Ok(AudioCommand::ReorderQueue { item_id, new_index }) => {
                queue.reorder(item_id, new_index);
                queue_changed = true;
            }
            Ok(AudioCommand::ClearQueue) => {
                queue.clear();
                queue_changed = true;
                player.stop();
                has_track = false;
                duration = None;
            }
            Ok(AudioCommand::PlayQueueItem(item_id)) => {
                queue_changed = true;
                apply_track!(queue.play_item(item_id));
            }
            Ok(AudioCommand::NextTrack) => {
                queue_changed = true;
                apply_track!(queue.next());
            }
            Ok(AudioCommand::PreviousTrack) => {
                // Si no hay historial (primera o única pista de la cola), no hacer nada: la
                // reproducción en curso no debe interrumpirse por retroceder sin tener adónde ir.
                if let Some(track) = queue.previous() {
                    queue_changed = true;
                    apply_track!(Some(track));
                }
            }
            Ok(AudioCommand::SetShuffle(enabled)) => {
                queue.set_shuffle(enabled);
                queue_changed = true;
            }
            Ok(AudioCommand::SetRepeatMode(mode)) => {
                queue.set_repeat(mode);
                queue_changed = true;
            }
            Ok(AudioCommand::GetQueueState(reply)) => {
                let _ = reply.send(queue_snapshot(&queue, duration, player.get_pos()));
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if has_track {
            if player.empty() {
                has_track = false;
                let next_track = queue.next();
                if next_track.is_some() {
                    queue_changed = true;
                }
                apply_track!(next_track);
                if !has_track {
                    let _ = app.emit("player://track-ended", ());
                }
            } else {
                let _ = app.emit(
                    "player://progress",
                    ProgressPayload {
                        position_secs: player.get_pos().as_secs_f64(),
                        duration_secs: duration.map(|d| d.as_secs_f64()),
                        is_playing: !player.is_paused(),
                    },
                );

                if !player.is_paused() && last_position_save.elapsed() >= POSITION_SAVE_INTERVAL {
                    persist_queue(&db, &queue, player.get_pos());
                    last_position_save = Instant::now();
                }
            }
        }

        // Sincroniza el estado de reproducción con los controles nativos de medios del SO: la
        // posición/estado playing-vs-paused se reenvía en cada tick (misma cadencia que
        // `player://progress`, para que el widget del SO no se desincronice), pero "Stopped" solo
        // se manda una vez, justo en la transición, para no spamear el bus de mensajes mientras
        // está inactivo.
        if let Some(controls) = media_controls.as_mut() {
            if has_track {
                let progress = Some(MediaPosition(player.get_pos()));
                let playback = if player.is_paused() {
                    MediaPlayback::Paused { progress }
                } else {
                    MediaPlayback::Playing { progress }
                };
                media_controls::update_playback(controls, playback);
            } else if had_track {
                media_controls::update_playback(controls, MediaPlayback::Stopped);
                media_controls::clear_metadata(controls);
            }
        }

        if queue_changed {
            persist_queue(&db, &queue, player.get_pos());
            last_position_save = Instant::now();
            let _ = app.emit(
                "player://queue-changed",
                queue_snapshot(&queue, duration, player.get_pos()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn load_persisted_queue_returns_none_when_nothing_saved() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        assert!(load_persisted_queue(&db).is_none());
    }

    #[test]
    fn persisted_queue_round_trips_items_current_track_and_modes() {
        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let mut queue = QueueState::default();
        queue.set_items(vec![
            QueueTrackInput {
                path: "a.mp3".to_string(),
                track_id: Some(1),
            },
            QueueTrackInput {
                path: "b.mp3".to_string(),
                track_id: Some(2),
            },
            QueueTrackInput {
                path: "c.mp3".to_string(),
                track_id: None,
            },
        ]);
        queue.next(); // a
        queue.next(); // b <- queda como actual
        queue.set_repeat(RepeatMode::Track);

        persist_queue(&db, &queue, Duration::from_secs(42));

        let restored = load_persisted_queue(&db).expect("debería haber cola persistida");
        assert_eq!(restored.items.len(), 3);
        assert_eq!(restored.items[1].path, "b.mp3");
        assert_eq!(
            restored.position_secs, 42.0,
            "la posición también debería quedar guardada"
        );
        assert_eq!(
            restored.current_index,
            Some(1),
            "b es la pista actual, en la posición 1"
        );
        assert!(!restored.shuffle);
        assert_eq!(restored.repeat, RepeatMode::Track);
    }

    /// Prueba de humo manual: siembra una cola guardada directamente en la base de datos antes
    /// de arrancar el motor, y verifica que la restaura (pista actual cargada, con duración ya
    /// conocida) sin arrancar a reproducir sola. Depende de hardware de audio real; se corre a
    /// propósito con `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn restores_persisted_queue_paused_on_startup() {
        let fixtures =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        let path = |name: &str| fixtures.join(name).to_string_lossy().into_owned();

        let db = DbHandle::open_at(Path::new(":memory:")).unwrap();
        let persisted = PersistedQueue {
            items: vec![
                QueueTrackInput {
                    path: path("test-tone-no-tags.mp3"),
                    track_id: None,
                },
                QueueTrackInput {
                    path: path("test-tone.mp3"),
                    track_id: None,
                },
            ],
            current_index: Some(1),
            shuffle: false,
            repeat: RepeatMode::Off,
            position_secs: 3.0,
        };
        {
            let conn = db.lock();
            settings::set(
                &conn,
                QUEUE_STATE_KEY,
                &serde_json::to_string(&persisted).unwrap(),
            )
            .unwrap();
        }

        let app = tauri::test::mock_app();
        let handle = AudioEngineHandle::spawn(app.handle().clone(), EqualizerControl::new(), db);

        sleep(Duration::from_millis(300));
        let state = queue_state(&handle);
        assert_eq!(state.items.len(), 2);
        assert_eq!(
            state.current_id,
            Some(state.items[1].id),
            "debería restaurar el índice 1 (test-tone.mp3) como actual"
        );
        assert!(
            state.duration_secs.is_some(),
            "la pista restaurada debería tener duración conocida sin necesidad de reproducirla"
        );
        assert!(
            state.position_secs >= 2.5,
            "debería haber buscado a los ~3s guardados, quedó en {}",
            state.position_secs
        );
        println!(
            "Cola restaurada OK: pista actual = {}, posición = {:.2}s",
            state.items[1].path, state.position_secs
        );

        handle.send(AudioCommand::Stop).unwrap();
    }

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

    fn queue_state(handle: &AudioEngineHandle) -> QueueSnapshot {
        let (tx, rx) = mpsc::channel();
        handle.send(AudioCommand::GetQueueState(tx)).unwrap();
        rx.recv().unwrap()
    }

    /// Prueba de humo manual: arma una cola con 3 pistas cortas de prueba y verifica, contra el
    /// motor real (dispositivo de audio real), que el avance automático al terminar una pista,
    /// `next_track` y `previous_track` navegan la cola correctamente. Se ejecuta a propósito con
    /// `cargo test -- --ignored --nocapture` porque depende de hardware de audio real y de
    /// tiempos de reproducción reales (no es instantánea).
    #[test]
    #[ignore]
    fn queue_auto_advances_and_supports_manual_navigation() {
        let fixtures =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
        let path = |name: &str| fixtures.join(name).to_string_lossy().into_owned();

        let app = tauri::test::mock_app();
        let db = DbHandle::open_at(std::path::Path::new(":memory:")).unwrap();
        let handle = AudioEngineHandle::spawn(app.handle().clone(), EqualizerControl::new(), db);

        // Pista 1 dura ~2s, para que el avance automático a la pista 2 ocurra pronto.
        let items = vec![
            QueueTrackInput {
                path: path("test-tone-no-tags.mp3"),
                track_id: None,
            },
            QueueTrackInput {
                path: path("test-tone-with-cover.mp3"),
                track_id: None,
            },
            QueueTrackInput {
                path: path("test-tone.mp3"),
                track_id: None,
            },
        ];

        handle
            .send(AudioCommand::SetQueue {
                items,
                start_index: Some(0),
                autoplay: true,
            })
            .unwrap();

        sleep(Duration::from_millis(300));
        let state = queue_state(&handle);
        let [id1, id2, id3] = [state.items[0].id, state.items[1].id, state.items[2].id];
        assert_eq!(state.current_id, Some(id1));
        println!("Reproduciendo pista 1/3...");

        sleep(Duration::from_millis(2300));
        let state = queue_state(&handle);
        assert_eq!(
            state.current_id,
            Some(id2),
            "debería haber avanzado solo a la pista 2 al terminar la 1"
        );
        println!("Avance automático OK: ahora en pista 2/3.");

        handle.send(AudioCommand::NextTrack).unwrap();
        sleep(Duration::from_millis(300));
        assert_eq!(queue_state(&handle).current_id, Some(id3));
        println!("next_track OK: ahora en pista 3/3.");

        handle.send(AudioCommand::PreviousTrack).unwrap();
        sleep(Duration::from_millis(300));
        assert_eq!(queue_state(&handle).current_id, Some(id2));
        println!("previous_track OK: volvió a la pista 2/3.");

        handle.send(AudioCommand::Stop).unwrap();
        println!("Prueba de cola completada.");
    }

    /// Prueba de humo manual: reproduce el tono de prueba y alterna el volumen entre silencio,
    /// bajo y alto, para confirmar audiblemente que `Player::set_volume` (vía `AudioCommand::
    /// SetVolume`) sí afecta el audio real. Diagnóstico puntual para un reporte de que el slider
    /// de volumen del frontend no cambiaba nada.
    #[test]
    #[ignore]
    fn set_volume_is_audible() {
        let fixture = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-tone.mp3"
        ));

        let app = tauri::test::mock_app();
        let db = DbHandle::open_at(std::path::Path::new(":memory:")).unwrap();
        let handle = AudioEngineHandle::spawn(app.handle().clone(), EqualizerControl::new(), db);

        handle
            .send(AudioCommand::Load {
                path: fixture,
                autoplay: true,
            })
            .unwrap();

        println!("Volumen normal (1.0) por 2s...");
        sleep(Duration::from_secs(2));

        println!("Volumen en 0.0 (silencio) por 2s...");
        handle.send(AudioCommand::SetVolume(0.0)).unwrap();
        sleep(Duration::from_secs(2));

        println!("Volumen en 2.0 (el doble) por 2s...");
        handle.send(AudioCommand::SetVolume(2.0)).unwrap();
        sleep(Duration::from_secs(2));

        handle.send(AudioCommand::Stop).unwrap();
        println!("Prueba de volumen completada.");
    }

    /// Diagnóstico aislado: usa `Player` directamente (sin pasar por `AudioCommand`/
    /// `run_engine`) para descartar que el bug esté en el despacho de comandos del motor en vez
    /// de en el propio `Player::set_volume` de rodio.
    #[test]
    #[ignore]
    fn set_volume_direct_player_is_audible() {
        let fixture = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-tone.mp3"
        ));

        let sink_handle =
            DeviceSinkBuilder::open_default_sink().expect("debería abrir el dispositivo de audio");
        let player = Player::connect_new(sink_handle.mixer());

        let decoder = TrackDecoder::open(&fixture).expect("debería decodificar el mp3 de prueba");
        player.append(decoder);
        player.play();

        println!("volume() = {}", player.volume());
        println!("Volumen normal (1.0) por 2s...");
        sleep(Duration::from_secs(2));

        player.set_volume(0.0);
        println!("volume() tras set_volume(0.0) = {}", player.volume());
        println!("Volumen en 0.0 (silencio) por 2s...");
        sleep(Duration::from_secs(2));

        player.set_volume(3.0);
        println!("volume() tras set_volume(3.0) = {}", player.volume());
        println!("Volumen en 3.0 (el triple) por 2s...");
        sleep(Duration::from_secs(2));

        player.stop();
        println!("Prueba de volumen directa completada.");
    }
}
