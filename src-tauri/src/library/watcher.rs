use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Runtime};

use crate::db::DbHandle;

use super::scanner;

/// Ventana de agrupación de eventos: evita procesar archivo por archivo cuando el SO emite
/// ráfagas de eventos (p. ej. al copiar muchas canciones de golpe a una carpeta vigilada).
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

pub enum WatcherCommand {
    AddFolder(PathBuf),
    RemoveFolder(PathBuf),
}

enum ThreadMessage {
    Command(WatcherCommand),
    FileEvent(notify::Result<Event>),
}

/// Asa liviana y clonable para controlar el hilo dedicado del watcher de biblioteca.
#[derive(Clone)]
pub struct LibraryWatcherHandle {
    tx: mpsc::Sender<WatcherCommand>,
}

impl LibraryWatcherHandle {
    pub fn spawn<R: Runtime + 'static>(
        app: AppHandle<R>,
        db: DbHandle,
        initial_folders: Vec<PathBuf>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WatcherCommand>();
        let (msg_tx, msg_rx) = mpsc::channel::<ThreadMessage>();

        // Reenvía los comandos de control al mismo canal que recibe los eventos del filesystem,
        // para que el hilo del watcher pueda atenderlos con un único `recv` en el bucle principal.
        {
            let msg_tx = msg_tx.clone();
            thread::spawn(move || {
                while let Ok(cmd) = cmd_rx.recv() {
                    if msg_tx.send(ThreadMessage::Command(cmd)).is_err() {
                        break;
                    }
                }
            });
        }

        thread::spawn(move || run_watcher(msg_rx, msg_tx, app, db, initial_folders));

        Self { tx: cmd_tx }
    }

    pub fn add_folder(&self, path: PathBuf) {
        let _ = self.tx.send(WatcherCommand::AddFolder(path));
    }

    pub fn remove_folder(&self, path: PathBuf) {
        let _ = self.tx.send(WatcherCommand::RemoveFolder(path));
    }
}

fn run_watcher<R: Runtime>(
    msg_rx: mpsc::Receiver<ThreadMessage>,
    msg_tx: mpsc::Sender<ThreadMessage>,
    app: AppHandle<R>,
    db: DbHandle,
    initial_folders: Vec<PathBuf>,
) {
    let event_tx = msg_tx;
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = event_tx.send(ThreadMessage::FileEvent(res));
    }) {
        Ok(w) => w,
        Err(e) => {
            let _ = app.emit(
                "library://error",
                format!("No se pudo iniciar el watcher: {e}"),
            );
            return;
        }
    };

    for folder in initial_folders {
        let _ = watcher.watch(&folder, RecursiveMode::Recursive);
    }

    let mut pending: HashSet<PathBuf> = HashSet::new();

    while let Ok(first) = msg_rx.recv() {
        let mut messages = vec![first];

        let deadline = Instant::now() + DEBOUNCE_WINDOW;
        loop {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            match msg_rx.recv_timeout(deadline - now) {
                Ok(msg) => messages.push(msg),
                Err(_) => break,
            }
        }

        let mut updated = false;

        for message in messages {
            match message {
                ThreadMessage::Command(WatcherCommand::AddFolder(folder)) => {
                    if watcher.watch(&folder, RecursiveMode::Recursive).is_ok() {
                        scanner::scan_folder(&db, &app, &folder);
                        updated = true;
                    }
                }
                ThreadMessage::Command(WatcherCommand::RemoveFolder(folder)) => {
                    let _ = watcher.unwatch(&folder);
                }
                ThreadMessage::FileEvent(Ok(event)) => {
                    if !event.kind.is_access() {
                        pending.extend(event.paths);
                    }
                }
                ThreadMessage::FileEvent(Err(_)) => {}
            }
        }

        if !pending.is_empty() {
            process_pending(&db, &app, &mut pending);
            updated = true;
        }

        if updated {
            let _ = app.emit("library://updated", ());
        }
    }
}

fn process_pending<R: Runtime>(db: &DbHandle, app: &AppHandle<R>, pending: &mut HashSet<PathBuf>) {
    for path in pending.drain() {
        if !scanner::is_supported_audio_file(&path) {
            continue;
        }

        if path.is_file() {
            let _ = scanner::upsert_track_file(db, app, &path);
        } else {
            let _ = scanner::remove_track_by_path(db, &path);
        }
    }
}
