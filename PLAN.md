# Plan de Construcción — Reproductor de Audio MP3 Multiplataforma (Tauri + Symphonia)

## Stack tecnológico propuesto

- **Shell/Empaquetado**: Tauri 2.x (Windows, macOS, Linux)
- **Backend**: Rust
  - `symphonia` — demuxing/decoding de audio y lectura de metadatos (ID3v2, etc.)
  - `rodio` o `cpal` — salida de audio (Symphonia solo decodifica, no reproduce)
  - `lofty` — lectura/escritura de metadatos y carátulas embebidas (complemento a Symphonia, más robusto para tags)
  - `rusqlite` (o `sqlx` con SQLite) — persistencia de biblioteca, playlists, configuración
  - `walkdir` + `notify` — escaneo de carpetas locales y detección de cambios
  - `serde` / `serde_json` — serialización para comunicación IPC con el frontend
- **Frontend**: React + TypeScript + Tailwind CSS
  - `zustand` — estado global (cola, reproducción, biblioteca)
  - `i18next` / `react-i18next` — selector de idioma Inglés/Español
- **Comunicación**: Tauri commands (invoke) + eventos (emit/listen) para progreso de reproducción y escaneo

> Nota: este stack es una recomendación razonable; si prefieres otro framework de frontend (Svelte, Vue) o SQLite alternativo, la arquitectura general no cambia.

---

## Fase 0 — Preparación del proyecto ✅

- [x] Instalar prerequisitos (Rust vía `rustup`, Node.js ya presente, Bun como gestor de paquetes, dependencias del sistema para Tauri vía `apt`: webkit2gtk, gtk3, libsoup, etc.)
- [x] Inicializar repositorio git
- [x] Crear proyecto Tauri (`bun create tauri-app`) con plantilla React + TypeScript
- [x] Configurar linting/formatting (ESLint 9 flat config + Prettier, `rustfmt`, `clippy`)
- [x] Configurar estructura de carpetas (`src/` frontend, `src-tauri/src/` backend organizado por módulos: `audio`, `library`, `playlist`, `metadata`, `db`)
- [x] Añadir dependencias Rust iniciales al `Cargo.toml`: `symphonia`, `rodio`, `lofty`, `rusqlite`, `walkdir`, `notify`, `serde`
- [x] Verificar compilación y arranque (`bun run tauri dev` compila 546 crates sin errores y ejecuta el binario)

## Fase 1 — Motor de audio (backend Rust) ✅

- [x] Implementar módulo `audio::decoder` (`TrackDecoder`) usando `symphonia::core::formats::probe` para abrir y detectar formato del archivo
- [x] Implementar decodificación de paquetes a buffers PCM interleaved `f32` (`GenericAudioBufferRef::copy_to_vec_interleaved`), expuesto como `rodio::Source`
- [x] Implementar módulo `audio::output` con `rodio`/`cpal` (`DeviceSinkBuilder` + `Player`) corriendo en un hilo dedicado (el `cpal::Stream` no es `Send`/`Sync` en todas las plataformas)
- [x] Implementar controles básicos: play, pause, stop, seek (`Source::try_seek` sobre el `FormatReader` de Symphonia)
- [x] Implementar control de volumen (`Player::set_volume`)
- [x] Emitir eventos de progreso de reproducción (`player://progress`, `player://loaded`) hacia el frontend vía Tauri events, cada 250ms
- [x] Manejar fin de pista (evento `player://track-ended`) detectando cuando la cola de `Player` queda vacía
- [x] Pruebas unitarias del decodificador con un MP3 de muestra generado con ffmpeg (`src-tauri/tests/fixtures/test-tone.mp3`): specs de stream, conteo de muestras, seek, archivo inexistente
- [x] Prueba de humo manual (`cargo test -- --ignored`) que reproduce el tono por el dispositivo de audio real — confirmado audible por el usuario

## Fase 2 — Lectura de metadatos y carátulas ✅

- [x] Implementar módulo `metadata::reader` usando `lofty` para extraer:
  - [x] Título
  - [x] Álbum
  - [x] Artista
  - [x] Género
  - [x] Año, número de pista, duración (datos adicionales)
- [x] Extraer imagen de carátula embebida (picture/APIC frame vía `lofty::picture`) — `metadata::reader::extract_cover_bytes`
- [x] Manejar archivos sin metadatos o con tags corruptos: el título cae al nombre del archivo; el resto de campos quedan en `None` (sin texto placeholder codificado en el backend, para no romper el selector de idioma EN/ES — esa lógica de "Desconocido"/"Unknown" vive en el frontend con i18n)
- [x] Cachear carátulas extraídas en disco (`metadata::cover_cache`, en `app_cache_dir()/covers/`, con clave por ruta+tamaño+mtime para invalidar si el archivo cambia)

## Fase 3 — Biblioteca musical y base de datos ✅

- [x] Diseñar esquema SQLite: tablas `tracks`, `albums`, `artists`, `genres`, `playlists`, `playlist_tracks`, `settings`, y `watched_folders` (necesaria para persistir qué carpetas vigilar entre sesiones)
- [x] Implementar módulo `db` con `rusqlite` (`db::connection::DbHandle` — conexión compartida vía `Arc<Mutex<Connection>>` — y aplicación del esquema en `db/schema.sql` vía `execute_batch`, idempotente con `CREATE TABLE IF NOT EXISTS`)
- [x] Comando Tauri `pick_and_add_folder`: selecciona carpeta con diálogo nativo (`tauri-plugin-dialog`, invocado desde Rust como comando `async` — ver nota de bug en Fase 8) y dispara su escaneo inicial
- [x] Escaneo recursivo con `walkdir` (`library::scanner`), filtrando por extensión (mp3, flac, ogg, wav, m4a, aac, mp4), con detección de cambios por tamaño+mtime para saltar archivos sin modificar en re-escaneos
- [x] Insertar/actualizar pistas en la base de datos con metadatos (`lofty`) y ruta de carátula cacheada (`metadata::cover_cache`); get-or-create de artistas/álbumes/géneros, `ON CONFLICT` upsert por ruta
- [x] Watcher con `notify` (`library::watcher::LibraryWatcherHandle`) corriendo en hilo dedicado, con debounce de 500ms para agrupar ráfagas de eventos, que agrega/actualiza/elimina pistas automáticamente y emite `library://updated`
- [x] Comandos Tauri de consulta (`library::queries` + `library::commands`): `list_tracks` (con filtro por artista/álbum/género, búsqueda de texto y paginación limit/offset), `list_artists`, `list_albums`, `list_genres`, `list_watched_folders`, `remove_watched_folder`, `rescan_library`
- [x] Vista de biblioteca en frontend (`LibraryView`/`TrackTable`): búsqueda con debounce, filtros por artista/álbum/género, chips de carpetas vigiladas con opción de quitar, spinner mientras escanea una carpeta recién agregada
- [x] Progreso en vivo durante el escaneo: `scan_folder` ya guardaba cada pista en SQLite una por una, pero solo avisaba al frontend una vez al terminar toda la carpeta — el usuario no tenía forma de saber si el escaneo de una carpeta grande estaba avanzando o colgado. Se agregó el evento `library://scan-progress`, emitido a intervalos de 300ms con el conteo parcial (`ScanSummary` reutilizado), que dispara un refresco incremental de la tabla (las canciones van apareciendo mientras el escaneo sigue) y actualiza el texto del botón ("Escaneando… N encontradas") en vez de solo un mensaje estático. Bug detectado al probar esto en vivo: la ventana se congelaba por completo (ni siquiera respondía a clics) apenas se intentaba reproducir una canción durante el escaneo. Causa raíz: en esta versión de Tauri, los comandos `#[tauri::command]` no-`async` corren en el hilo principal de la UI; `list_tracks`/`list_artists`/`list_albums`/`list_genres`/`rescan_library`/`list_watched_folders`/`remove_watched_folder` eran todos sincrónicos, así que el refresco cada 300ms disparaba 4 de esos comandos en el hilo principal en bucle durante todo el escaneo, saturándolo (mismo motivo, ya documentado, por el que `pick_and_add_folder` tuvo que ser `async`). Se corrigió convirtiendo todos los comandos de `library::commands` a `async` — el resto de la app no necesitó cambios porque `invoke()` en el frontend ya trata cualquier comando como una promesa, sin importar si el lado Rust es sync o async
- [x] Manejo de biblioteca grande: paginación básica vía `limit`/`offset` (tope de 1000 pistas por consulta); **no** se implementó virtualización de listas en el DOM — con bibliotecas muy grandes (decenas de miles de pistas) el render de la tabla podría volverse pesado

## Fase 4 — Reproducción, cola, aleatorio y repetición ✅

- [x] Fuente de verdad: la cola vive en el backend (`audio::queue::QueueState`), dentro del mismo hilo dedicado del motor de audio (`audio::output`) que ya manejaba play/pause/stop/seek desde la Fase 1 — así el avance automático funciona sin depender de que el frontend esté escuchando
- [x] Comandos Tauri: `set_queue` (reemplaza la cola y puede arrancar en un índice), `add_to_queue`, `remove_from_queue`, `reorder_queue`, `clear_queue`, `play_queue_item`, `get_queue_state`
- [x] Reproducción secuencial automática: al vaciarse el `Player` (pista terminada), el motor llama a `queue.next()` y carga la siguiente sola, sin intervención del frontend
- [x] Modo aleatorio: `QueueState` mantiene una "bolsa" (`shuffle_bag`) con los ids pendientes del ciclo actual, mezclada con `rand`; garantiza visitar cada pista una vez antes de repetir
- [x] Modos de repetición: `RepeatMode::{Off, Track, Queue}` vía `set_repeat_mode`
- [x] Historial de reproducción (`previous_track`) independiente del modo aleatorio/secuencial, saltando pistas que fueron removidas de la cola mientras tanto
- [x] Botones de control en UI (`PlayerBar`): anterior, siguiente, play/pause, shuffle, repeat (cíclico off→queue→track), barra de progreso con seek (arrastre), volumen — el volumen usa una curva cúbica (posición del slider³) al convertir a la ganancia lineal real, porque el oído percibe el volumen logarítmicamente y una ganancia lineal se sentía "sin cambios" hasta cerca del máximo
- [x] Panel de cola (`QueuePanel`) con reordenamiento por drag-and-drop nativo (HTML5), quitar ítems y saltar a cualquier pista
- [x] Sincronización en tiempo real vía eventos Tauri: `player://loaded`, `player://progress`, `player://track-ended`, `player://queue-changed` (con el snapshot completo: ítems, pista actual, shuffle, repeat), `player://error`

## Fase 5 — Playlists ✅

- [x] Comandos Tauri: `create_playlist`, `rename_playlist`, `delete_playlist`
- [x] Comandos Tauri: `add_tracks_to_playlist`, `remove_track_from_playlist`, `reorder_playlist_track` (reescribe posiciones 0..n dentro de una transacción, sin depender de aritmética de huecos)
- [x] Persistencia en `playlists`/`playlist_tracks` (ya creadas en el esquema de la Fase 3); `playlist::queries` expone las operaciones como funciones puras sobre `&Connection`, testeables sin el harness de Tauri (mismo patrón que `library::queries`)
- [x] UI: vista de playlists (`PlaylistList` + `PlaylistDetail`), crear/renombrar (doble clic)/eliminar (con confirmación nativa), agregar canciones desde la biblioteca (menú por pista), reordenar con drag-and-drop nativo (HTML5)
- [x] Reproducir playlist completa: `play_playlist` arma un `Vec<QueueTrackInput>` desde `playlist_tracks` y lo manda como `AudioCommand::SetQueue` al motor de la Fase 4 (tuve que volver `pub(crate)` los módulos `audio::output`/`audio::queue`, antes privados, para que `playlist::commands` pudiera construir el comando)
- [x] Exportar/importar M3U (`playlist::m3u` + botones en `PlaylistDetail` usando el selector nativo de archivos vía `@tauri-apps/plugin-dialog` en el frontend): exporta `#EXTM3U` con `#EXTINF` (duración + "artista - título") y rutas absolutas; al importar, las pistas que ya están en la biblioteca se enlazan directamente y las que faltan se escanean y agregan automáticamente vía `library::scanner::upsert_track_file`; las entradas cuyo archivo ya no existe se omiten sin fallar la importación

## Fase 6 — Ecualizador ✅

- [x] 10 bandas ISO estándar (31Hz–16kHz, `audio::equalizer::BAND_FREQUENCIES`), rango de ganancia ±12dB
- [x] Filtros biquad "peaking EQ" (fórmulas del Audio EQ Cookbook de Robert Bristow-Johnson) en Rust puro, con estado de filtro independiente por canal (evita mezclar izquierda/derecha en estéreo)
- [x] `audio::equalizer::EqualizerSource<S>` envuelve el `TrackDecoder` (implementa `rodio::Source`) e inserta el procesamiento de EQ justo antes de llegar al `Player`; reinicia el estado de los filtros en cada `seek` para evitar clics
- [x] Comandos Tauri: `set_eq_band_gain` (ganancia en tiempo real vía `EqualizerControl`, atómicos sin locks para no bloquear el hilo de audio), `set_eq_preset`, `get_eq_state`
- [x] Presets Flat/Rock/Pop/Jazz con curvas propias; `Custom` no tiene curva — indica que el usuario ajustó bandas a mano. Persistidos en `settings` (`eq_gains` como JSON, `eq_preset`) vía el nuevo helper `db::settings`, y recargados al iniciar la app
- [x] UI: 10 sliders verticales (`EqualizerView`, sliders horizontales rotados 90° por CSS — `-webkit-appearance: slider-vertical` no se veía bien en el WebView de Linux) con relleno de progreso y selector de preset
- [x] Persistencia entre sesiones (ver arriba)

En el frontend, el envío de cada cambio de ganancia al backend está debounced (~120ms): mandar cada micro-movimiento del slider disparaba una escritura sincrónica a SQLite por evento, lo que se sentía como lag al arrastrar.

Verificación: pruebas unitarias con respuesta en frecuencia calculada analíticamente (sin necesidad de simular audio) confirman que a 0dB el filtro es la identidad exacta y que el boost/cut se concentra en la banda correspondiente; prueba manual con audio real confirmó el cambio de volumen audible al mover una banda entre -12dB y +12dB.

## Fase 7 — Internacionalización (selector de idioma Inglés/Español) ✅

- [x] Configurar `i18next` + `react-i18next` en el frontend (`src/i18n/index.ts`)
- [x] Crear archivos de traducción `en.json` y `es.json` con las cadenas de la UI (`src/i18n/locales/`)
- [x] Implementar selector de idioma en Ajustes (`SettingsView`)
- [x] Persistir preferencia de idioma vía backend: `settings::commands::get_language_preference` / `set_language_preference` (reutiliza `db::settings` de la Fase 6; valida contra `en`/`es`, devuelve `None` si el usuario nunca eligió explícitamente)
- [x] Detectar idioma del sistema como valor por defecto en primer arranque: `i18n/index.ts` usa `navigator.language` de forma síncrona al iniciar, y solo lo reemplaza si hay una preferencia guardada en el backend
- [ ] Revisar que fechas/números se formateen según locale — no aplica todavía: la UI actual no muestra fechas (p. ej. `created_at`/`updated_at` de playlists) en ningún lado

## Fase 8 — UI/UX general ✅

- [x] Layout principal: sidebar de navegación (Biblioteca, Playlists, Ecualizador, Ajustes), panel central por vista, reproductor fijo inferior (`PlayerBar`) + panel de cola deslizable
- [x] Vista "Now Playing" dedicada con carátula grande (`NowPlayingView`) — se abre al hacer clic en la carátula/título de la `PlayerBar`, con botón de volver a la vista anterior
- [x] Tema oscuro (paleta con acento naranja neón, ajustada a pedido durante la revisión). **No** se implementó selector claro/oscuro — quedó fijo en oscuro
- [x] Estados vacíos (sin carpetas, sin pistas, playlist vacía, sin playlists) con mensajes vía i18n
- [x] Indicador de carga durante el escaneo inicial de una carpeta agregada (spinner + mensaje) — el bug que parecía "no encuentra los archivos" era en realidad esto: el escaneo de 545 canciones tardaba varios segundos sin ningún feedback visual
- [x] Atajos de teclado: espacio = play/pause, flechas izq/der = anterior/siguiente (`useGlobalShortcuts`), ignorados mientras el foco está en un campo de texto/select/slider para no pisar su interacción nativa
- [x] Integración con los controles nativos de medios del SO (`audio::media_controls`, crate `souvlaki`): teclas multimedia del teclado y botones de dispositivos Bluetooth (parlantes/auriculares con play/pause/siguiente/anterior vía AVRCP) controlan la app — MPRIS en Linux, SMTC en Windows, Now Playing en macOS con una sola API. El motor de audio (`run_engine`) reporta título/artista/álbum/carátula/posición al SO en cada tick y reenvía sus eventos como `AudioCommand`s; probado en vivo en Linux (teclado y parlante), Windows/macOS solo compilados — sin probar, quedan pendientes para cuando haya esas plataformas disponibles. Efecto secundario detectado y corregido: el ícono de play/pause de la UI no se actualizaba cuando la reproducción se togglaba desde afuera de la propia app (teclado/Bluetooth), porque `isPlaying` en el frontend solo se seteaba de forma optimista al iniciar la acción desde la UI misma. Se agregó `is_playing` al evento `player://progress` (ya emitido cada 250ms) como fuente de verdad real del backend.

Bugs encontrados y corregidos durante la revisión manual con el usuario (primera vez que se ejerció la UI end-to-end):

- `pick_and_add_folder` crasheaba la app: el diálogo nativo (`blocking_pick_folder`) bloqueaba el hilo principal al ser un comando Tauri sincrónico; la documentación del plugin exige que los métodos `blocking_*` se llamen desde un comando `async` (que Tauri despacha a un hilo del runtime, no al principal). Se corrigió agregando `async` a la firma y `State<'_, T>` en los parámetros.
- Los `<select>` nativos se veían con fondo blanco y texto claro ilegible: faltaba `color-scheme: dark` en el HTML raíz.
- `accent-color` no se renderiza de forma confiable en el WebView de Linux (WebKitGTK): se reemplazó por estilos manuales vía pseudo-elementos `::-webkit-slider-thumb`/`::-webkit-slider-runnable-track` (soportados también por WebView2 y WKWebView, los tres webviews de escritorio de Tauri).
- El volumen "no hacía nada" perceptiblemente hasta cerca del máximo: no era un bug, la ganancia lineal de rodio no coincide con la percepción logarítmica del oído. Se aplicó una curva cúbica en el frontend al convertir la posición del slider a la ganancia real.
- Activar aleatorio con una cola ya en curso no reordenaba la lista visible: `QueueState` llevaba una "bolsa" de orden interno (`shuffle_bag`) separada de `items`, así que la cola que veía el usuario no cambiaba. Se rediseñó para mezclar físicamente `items` (dejando la pista actual primero) al activar `shuffle`, y para re-mezclar al llegar al final de la cola cuando `shuffle` + `repeat: queue` están activos a la vez.
- El menú "Agregar a playlist" de una pista solo listaba playlists existentes; para crear una había que ir primero a la vista Playlists. Se agregó un formulario inline (nombre + botón "Crear") dentro del mismo menú desplegable.
- El botón de repetir tenía dos estados activos (naranja) visualmente casi idénticos — el dígito "1" de "repetir pista" se veía tan pequeño que se confundía con una "T", y sin pasar el mouse por el tooltip no se distinguía de "repetir cola". Se le agregó un círculo relleno de fondo al ícono de "repetir pista" para que el modo se note de un vistazo.
- El área de clic vertical de la barra de progreso era demasiado angosta (calcada al grosor visual de 4px) y costaba acertarle con el mouse. Se agrandó el hitbox del `<input type="range">` sin tocar el grosor visual del track (fijado aparte por CSS).
- Al presionar "anterior" en la primera o única pista de la cola, `QueueState::previous()` devolvía `None` pero el motor igual llamaba a `player.stop()` incondicionalmente, cortando la reproducción en curso sin ninguna señal al frontend (por eso el botón "play" quedaba desincronizado). Se agregó `QueueState::has_previous()`, se expuso en el snapshot de la cola, se deshabilita el botón "anterior" en el frontend cuando no hay historial navegable, y el motor ya no toca el reproductor cuando `previous()` no devuelve pista.

## Fase 9 — Empaquetado y distribución multiplataforma

- [ ] Configurar `tauri.conf.json` (íconos, identificador de bundle, permisos de filesystem/dialog)
- [ ] Generar builds para Windows (.msi/.exe), macOS (.dmg), Linux (.AppImage/.deb)
- [ ] Verificar permisos de acceso a sistema de archivos en cada plataforma
- [ ] Probar rendimiento de audio (latencia, glitches) en cada SO
- [ ] Firmar/notarizar build de macOS si se distribuye fuera de la App Store (opcional)

## Fase 10 — Pruebas y pulido final

- [ ] Pruebas manuales de reproducción con distintos MP3 (bitrates variados, VBR, con/sin tags)
- [ ] Pruebas de biblioteca con carpetas grandes (rendimiento de escaneo)
- [ ] Pruebas de cola, shuffle y repeat en casos límite (cola vacía, una sola pista)
- [ ] Pruebas de EQ (verificar que los cambios se escuchan sin artefactos/clicks)
- [ ] Revisión de traducciones ES/EN completas
- [ ] Corrección de bugs y optimización final antes de release

---

## Extensiones futuras (fuera del alcance inicial)

- Soporte para otros formatos (FLAC, OGG, WAV) — Symphonia ya los soporta a nivel de decodificación
- Sincronización de biblioteca en la nube
- Letras de canciones (lyrics)
- Visualizador de espectro de audio
