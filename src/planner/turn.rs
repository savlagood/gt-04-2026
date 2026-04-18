#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::geom::{adjacent4, chebyshev, in_bounds};
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Plantation, Pos};
use crate::planner::tasks::{Assignment, Task, TaskKind, TurnPlan};
use crate::planner::tasks::Phase;
use crate::tactics::build::generate_build_tasks;
use crate::tactics::{
    choose_upgrade, generate_beaver_tasks, generate_repair_tasks, generate_sabotage_tasks,
    plan_relocate_main,
};

/// MVP версия: repair (если HP низкий) + build (если есть куда).
/// Всегда пытаемся купить upgrade, если есть очки.
/// Полная версия (sabotage, beaver, scoring) появится в Steps 11–14.
pub fn plan_turn(state: &GameState, memory: &Memory, cfg: &Config) -> TurnPlan {
    let params = DerivedParams::from_state(state);

    let phase = Phase::from_turn(state.turn_no, cfg);

    let mut tasks: Vec<Task> = Vec::new();
    tasks.extend(generate_repair_tasks(state, memory, &params, cfg));
    tasks.extend(generate_build_tasks(state, memory, &params, cfg, phase));
    tasks.extend(generate_beaver_tasks(state, memory, &params, cfg, phase));
    tasks.extend(generate_sabotage_tasks(state, memory, &params, cfg, phase));
    tasks.sort_by(|a, b| {
        b.composite_score()
            .partial_cmp(&a.composite_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut assignments = assign_tasks_mvp(&tasks, state, &params);

    // Апгрейд — покупаем каждый ход, если есть очки.
    let upgrade = choose_upgrade(state, cfg);

    // Relocate ЦУ — при угрозе HP/completion/storm и наличии 4-adj соседа.
    let relocate_main = plan_relocate_main(state, memory, &params, cfg);

    if assignments.is_empty() && upgrade.is_none() && relocate_main.is_none() {
        if let Some(fb) = fallback_action(state, memory, &params) {
            assignments.push(fb);
        }
    }

    TurnPlan {
        assignments,
        upgrade,
        relocate_main,
    }
}

/// MVP-назначение: один автор → одна задача. Автор — ближайшая useful
/// плантация в AR от цели. Реле не используем (автор = relay).
///
/// Для `TaskKind::Repair` автор не может быть целью ремонта (task.md:
/// «Плантация не может ремонтировать саму себя»).
fn assign_tasks_mvp(
    tasks: &[Task],
    state: &GameState,
    params: &DerivedParams,
) -> Vec<Assignment> {
    let mut used_authors: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for t in tasks {
        let best = state
            .useful_authors(params)
            .filter(|p| !used_authors.contains(&p.id))
            .filter(|p| chebyshev(p.pos, t.target) <= params.ar)
            .filter(|p| match &t.kind {
                TaskKind::Repair { target_id } => &p.id != target_id,
                _ => true,
            })
            .min_by_key(|p| chebyshev(p.pos, t.target));
        if let Some(p) = best {
            used_authors.insert(p.id.clone());
            out.push(Assignment {
                author_id: p.id.clone(),
                author_pos: p.pos,
                relay_pos: p.pos,
                target_pos: t.target,
                kind: t.kind.clone(),
                expected_effect: t.kind.base_stat(params),
            });
        }
    }
    out
}

/// Fix 8: fallback при пустом плане.
///
/// 1. попробовать ремонт соседа (если есть ≥ 2 плантации в AR);
/// 2. иначе — стройка первой 4-adj пустой клетки;
/// 3. иначе None (одна плантация на границе без опций).
pub fn fallback_action(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
) -> Option<Assignment> {
    // Шаг 1: ремонт
    let controllable: Vec<&Plantation> = state.controllable().collect();
    for p in &controllable {
        for other in &controllable {
            if other.id == p.id {
                continue;
            }
            if chebyshev(p.pos, other.pos) <= params.ar {
                return Some(Assignment {
                    author_id: p.id.clone(),
                    author_pos: p.pos,
                    relay_pos: p.pos,
                    target_pos: other.pos,
                    kind: TaskKind::Repair {
                        target_id: other.id.clone(),
                    },
                    expected_effect: params.rs,
                });
            }
        }
    }

    // Шаг 2: стройка любой 4-adj пустой клетки
    let occupied: HashMap<Pos, ()> = state.plantations.iter().map(|p| (p.pos, ())).collect();
    let mountains: HashSet<Pos> = state
        .mountains
        .iter()
        .copied()
        .chain(memory.known_mountains.iter().copied())
        .collect();
    let construction: HashSet<Pos> = state.construction.iter().map(|c| c.pos).collect();
    let beavers: HashSet<Pos> = state.beavers.iter().map(|b| b.pos).collect();
    let enemies: HashSet<Pos> = state.enemies.iter().map(|e| e.pos).collect();

    for p in &controllable {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let cell = Pos::new(p.pos.x + dx, p.pos.y + dy);
            if !in_bounds(cell, state.map_size) {
                continue;
            }
            if occupied.contains_key(&cell)
                || mountains.contains(&cell)
                || construction.contains(&cell)
                || beavers.contains(&cell)
                || enemies.contains(&cell)
            {
                continue;
            }
            debug_assert!(adjacent4(cell, p.pos));
            return Some(Assignment {
                author_id: p.id.clone(),
                author_pos: p.pos,
                relay_pos: p.pos,
                target_pos: cell,
                kind: TaskKind::Build,
                expected_effect: params.cs,
            });
        }
    }
    None
}
