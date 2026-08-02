use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use tauri::State;

use crate::db::{settings, DbHandle};

use super::equalizer::{EqPreset, EqStateSnapshot, EqualizerControl, NUM_BANDS};
use super::output::{AudioCommand, AudioEngineHandle, QueueSnapshot};
use super::queue::{QueueTrackInput, RepeatMode};

const EQ_GAINS_KEY: &str = "eq_gains";
const EQ_PRESET_KEY: &str = "eq_preset";

#[tauri::command]
pub fn load_track(
    state: State<AudioEngineHandle>,
    path: String,
    autoplay: bool,
) -> Result<(), String> {
    state.send(AudioCommand::Load {
        path: PathBuf::from(path),
        autoplay,
    })
}

#[tauri::command]
pub fn play(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::Play)
}

#[tauri::command]
pub fn pause(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::Pause)
}

#[tauri::command]
pub fn stop(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::Stop)
}

#[tauri::command]
pub fn seek(state: State<AudioEngineHandle>, position_secs: f64) -> Result<(), String> {
    state.send(AudioCommand::Seek(Duration::from_secs_f64(
        position_secs.max(0.0),
    )))
}

#[tauri::command]
pub fn set_volume(state: State<AudioEngineHandle>, volume: f32) -> Result<(), String> {
    state.send(AudioCommand::SetVolume(volume))
}

/// Reemplaza toda la cola de reproducción. Si `start_index` se indica, comienza a reproducir
/// desde esa posición; si no, comienza según el modo aleatorio/secuencial vigente.
#[tauri::command]
pub fn set_queue(
    state: State<AudioEngineHandle>,
    items: Vec<QueueTrackInput>,
    start_index: Option<usize>,
    autoplay: bool,
) -> Result<(), String> {
    state.send(AudioCommand::SetQueue {
        items,
        start_index,
        autoplay,
    })
}

#[tauri::command]
pub fn add_to_queue(
    state: State<AudioEngineHandle>,
    items: Vec<QueueTrackInput>,
) -> Result<(), String> {
    state.send(AudioCommand::AddToQueue(items))
}

#[tauri::command]
pub fn remove_from_queue(state: State<AudioEngineHandle>, item_id: u64) -> Result<(), String> {
    state.send(AudioCommand::RemoveFromQueue(item_id))
}

#[tauri::command]
pub fn reorder_queue(
    state: State<AudioEngineHandle>,
    item_id: u64,
    new_index: usize,
) -> Result<(), String> {
    state.send(AudioCommand::ReorderQueue { item_id, new_index })
}

#[tauri::command]
pub fn clear_queue(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::ClearQueue)
}

#[tauri::command]
pub fn play_queue_item(state: State<AudioEngineHandle>, item_id: u64) -> Result<(), String> {
    state.send(AudioCommand::PlayQueueItem(item_id))
}

#[tauri::command]
pub fn next_track(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::NextTrack)
}

#[tauri::command]
pub fn previous_track(state: State<AudioEngineHandle>) -> Result<(), String> {
    state.send(AudioCommand::PreviousTrack)
}

#[tauri::command]
pub fn set_shuffle(state: State<AudioEngineHandle>, enabled: bool) -> Result<(), String> {
    state.send(AudioCommand::SetShuffle(enabled))
}

#[tauri::command]
pub fn set_repeat_mode(state: State<AudioEngineHandle>, mode: RepeatMode) -> Result<(), String> {
    state.send(AudioCommand::SetRepeatMode(mode))
}

#[tauri::command]
pub fn get_queue_state(state: State<AudioEngineHandle>) -> Result<QueueSnapshot, String> {
    let (tx, rx) = mpsc::channel();
    state.send(AudioCommand::GetQueueState(tx))?;
    rx.recv()
        .map_err(|_| "El motor de audio no respondió".to_string())
}

fn persist_eq(db: &DbHandle, gains: &[f32; NUM_BANDS], preset: EqPreset) -> Result<(), String> {
    let gains_json = serde_json::to_string(gains).map_err(|e| e.to_string())?;
    let preset_json = serde_json::to_string(&preset).map_err(|e| e.to_string())?;

    let conn = db.lock();
    settings::set(&conn, EQ_GAINS_KEY, &gains_json).map_err(|e| e.to_string())?;
    settings::set(&conn, EQ_PRESET_KEY, &preset_json).map_err(|e| e.to_string())
}

/// Ajusta la ganancia de una sola banda (0..10). El preset guardado pasa a "custom" porque ya no
/// coincide con ninguna curva con nombre.
#[tauri::command]
pub fn set_eq_band_gain(
    eq: State<EqualizerControl>,
    db: State<DbHandle>,
    band: usize,
    gain_db: f32,
) -> Result<(), String> {
    eq.set_gain(band, gain_db);
    persist_eq(&db, &eq.gains_db(), EqPreset::Custom)
}

#[tauri::command]
pub fn set_eq_preset(
    eq: State<EqualizerControl>,
    db: State<DbHandle>,
    preset: EqPreset,
) -> Result<(), String> {
    let gains = preset
        .gains_db()
        .ok_or_else(|| "El preset 'custom' no tiene una curva propia para aplicar".to_string())?;
    eq.set_gains(&gains);
    persist_eq(&db, &gains, preset)
}

#[tauri::command]
pub fn get_eq_state(
    eq: State<EqualizerControl>,
    db: State<DbHandle>,
) -> Result<EqStateSnapshot, String> {
    let preset = {
        let conn = db.lock();
        settings::get(&conn, EQ_PRESET_KEY)
            .map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(EqPreset::Flat)
    };
    Ok(EqStateSnapshot {
        gains_db: eq.gains_db(),
        preset,
    })
}
