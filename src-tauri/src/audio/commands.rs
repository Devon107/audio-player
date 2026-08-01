use std::path::PathBuf;
use std::time::Duration;

use tauri::State;

use super::output::{AudioCommand, AudioEngineHandle};

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
