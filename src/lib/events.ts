import { listen } from '@tauri-apps/api/event'
import type { LoadedPayload, ProgressPayload, QueueSnapshot } from './types'

// Envoltorios tipados sobre `listen()` para los eventos que emite el motor de audio
// (src-tauri/src/audio/output.rs) y el watcher de biblioteca (src-tauri/src/library/watcher.rs).
// Cada uno devuelve la función `unlisten` que ya da `listen()`, para limpiar en `useEffect`.

export function onPlayerLoaded(handler: (payload: LoadedPayload) => void) {
  return listen<LoadedPayload>('player://loaded', (event) => handler(event.payload))
}

export function onPlayerProgress(handler: (payload: ProgressPayload) => void) {
  return listen<ProgressPayload>('player://progress', (event) => handler(event.payload))
}

export function onPlayerTrackEnded(handler: () => void) {
  return listen('player://track-ended', () => handler())
}

export function onPlayerQueueChanged(handler: (snapshot: QueueSnapshot) => void) {
  return listen<QueueSnapshot>('player://queue-changed', (event) => handler(event.payload))
}

export function onPlayerError(handler: (message: string) => void) {
  return listen<string>('player://error', (event) => handler(event.payload))
}

export function onLibraryUpdated(handler: () => void) {
  return listen('library://updated', () => handler())
}

export function onLibraryError(handler: (message: string) => void) {
  return listen<string>('library://error', (event) => handler(event.payload))
}
