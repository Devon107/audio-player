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

## Fase 2 — Lectura de metadatos y carátulas

- [ ] Implementar módulo `metadata::reader` usando `lofty` (o `symphonia::core::meta`) para extraer:
  - [ ] Título
  - [ ] Álbum
  - [ ] Artista
  - [ ] Género
  - [ ] Año, número de pista, duración (datos adicionales)
- [ ] Extraer imagen de carátula embebida (picture/APIC frame) y convertir a formato utilizable por el frontend (base64 o guardado en caché local)
- [ ] Manejar archivos sin metadatos o con tags corruptos (valores por defecto: nombre de archivo, "Desconocido", etc.)
- [ ] Cachear carátulas extraídas en disco (evitar re-extracción en cada escaneo)

## Fase 3 — Biblioteca musical y base de datos

- [ ] Diseñar esquema SQLite: tablas `tracks`, `albums`, `artists`, `genres`, `playlists`, `playlist_tracks`, `settings`
- [ ] Implementar módulo `db` con `rusqlite` (conexión, migraciones iniciales)
- [ ] Comando Tauri: seleccionar carpeta(s) local(es) mediante diálogo nativo (`tauri-plugin-dialog`)
- [ ] Implementar escaneo recursivo de carpetas con `walkdir`, filtrando extensiones de audio soportadas
- [ ] Insertar/actualizar pistas encontradas en la base de datos (con sus metadatos y ruta de carátula)
- [ ] Implementar watcher con `notify` para detectar cambios en carpetas monitoreadas (archivos añadidos/eliminados) y refrescar biblioteca
- [ ] Comandos Tauri para consultar biblioteca: listar pistas, filtrar por artista/álbum/género, búsqueda por texto
- [ ] Vista de biblioteca en frontend: lista de canciones, agrupación por álbum/artista/género
- [ ] Manejo de biblioteca grande: paginación o virtualización de listas en frontend

## Fase 4 — Reproducción, cola, aleatorio y repetición

- [ ] Implementar estado de "cola de reproducción" (queue) en backend o frontend (definir fuente de verdad)
- [ ] Comandos: agregar a la cola, quitar de la cola, reordenar cola, reproducir pista específica de la cola
- [ ] Implementar reproducción secuencial automática (siguiente pista al terminar la actual)
- [ ] Implementar modo aleatorio (shuffle) — algoritmo de mezcla sin repetición hasta agotar la cola
- [ ] Implementar modos de repetición: repetir pista actual, repetir cola completa, sin repetición
- [ ] Botones de control en UI: anterior, siguiente, play/pause, shuffle, repeat, barra de progreso (seek), volumen
- [ ] Sincronizar estado de reproducción entre backend y frontend en tiempo real (eventos)

## Fase 5 — Playlists

- [ ] Comandos Tauri: crear, renombrar, eliminar playlist
- [ ] Comandos Tauri: agregar/quitar pistas de una playlist, reordenar pistas dentro de playlist
- [ ] Persistir playlists en base de datos (tabla `playlists` / `playlist_tracks`)
- [ ] UI: vista de playlists (sidebar o sección dedicada), drag-and-drop para reordenar/agregar canciones
- [ ] Reproducir playlist completa (cargar como cola)
- [ ] Exportar/importar playlists (opcional: formato `.m3u`)

## Fase 6 — Ecualizador

- [ ] Definir bandas de frecuencia del ecualizador (ej. 10 bandas: 31Hz–16kHz)
- [ ] Implementar filtros biquad (peaking EQ) en Rust, aplicados al stream PCM antes de la salida de audio
- [ ] Insertar el procesamiento de EQ en la cadena de audio entre decodificación y `rodio`/`cpal` output
- [ ] Comandos Tauri para ajustar ganancia de cada banda en tiempo real
- [ ] Presets de ecualizador (Rock, Pop, Jazz, Plano, Personalizado) guardados en `settings`
- [ ] UI: sliders verticales por banda, selector de preset
- [ ] Persistir configuración de EQ entre sesiones

## Fase 7 — Internacionalización (selector de idioma Inglés/Español)

- [ ] Configurar `i18next` + `react-i18next` en el frontend
- [ ] Crear archivos de traducción `en.json` y `es.json` con todas las cadenas de la UI
- [ ] Implementar selector de idioma en configuración/ajustes de la app
- [ ] Persistir preferencia de idioma (localStorage o tabla `settings` vía backend)
- [ ] Detectar idioma del sistema como valor por defecto en primer arranque
- [ ] Revisar que fechas/números se formateen según locale si aplica

## Fase 8 — UI/UX general

- [ ] Diseñar layout principal: sidebar de navegación (Biblioteca, Playlists, Ajustes), panel central de listado, reproductor fijo inferior (mini-player)
- [ ] Vista "Now Playing" con carátula grande, título, álbum, artista, barra de progreso
- [ ] Tema claro/oscuro (opcional pero recomendado dado uso de Tailwind)
- [ ] Estados vacíos (biblioteca sin canciones, playlist vacía) con mensajes claros
- [ ] Indicadores de carga durante escaneo de biblioteca
- [ ] Atajos de teclado (espacio = play/pause, flechas = siguiente/anterior)

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
