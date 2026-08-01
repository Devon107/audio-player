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
/// internamente).
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Default)]
pub struct QueueState {
    items: Vec<QueueTrack>,
    next_id: u64,
    current_id: Option<u64>,
    /// Pila de ids reproducidos anteriormente, para poder retroceder con `previous()`.
    history: Vec<u64>,
    /// Ids pendientes de reproducir en el ciclo aleatorio actual (se rellena cuando se vacía).
    shuffle_bag: Vec<u64>,
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

    /// Reemplaza toda la cola. Reinicia posición actual, historial y bolsa de aleatorio.
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
        self.shuffle_bag.clear();
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
        self.shuffle_bag.retain(|id| *id != item_id);
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
        self.shuffle_bag.clear();
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        if enabled {
            self.refill_shuffle_bag();
        } else {
            self.shuffle_bag.clear();
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
        self.shuffle_bag.retain(|id| *id != item_id);
        Some(track)
    }

    /// Calcula y adopta la siguiente pista según el modo de repetición/aleatorio. Devuelve
    /// `None` si no hay nada más que reproducir (cola vacía o fin de cola sin repetición).
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

        let next_id = if self.shuffle {
            if self.shuffle_bag.is_empty()
                && (self.repeat == RepeatMode::Queue || self.current_id.is_none())
            {
                self.refill_shuffle_bag();
            }
            self.shuffle_bag.pop()
        } else {
            let idx = self
                .current_id
                .and_then(|id| self.items.iter().position(|t| t.id == id));
            match idx {
                None => self.items.first().map(|t| t.id),
                Some(i) if i + 1 < self.items.len() => Some(self.items[i + 1].id),
                Some(_) if self.repeat == RepeatMode::Queue => self.items.first().map(|t| t.id),
                Some(_) => None,
            }
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

    fn find(&self, id: u64) -> Option<&QueueTrack> {
        self.items.iter().find(|t| t.id == id)
    }

    fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn refill_shuffle_bag(&mut self) {
        let mut ids: Vec<u64> = self.items.iter().map(|t| t.id).collect();
        if let Some(current) = self.current_id {
            ids.retain(|id| *id != current);
        }
        ids.shuffle(&mut rand::rng());
        self.shuffle_bag = ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn shuffle_visits_every_track_exactly_once_before_repeating() {
        let mut q = QueueState::default();
        q.set_items(inputs(&["a.mp3", "b.mp3", "c.mp3", "d.mp3"]));
        q.set_shuffle(true);

        let mut visited = Vec::new();
        for _ in 0..4 {
            visited.push(
                q.next()
                    .expect("debería haber pista mientras queden en la bolsa")
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

        // Sin repeat, al agotar la bolsa no hay más.
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
