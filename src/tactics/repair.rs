#![allow(dead_code)]

use crate::config::Config;
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::GameState;
use crate::planner::tasks::{Task, TaskKind};
use crate::predict::predict_hp_next_turn;

/// Генерация задач ремонта на текущий ход.
///
/// Два приоритета:
///   * **критический** — предсказанный HP ≤ MHP/4 → urgency=critical_repair,
///     utility огромный (чтобы обойти все build).
///   * **плановый** — текущий HP < 70% MHP → urgency=maintenance_repair.
///
/// Плантация не может ремонтировать сама себя (task.md §Ремонт плантации).
/// Фильтр автора выполняет `assign_tasks_mvp` (мы только создаём задачу,
/// в `TaskKind::Repair { target_id }` указываем цель).
pub fn generate_repair_tasks(
    state: &GameState,
    _memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
) -> Vec<Task> {
    let mut tasks = Vec::new();
    for p in state.controllable() {
        let predicted_hp = predict_hp_next_turn(p, state, params);

        // Критический: после следующей фазы HP ≤ 25% MHP (или уже отрицательный).
        if predicted_hp <= params.mhp / 4 {
            let missing = (params.mhp - predicted_hp).max(1);
            tasks.push(Task {
                kind: TaskKind::Repair {
                    target_id: p.id.clone(),
                },
                target: p.pos,
                // Очень большой utility, чтобы обойти любой build.
                utility: 100_000.0 + missing as f64 * 100.0,
                urgency: cfg.urgency.critical_repair,
                required_effort: (missing as f64 / params.rs as f64).ceil().max(1.0),
            });
            continue;
        }

        // Плановый: HP < 70% MHP и нет проекта смерти.
        if (p.hp as f64) < (params.mhp as f64) * 0.7 {
            let missing = (params.mhp - p.hp).max(1);
            tasks.push(Task {
                kind: TaskKind::Repair {
                    target_id: p.id.clone(),
                },
                target: p.pos,
                utility: missing as f64 * 20.0,
                urgency: cfg.urgency.maintenance_repair,
                required_effort: (missing as f64 / params.rs as f64).ceil().max(1.0),
            });
        }
    }
    tasks
}
