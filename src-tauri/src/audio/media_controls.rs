use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use tauri::{AppHandle, Runtime};

use super::output::AudioCommand;
use crate::metadata::{cover_cache, reader};

/// Identificador ante el sistema operativo (nombre de servicio D-Bus en Linux/MPRIS) y nombre
/// mostrado en los widgets nativos de medios (centro de control de GNOME/KDE, SMTC de Windows,
/// Now Playing de macOS).
const DBUS_NAME: &str = "audio_player";
const DISPLAY_NAME: &str = "Reproductor de Audio";

/// Inicializa los controles nativos de medios del sistema operativo y reenvía sus eventos
/// (play/pause/siguiente/anterior/buscar, disparados por teclas multimedia del teclado o un
/// dispositivo Bluetooth como un parlante con botón de play) como `AudioCommand`s al motor de
/// audio a través de `tx`.
///
/// Si la plataforma no tiene una sesión de medios disponible (p. ej. sin D-Bus en Linux, o en
/// tests sin una ventana real en Windows), devuelve `None` y el resto de la app sigue
/// funcionando igual, solo sin esta integración — no es un error fatal.
///
/// Debe llamarse desde el hilo dedicado del motor de audio, no desde el hilo principal: en
/// Windows, obtener el HWND de la ventana sincrónicamente desde el hilo principal puede
/// interbloquearse con el bucle de eventos (mismo motivo por el que `pick_and_add_folder` debe
/// ser `async` en vez de sincrónico).
pub fn init<R: Runtime>(
    app: &AppHandle<R>,
    tx: mpsc::Sender<AudioCommand>,
) -> Option<MediaControls> {
    let config = PlatformConfig {
        dbus_name: DBUS_NAME,
        display_name: DISPLAY_NAME,
        hwnd: window_hwnd(app),
    };

    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(e) => {
            eprintln!("No se pudieron inicializar los controles de medios del sistema: {e:?}");
            return None;
        }
    };

    let attach_result = controls.attach(move |event: MediaControlEvent| {
        let command = match event {
            MediaControlEvent::Play => Some(AudioCommand::Play),
            MediaControlEvent::Pause => Some(AudioCommand::Pause),
            MediaControlEvent::Toggle => Some(AudioCommand::TogglePlayPause),
            MediaControlEvent::Next => Some(AudioCommand::NextTrack),
            MediaControlEvent::Previous => Some(AudioCommand::PreviousTrack),
            MediaControlEvent::Stop => Some(AudioCommand::Stop),
            MediaControlEvent::SetPosition(MediaPosition(pos)) => Some(AudioCommand::Seek(pos)),
            _ => None,
        };
        if let Some(command) = command {
            let _ = tx.send(command);
        }
    });

    if let Err(e) = attach_result {
        eprintln!("No se pudieron conectar los controles de medios del sistema: {e:?}");
        return None;
    }

    let _ = controls.set_playback(MediaPlayback::Stopped);

    Some(controls)
}

/// Actualiza título/artista/álbum/carátula/duración reportados al SO cuando cambia la pista que
/// está sonando. Reutiliza el mismo lector de etiquetas y la misma caché de carátulas que ya usa
/// el resto de la app (comandos de metadata del frontend), así que no hay una segunda fuente de
/// verdad para esos datos.
pub fn update_metadata<R: Runtime>(
    controls: &mut MediaControls,
    app: &AppHandle<R>,
    path: &Path,
    duration_secs: Option<f64>,
) {
    let meta = reader::read_metadata(path).ok();
    let cover_url = cover_cache::get_or_extract_cover(app, path)
        .ok()
        .flatten()
        .and_then(|p| p.to_str().map(|p| format!("file://{p}")));

    let _ = controls.set_metadata(MediaMetadata {
        title: meta.as_ref().map(|m| m.title.as_str()),
        artist: meta.as_ref().and_then(|m| m.artist.as_deref()),
        album: meta.as_ref().and_then(|m| m.album.as_deref()),
        cover_url: cover_url.as_deref(),
        duration: duration_secs.map(Duration::from_secs_f64),
    });
}

/// Limpia la metadata reportada al SO cuando la cola se queda sin nada que reproducir.
pub fn clear_metadata(controls: &mut MediaControls) {
    let _ = controls.set_metadata(MediaMetadata::default());
}

pub fn update_playback(controls: &mut MediaControls, playback: MediaPlayback) {
    let _ = controls.set_playback(playback);
}

#[cfg(target_os = "windows")]
fn window_hwnd<R: Runtime>(app: &AppHandle<R>) -> Option<*mut std::ffi::c_void> {
    use tauri::Manager;
    let window = app.get_webview_window("main")?;
    window
        .hwnd()
        .ok()
        .map(|hwnd| hwnd.0 as *mut std::ffi::c_void)
}

#[cfg(not(target_os = "windows"))]
fn window_hwnd<R: Runtime>(_app: &AppHandle<R>) -> Option<*mut std::ffi::c_void> {
    None
}
