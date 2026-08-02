use std::path::Path;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

/// Pista dentro de la cola de reproducción. `track_id` referencia la fila en la base de datos
/// de la biblioteca cuando la pista viene de ahí (para que el frontend pueda pedir sus
/// metadatos); puede ser `None` si se agregó un archivo suelto que no está en la biblioteca.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueueTrack {
    pub id: u64,
    pub path: String,
    pub track_id: Option<i64>,
}

/// Lo que recibe la cola desde el frontend para agregar una pista (sin `id`, que es asignado
/// internamente). También `Serialize` para poder persistir la cola en `settings` y restaurarla
/// al reiniciar la app — ahí no tiene sentido guardar el `id` interno, que se reasigna desde 0
/// cada vez que arranca el motor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTrackInput {
    pub path: String,
    pub track_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

/// Estado de la cola de reproducción: orden de las pistas, cuál está sonando, modo aleatorio y
/// modo de repetición. No sabe nada de audio real; solo decide "cuál sigue" — `audio::output` es
/// quien usa esto para cargar y reproducir la pista resultante.
///
/// El modo aleatorio reordena físicamente `items` (dejando la pista actual primero) en vez de
/// llevar un orden de reproducción oculto: así la cola que ve el usuario en la UI siempre refleja
/// el orden real en el que se va a reproducir.
#[derive(Default)]
pub struct QueueState {
    items: Vec<QueueTrack>,
    next_id: u64,
    current_id: Option<u64>,
    /// Pila de ids reproducidos anteriormente, para poder retroceder con `previous()`.
    history: Vec<u64>,
    shuffle: bool,
    repeat: RepeatMode,
}

impl QueueState {
    pub fn items(&self) -> &[QueueTrack] {
        &self.items
    }

    pub fn current_id(&self) -> Option<u64> {
        self.current_id
    }

    pub fn current(&self) -> Option<&QueueTrack> {
        self.current_id.and_then(|id| self.find(id))
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    /// Reemplaza toda la cola. Reinicia posición actual e historial.
    pub fn set_items(&mut self, inputs: Vec<QueueTrackInput>) {
        let mut items = Vec::with_capacity(inputs.len());
        for input in inputs {
            let id = self.alloc_id();
            items.push(QueueTrack {
                id,
                path: input.path,
                track_id: input.track_id,
            });
        }
        self.items = items;
        self.current_id = None;
        self.history.clear();
    }

    /// Agrega pistas al final de la cola sin afectar lo que está sonando.
    pub fn add_items(&mut self, inputs: Vec<QueueTrackInput>) {
        for input in inputs {
            let id = self.alloc_id();
            self.items.push(QueueTrack {
                id,
                path: input.path,
                track_id: input.track_id,
            });
        }
    }

    /// Quita una pista de la cola. Si era la que estaba sonando, se deja como `current_id` de
    /// todas formas (sigue sonando hasta terminar); simplemente desaparece de la lista visible.
    pub fn remove(&mut self, item_id: u64) {
        self.items.retain(|t| t.id != item_id);
        self.history.retain(|id| *id != item_id);
    }

    /// Quita de la cola todas las pistas cuyo archivo está bajo `root` (usado cuando se deja de
    /// vigilar una carpeta de la biblioteca). A diferencia de `remove()`, si la pista actual
    /// queda atrapada en el borrado, se limpia `current_id` en vez de dejarla terminar de sonar
    /// — la carpeta ya no es parte de la biblioteca, no tiene sentido seguir reproduciéndola en
    /// segundo plano. Devuelve `true` si la pista actual fue una de las quitadas.
    pub fn remove_under(&mut self, root: &Path) -> bool {
        let current_affected = self
            .current_id
            .and_then(|id| self.find(id))
            .is_some_and(|t| Path::new(&t.path).starts_with(root));

        self.items.retain(|t| !Path::new(&t.path).starts_with(root));
        self.history
            .retain(|id| self.items.iter().any(|t| t.id == *id));

        if current_affected {
            self.current_id = None;
        }

        current_affected
    }

    pub fn reorder(&mut self, item_id: u64, new_index: usize) {
        let Some(pos) = self.items.iter().position(|t| t.id == item_id) else {
            return;
        };
        let track = self.items.remove(pos);
        let clamped = new_index.min(self.items.len());
        self.items.insert(clamped, track);
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_id = None;
        self.history.clear();
    }

    /// Al activarlo, mezcla la cola dejando la pista actual (si hay una) primero, para que lo
    /// que se estaba escuchando no salte de golpe a otra cosa. Al desactivarlo se deja el orden
    /// tal cual quedó (no se intenta reconstruir el orden original).
    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        if enabled {
            self.shuffle_keeping_current_first();
        }
    }

    /// Igual que `set_shuffle`, pero sin el efecto colateral de mezclar `items`. Pensado
    /// exclusivamente para restaurar una cola ya guardada: esos `items` ya están en el orden
    /// (mezclado o no) en el que quedaron la sesión anterior, y en ese momento todavía no hay
    /// una pista actual establecida — si se llamara a `set_shuffle` normal, mezclaría toda la
    /// lista sin ancla (sin pista actual a la cual anteponer) y el índice de "pista actual"
    /// guardado terminaría apuntando a otra pista distinta en la lista recién remezclada.
    /// Aleatorio solo debe disparar una mezcla nueva cuando el usuario lo activa desde la UI.
    pub fn set_shuffle_flag(&mut self, enabled: bool) {
        self.shuffle = enabled;
    }

    /// Vuelve a mezclar la cola (dejando la pista actual primero) si el aleatorio está activo.
    /// Pensado para cuando `set_items` reemplaza toda la cola — por ejemplo, al elegir reproducir
    /// una pista desde la biblioteca — mientras el aleatorio ya estaba encendido: la cola nueva no
    /// hereda ninguna mezcla de la anterior, así que sin esto quedaría en el orden original hasta
    /// que el usuario apague y prenda el aleatorio a mano. Debe llamarse después de establecer
    /// `current_id` (vía `play_item`), no antes, porque `shuffle_keeping_current_first` usa ese
    /// id como ancla.
    pub fn reshuffle_if_active(&mut self) {
        if self.shuffle {
            self.shuffle_keeping_current_first();
        }
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Marca `item_id` como la pista actual (empujando la anterior al historial) y la devuelve.
    pub fn play_item(&mut self, item_id: u64) -> Option<QueueTrack> {
        let track = self.find(item_id)?.clone();
        if let Some(current) = self.current_id {
            if current != item_id {
                self.history.push(current);
            }
        }
        self.current_id = Some(item_id);
        Some(track)
    }

    /// Calcula y adopta la siguiente pista según el modo de repetición/aleatorio. Devuelve
    /// `None` si no hay nada más que reproducir (cola vacía o fin de cola sin repetición).
    ///
    /// Con aleatorio activado, avanza secuencialmente por `items` igual que sin aleatorio — el
    /// orden aleatorio ya está "cocinado" en `items` desde que se activó `shuffle` (o desde el
    /// último `set_items`). Al llegar al final con `repeat = Queue`, si además `shuffle` está
    /// activo, se vuelve a mezclar toda la cola para el siguiente ciclo.
    pub fn next(&mut self) -> Option<QueueTrack> {
        if self.items.is_empty() {
            self.current_id = None;
            return None;
        }

        if self.repeat == RepeatMode::Track {
            if let Some(track) = self.current() {
                return Some(track.clone());
            }
        }

        if let Some(id) = self.current_id {
            self.history.push(id);
        }

        let idx = self
            .current_id
            .and_then(|id| self.items.iter().position(|t| t.id == id));

        let next_id = match idx {
            None => self.items.first().map(|t| t.id),
            Some(i) if i + 1 < self.items.len() => Some(self.items[i + 1].id),
            Some(_) if self.repeat == RepeatMode::Queue => {
                if self.shuffle {
                    self.items.shuffle(&mut rand::rng());
                }
                self.items.first().map(|t| t.id)
            }
            Some(_) => None,
        };

        self.current_id = next_id;
        next_id.and_then(|id| self.find(id).cloned())
    }

    /// Retrocede al ítem anterior del historial, saltando los que ya no existen en la cola
    /// (porque fueron removidos).
    pub fn previous(&mut self) -> Option<QueueTrack> {
        while let Some(id) = self.history.pop() {
            if let Some(track) = self.items.iter().find(|t| t.id == id) {
                let track = track.clone();
                self.current_id = Some(id);
                return Some(track);
            }
        }
        None
    }

    /// Indica si `previous()` devolvería una pista real en vez de `None` — es decir, si queda
    /// algo en el historial que todavía exista en la cola. Se usa para deshabilitar el botón
    /// "anterior" en el frontend en vez de dejar que retroceder en la primera/única pista
    /// interrumpa la reproducción sin motivo.
    pub fn has_previous(&self) -> bool {
        self.history
            .iter()
            .any(|id| self.items.iter().any(|t| t.id == *id))
    }

    fn find(&self, id: u64) -> Option<&QueueTrack> {
        self.items.iter().find(|t| t.id == id)
    }

    fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn shuffle_keeping_current_first(&mut self) {
        if self.items.len() <= 1 {
            return;
        }

        let current = self.current_id;
        let mut rest: Vec<QueueTrack> = self
            .items
            .iter()
            .filter(|t| Some(t.id) != current)
            .cloned()
            .collect();
        rest.shuffle(&mut rand::rng());

        let mut reordered = Vec::with_capacity(self.items.len());
        if let Some(current_track) = current.and_then(|id| self.find(id)).cloned() {
            reordered.push(current_track);
        }
        reordered.extend(rest);
        self.items = reordered;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn inputs(paths: &[&str]) -> Vec<QueueTrackInput> {
        paths
            .iter()
            .map(|p| QueueTrackInput {
                path: (*p).to_string(),
                track_id: None,
            })
            .collect()
    }

    fn ids(state: &QueueState) -> Vec<u64> {
        state.items().iter().map(|t| t.id).collect()
    }

    #[test]
    fn sequential_next_walks_through_the_queue_in_order() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        let [id_a, id_b, id_c] = ids(&q)[..] else {
            panic!("se esperaban 3 ítems")
        };

        assert_eq!(q.next().map(|t| t.id), Some(id_a));
        assert_eq!(q.next().map(|t| t.id), Some(id_b));
        assert_eq!(q.next().map(|t| t.id), Some(id_c));
        assert_eq!(
            q.next(),
            None,
            "sin repeat, al final de la cola no hay siguiente"
        );
    }

    #[test]
    fn has_previous_reflects_whether_previous_would_return_a_track() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3"]));
        assert!(!q.has_previous(), "sin historial no hay pista anterior");

        q.next(); // a
        assert!(
            !q.has_previous(),
            "en la primera pista todavía no hay historial"
        );

        q.next(); // b
        assert!(q.has_previous(), "ahora sí hay una pista anterior (a)");

        q.previous();
        assert!(!q.has_previous(), "de vuelta en a, sin más historial");
    }

    #[test]
    fn repeat_queue_wraps_around_to_the_first_track() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3"]));
        q.set_repeat(RepeatMode::Queue);
        let id_a = q.items()[0].id;

        q.next();
        q.next();
        let wrapped = q.next();
        assert_eq!(wrapped.map(|t| t.id), Some(id_a));
    }

    #[test]
    fn repeat_track_keeps_returning_the_same_track() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3"]));
        let id_a = q.next().unwrap().id;
        q.set_repeat(RepeatMode::Track);

        assert_eq!(q.next().map(|t| t.id), Some(id_a));
        assert_eq!(q.next().map(|t| t.id), Some(id_a));
        assert_eq!(q.next().map(|t| t.id), Some(id_a));
    }

    #[test]
    fn previous_walks_back_through_history() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        let [id_a, id_b, id_c] = ids(&q)[..] else {
            panic!("se esperaban 3 ítems")
        };

        q.next(); // a
        q.next(); // b
        q.next(); // c
        assert_eq!(q.current_id(), Some(id_c));

        assert_eq!(q.previous().map(|t| t.id), Some(id_b));
        assert_eq!(q.previous().map(|t| t.id), Some(id_a));
        assert_eq!(
            q.previous(),
            None,
            "no hay más historial antes de la primera pista"
        );
    }

    #[test]
    fn enabling_shuffle_keeps_current_track_first_and_preserves_the_rest() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3", "e.mp3"]));
        let current = q.next().unwrap().id; // deja "a" como actual
        let original: HashSet<u64> = ids(&q).into_iter().collect();

        q.set_shuffle(true);

        assert_eq!(
            q.items()[0].id,
            current,
            "la pista actual debe quedar primera tras mezclar"
        );
        let after: HashSet<u64> = ids(&q).into_iter().collect();
        assert_eq!(after, original, "mezclar no debe perder ni duplicar ítems");
    }

    #[test]
    fn set_shuffle_flag_does_not_reorder_items() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3", "e.mp3"]));
        let original_order: Vec<u64> = ids(&q);

        q.set_shuffle_flag(true);

        assert!(q.shuffle(), "la bandera sí debe quedar activada");
        assert_eq!(
            ids(&q),
            original_order,
            "a diferencia de set_shuffle, no debería reordenar nada"
        );
    }

    #[test]
    fn reshuffle_if_active_reorders_a_freshly_replaced_queue_keeping_current_first() {
        let mut q = QueueState::default();
        q.set_shuffle_flag(true); // aleatorio ya estaba activo antes de elegir esta pista

        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3", "e.mp3"]));
        let original_order: Vec<u64> = ids(&q);
        let chosen = q.items()[2].id; // el usuario elige "c" de la biblioteca
        q.play_item(chosen);

        q.reshuffle_if_active();

        assert_eq!(
            q.items()[0].id,
            chosen,
            "la pista elegida debe quedar primera tras mezclar"
        );
        assert_ne!(
            ids(&q),
            original_order,
            "reemplazar la cola con aleatorio activo debe mezclarla, no dejarla en el orden original"
        );
    }

    #[test]
    fn reshuffle_if_active_does_nothing_when_shuffle_is_off() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3", "e.mp3"]));
        q.play_item(q.items()[2].id);
        let original_order: Vec<u64> = ids(&q);

        q.reshuffle_if_active();

        assert_eq!(
            ids(&q),
            original_order,
            "sin aleatorio no debe tocar el orden"
        );
    }

    #[test]
    fn shuffle_visits_every_track_exactly_once_before_repeating() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3"]));
        q.set_shuffle(true);

        let mut visited = Vec::new();
        for _ in 0..4 {
            visited.push(
                q.next()
                    .expect("debería haber pista mientras queden en la cola")
                    .id,
            );
        }

        let mut sorted = visited.clone();
        sorted.sort_unstable();
        let mut expected = ids(&q);
        expected.sort_unstable();
        assert_eq!(
            sorted, expected,
            "debe visitar cada pista exactamente una vez"
        );

        // Sin repeat, al llegar al final no hay más.
        assert_eq!(q.next(), None);
    }

    #[test]
    fn shuffle_with_repeat_queue_reshuffles_after_exhausting() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        q.set_shuffle(true);
        q.set_repeat(RepeatMode::Queue);

        // Recorre 2 ciclos completos sin quedarse sin pistas.
        for _ in 0..6 {
            assert!(q.next().is_some());
        }
    }

    #[test]
    fn removing_current_track_keeps_it_playing_but_hides_it_from_items() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3"]));
        let id_a = q.next().unwrap().id;

        q.remove(id_a);

        assert_eq!(
            q.current_id(),
            Some(id_a),
            "sigue sonando aunque ya no esté en la lista"
        );
        assert!(q.items().iter().all(|t| t.id != id_a));
        assert_eq!(q.items().len(), 1);
    }

    #[test]
    fn remove_under_clears_current_id_when_current_track_is_under_the_removed_root() {
        let mut q = QueueState::default();
        q.set_items(inputs(&[
            "/music/folder-a/a.mp3",
            "/music/folder-a/b.mp3",
            "/music/folder-b/c.mp3",
        ]));
        q.next(); // a, bajo folder-a

        let affected = q.remove_under(Path::new("/music/folder-a"));

        assert!(affected, "la pista actual estaba bajo la carpeta quitada");
        assert_eq!(
            q.current_id(),
            None,
            "a diferencia de remove(), acá sí se limpia: la carpeta ya no es parte de la biblioteca"
        );
        let paths: Vec<&str> = q.items().iter().map(|t| t.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["/music/folder-b/c.mp3"],
            "solo debería sobrevivir la pista fuera de folder-a"
        );
    }

    #[test]
    fn remove_under_leaves_current_id_untouched_when_not_affected() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["/music/folder-a/a.mp3", "/music/folder-b/c.mp3"]));
        q.next(); // a
        let id_a = q.current_id();
        q.next(); // c, bajo folder-b

        let affected = q.remove_under(Path::new("/music/folder-a"));

        assert!(
            !affected,
            "la pista actual (c) no está bajo la carpeta quitada"
        );
        assert_ne!(q.current_id(), id_a);
        assert_eq!(q.items().len(), 1);
        assert_eq!(q.items()[0].path, "/music/folder-b/c.mp3");
    }

    #[test]
    fn previous_skips_history_entries_that_were_removed() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        let id_a = q.items()[0].id;
        let id_b = q.items()[1].id;

        q.next(); // a
        q.next(); // b
        q.remove(id_a);
        q.next(); // c

        // El historial tiene [a, b], pero a ya no existe: previous() debe saltarlo.
        assert_eq!(q.previous().map(|t| t.id), Some(id_b));
    }

    #[test]
    fn reorder_moves_item_to_the_requested_position() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        let id_a = q.items()[0].id;

        q.reorder(id_a, 2);

        let paths: Vec<&str> = q.items().iter().map(|t| t.path.as_str()).collect();
        assert_eq!(paths, vec!["b.mp3", "c.mp3", "a.mp3"]);
    }

    #[test]
    fn play_item_pushes_previous_current_onto_history() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3"]));
        let id_a = q.items()[0].id;
        let id_c = q.items()[2].id;

        q.play_item(id_a);
        q.play_item(id_c);

        assert_eq!(q.current_id(), Some(id_c));
        assert_eq!(q.previous().map(|t| t.id), Some(id_a));
    }

    #[test]
    fn clear_resets_everything() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3"]));
        q.next();
        q.set_shuffle(true);

        q.clear();

        assert!(q.items().is_empty());
        assert_eq!(q.current_id(), None);
        assert_eq!(q.next(), None);
        assert_eq!(q.previous(), None);
    }

    #[test]
    fn empty_queue_next_returns_none() {
        let mut q = QueueState::default();
        assert_eq!(q.next(), None);
    }
}
