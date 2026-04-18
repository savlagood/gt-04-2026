#![allow(dead_code)]

use crate::config::Config;
use crate::geom::{cell_base_points, chebyshev};
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Plantation, Pos};
use crate::planner::tasks::{Phase, Task, TaskKind};

/// Фаза, на которой разрешена диверсия (по конфигу).
fn allowed(phase: Phase, cfg: &Config) -> bool {
    match phase {
        Phase::Early => cfg.sabotage.allowed_in_early,
        Phase::Growth => cfg.sabotage.allowed_in_growth,
        Phase::Harvest => cfg.sabotage.allowed_in_harvest,
        Phase::Endgame => true,
    }
}

/// Fix 2: эвристика иммунитета. В API у `EnemyPlantationDTO` **нет** поля
/// immunity — берём из `memory.suspected_enemy_immunity`:
/// первая встреча id с HP==MHP → вероятно в иммунитете.
pub fn generate_sabotage_tasks(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
    phase: Phase,
) -> Vec<Task> {
    if !allowed(phase, cfg) {
        return Vec::new();
    }
    let mut tasks = Vec::new();
    for e in &state.enemies {
        if memory.suspected_enemy_immunity(&e.id, e.hp) {
            // Fix 2: пропускаем свежего врага, возможно в иммунитете.
            continue;
        }
        // Проверка достижимости: хотя бы один useful_author в AR от цели.
        let reachable = state
            .useful_authors(params)
            .any(|p| can_reach_via_any_relay(p, e.pos, state, params));
        if !reachable {
            continue;
        }
        // Утилити — очки, которые можно получить при разрушении (база клетки).
        // Если HP цели <= SE, валим за один ход — приоритет максимум.
        let finishing = e.hp <= params.se;
        tasks.push(Task {
            kind: TaskKind::Sabotage {
                target_id: e.id.clone(),
            },
            target: e.pos,
            utility: cell_base_points(e.pos) as f64,
            urgency: if finishing { 0.8 } else { 0.4 },
            required_effort: ((e.hp as f64) / (params.se as f64)).ceil().max(1.0),
        });
    }
    tasks
}

fn can_reach_via_any_relay(
    author: &Plantation,
    target: Pos,
    state: &GameState,
    params: &DerivedParams,
) -> bool {
    if chebyshev(author.pos, target) <= params.ar {
        return true;
    }
    state.useful_authors(params).any(|r| {
        r.id != author.id
            && chebyshev(author.pos, r.pos) <= params.sr
            && chebyshev(r.pos, target) <= params.ar
    })
}
