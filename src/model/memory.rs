#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::model::state::{GameState, Plantation, Pos};

/// Состояние, которое сохраняется между ходами.
///
/// Поля делятся на две категории:
///  - **state-specific** (сбрасываются при респавне, Fix 7): `birth_turn`,
///    `our_construction_ever`, `construction_prev`, `our_prev_hp`,
///    `last_seen_enemy`, `last_seen_beaver`, `enemy_first_seen`.
///  - **глобальные** (сохраняются): `known_mountains`, `storm_history`,
///    `sabotages_done`, `beavers_killed`, `mains_lost`, `last_submitted_turn`.
#[derive(Debug, Default)]
pub struct Memory {
    /// Первое появление нашей плантации в observed state.
    pub birth_turn: HashMap<String, u32>,

    /// Все когда-либо замеченные горы (сервер шлёт только в VR, копим).
    pub known_mountains: HashSet<Pos>,

    /// id → (pos, hp, turn последнего наблюдения)
    pub last_seen_enemy: HashMap<String, (Pos, i32, u32)>,
    pub last_seen_beaver: HashMap<String, (Pos, i32, u32)>,

    /// Ход, когда мы впервые увидели вражеского id (Fix 2 — эвристика иммунитета).
    pub enemy_first_seen: HashMap<String, u32>,

    /// Прогресс наших строек в прошлом ходу (для детекции застрявших).
    pub construction_prev: HashMap<Pos, (i32, u32)>,

    /// HP наших в прошлом ходу (для детекции саботажа).
    pub our_prev_hp: HashMap<String, (i32, u32)>,

    /// История центров бурь для экстраполяции.
    pub storm_history: HashMap<String, Vec<(u32, Pos)>>,

    /// Счётчики для метрик/побед.
    pub sabotages_done: u32,
    pub beavers_killed: u32,
    pub mains_lost: u32,

    /// Защита от двойной отправки в том же turn_no.
    pub last_submitted_turn: Option<u32>,

    /// Клетки, где мы когда-либо инициировали стройку.
    /// Нужно, чтобы отличать нашу стройку от чужой в `construction[]`.
    /// Чистится в `update()` от клеток, которых больше нет (Fix 6).
    pub our_construction_ever: HashSet<Pos>,

    /// id ЦУ в прошлом ходу. Используется для детекта респавна при `N==1`
    /// (когда наш старый ЦУ был уничтожен и заменён свежим — разные id).
    pub last_main_id: Option<String>,
}

impl Memory {
    /// Основная обновляющая функция — вызывается в main loop на каждом ходу
    /// **после** `handle_respawn_if_detected`.
    pub fn update(&mut self, state: &GameState) {
        let t = state.turn_no;

        // 1. Возраст наших плантаций.
        for p in &state.plantations {
            self.birth_turn.entry(p.id.clone()).or_insert(t);
        }
        let alive: HashSet<String> = state.plantations.iter().map(|p| p.id.clone()).collect();
        self.birth_turn.retain(|id, _| alive.contains(id));

        // 2. Горы (копим).
        for m in &state.mountains {
            self.known_mountains.insert(*m);
        }

        // 3. Враги и бобры.
        for e in &state.enemies {
            self.last_seen_enemy
                .insert(e.id.clone(), (e.pos, e.hp, t));
            self.enemy_first_seen
                .entry(e.id.clone())
                .or_insert(t);
        }
        for b in &state.beavers {
            self.last_seen_beaver
                .insert(b.id.clone(), (b.pos, b.hp, t));
        }
        // Чистим слишком старые (>50 ходов).
        self.last_seen_enemy
            .retain(|_, (_, _, seen_t)| t.saturating_sub(*seen_t) < 50);
        self.last_seen_beaver
            .retain(|_, (_, _, seen_t)| t.saturating_sub(*seen_t) < 50);
        // enemy_first_seen — чистим только если id не виден давно и не в last_seen
        let known: HashSet<String> = self.last_seen_enemy.keys().cloned().collect();
        self.enemy_first_seen.retain(|id, _| known.contains(id));

        // 4. Стройки — снимок.
        let mut new_constr = HashMap::new();
        for c in &state.construction {
            new_constr.insert(c.pos, (c.progress, t));
        }
        self.construction_prev = new_constr;

        // 5. HP наших.
        self.our_prev_hp = state
            .plantations
            .iter()
            .map(|p| (p.id.clone(), (p.hp, t)))
            .collect();

        // 6. Бури.
        for m in &state.meteo {
            if m.kind == "sandstorm" {
                if let (Some(id), Some(pos)) = (m.id.as_ref(), m.position) {
                    self.storm_history
                        .entry(id.clone())
                        .or_default()
                        .push((t, pos));
                }
            }
        }

        // 7. Fix 6: чистим `our_construction_ever` от клеток, которых больше нет
        //    ни в `construction`, ни в `plantations`. Иначе при повторной стройке
        //    на той же клетке (уже чужой) мы ошибочно сочтём её своей.
        let keep: HashSet<Pos> = state
            .construction
            .iter()
            .map(|c| c.pos)
            .chain(state.plantations.iter().map(|p| p.pos))
            .collect();
        self.our_construction_ever.retain(|p| keep.contains(p));

        // 8. Снимок id ЦУ для детекта респавна на следующем ходу (Fix 7, сценарий B).
        // Важно: НЕ перезаписываем None, если ЦУ временно отсутствует
        // (пауза между раундами, plantations=[]). Иначе при появлении ЦУ с
        // новым id в начале следующего раунда main_id_changed не сработает.
        if let Some(main) = state.plantations.iter().find(|p| p.is_main) {
            self.last_main_id = Some(main.id.clone());
        }
    }

    /// Fix 7: детект респавна. Два важных случая **НЕ**-респавна, которые
    /// тоже меняют `main_id`:
    ///   1. `relocateMain` — сервер делает соседнюю плантацию новой ЦУ;
    ///      её id был известен нам как «обычная плантация» (есть в `birth_turn`).
    ///   2. Первое появление ЦУ (last_main_id был `None`) — не респавн, а спавн.
    ///
    /// Настоящий respawn — **main_id сменился на совершенно новый id**,
    /// которого мы раньше не видели (сервер сгенерировал свежий).
    ///
    /// Вызывать **до** `update`, чтобы `birth_turn` ещё содержал плантации
    /// прошлого хода (включая того соседа, который стал новым ЦУ).
    pub fn handle_respawn_if_detected(&mut self, state: &GameState) {
        let current_main = state.plantations.iter().find(|p| p.is_main);
        let main_id_changed = match (current_main.map(|p| &p.id), self.last_main_id.as_ref()) {
            (Some(new_id), Some(old_id)) => new_id != old_id,
            _ => false,
        };
        // Если новый main_id уже известен нам как обычная плантация — это
        // relocate, не respawn.
        let new_id_is_known_neighbor = current_main
            .map(|p| self.birth_turn.contains_key(&p.id))
            .unwrap_or(false);
        let is_respawn = main_id_changed && !new_id_is_known_neighbor;

        if is_respawn {
            self.mains_lost += 1;
            tracing::warn!(
                turn = state.turn_no,
                prev_plantations = self.birth_turn.len(),
                prev_main = self.last_main_id.as_deref().unwrap_or(""),
                new_main = current_main.map(|p| p.id.as_str()).unwrap_or(""),
                "respawn detected (main_id changed), clearing state-specific memory"
            );
            self.birth_turn.clear();
            self.our_construction_ever.clear();
            self.construction_prev.clear();
            self.our_prev_hp.clear();
            self.last_seen_enemy.clear();
            self.last_seen_beaver.clear();
            self.enemy_first_seen.clear();
        }
    }

    pub fn oldest_plantation<'a>(&self, state: &'a GameState) -> Option<&'a Plantation> {
        state
            .plantations
            .iter()
            .min_by_key(|p| self.birth_turn.get(&p.id).copied().unwrap_or(u32::MAX))
    }

    pub fn is_our_construction(&self, pos: Pos) -> bool {
        self.our_construction_ever.contains(&pos)
    }

    /// Fix 2: консервативная эвристика «враг в иммунитете».
    ///
    /// Если мы видим id впервые и у него полный HP (>=50), скорее всего
    /// это свежепостроенная плантация, и иммунитет ещё действует (3 хода).
    /// При повторной встрече считаем иммунитет уже истёкшим.
    pub fn suspected_enemy_immunity(&self, enemy_id: &str, hp: i32) -> bool {
        match self.enemy_first_seen.get(enemy_id) {
            None => hp >= 50,
            Some(_) => false,
        }
    }
}
