use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use tauri::State;

use super::output::{AudioCommand, AudioEngineHandle, QueueSnapshot};
use super::queue::{QueueTrackInput, RepeatMode};

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
