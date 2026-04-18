#![allow(dead_code)]

use std::collections::HashSet;

use crate::config::Config;
use crate::geom::{chebyshev, in_bounds, is_boosted, manhattan};
use crate::graph::ChainGraph;
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Pos};
use crate::planner::tasks::{Phase, Task, TaskKind};
use crate::predict::{
    analyze_limit, predict_construction_damage, predict_storm, safe_to_start_new_build,
    storm_threatens,
};

/// Fix 5: кандидаты на стройку — пустые клетки в AR от `useful_authors`,
/// 4-adjacent к плантации, связанной с ЦУ (`connected_to_main`).
/// Иначе новая плантация сразу окажется изолированной.
pub fn find_buildable_cells(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
) -> Vec<Pos> {
    let graph = ChainGraph::build(state);
    let connected_idx = graph.connected_to_main();
    let connected_positions: HashSet<Pos> = connected_idx
        .iter()
        .map(|&i| graph.pos_of(i))
        .collect();

    let mut out: HashSet<Pos> = HashSet::new();
    let occupied_by_us: HashSet<Pos> = state.plantations.iter().map(|p| p.pos).collect();
    let enemy_plants: HashSet<Pos> = state.enemies.iter().map(|e| e.pos).collect();
    let construction_positions: HashSet<Pos> =
        state.construction.iter().map(|c| c.pos).collect();
    let beaver_positions: HashSet<Pos> = state.beavers.iter().map(|b| b.pos).collect();
    let mountains: HashSet<Pos> = memory
        .known_mountains
        .iter()
        .copied()
        .chain(state.mountains.iter().copied())
        .collect();
    // Клетки, уже терраформированные на 100%: сервер их считает «готовыми»
    // и отклоняет `build` на них (`invalid target` в /api/logs). Прогресс
    // спадает только через 30 ходов после завершения. До этого — исключаем.
    let fully_terraformed: HashSet<Pos> = state
        .cells
        .iter()
        .filter(|c| c.terraformation_progress >= 100)
        .map(|c| c.pos)
        .collect();

    for author in state.useful_authors(params) {
        if !connected_positions.contains(&author.pos) {
            continue;
        }
        for dx in -params.ar..=params.ar {
            for dy in -params.ar..=params.ar {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let cell = Pos::new(author.pos.x + dx, author.pos.y + dy);
                if !in_bounds(cell, state.map_size) {
                    continue;
                }
                if mountains.contains(&cell) {
                    continue;
                }
                if occupied_by_us.contains(&cell) {
                    continue;
                }
                if enemy_plants.contains(&cell) {
                    continue;
                }
                if beaver_positions.contains(&cell) {
                    continue;
                }
                if construction_positions.contains(&cell) {
                    // Свои стройки обрабатываются отдельной тактикой (добивание).
                    continue;
                }
                if fully_terraformed.contains(&cell) {
                    continue;
                }
                // 4-adj К СВЯЗАННОЙ плантации (Fix 5).
                let adj_to_connected = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(ddx, ddy)| {
                        let np = Pos::new(cell.x + ddx, cell.y + ddy);
                        connected_positions.contains(&np)
                    });
                if !adj_to_connected {
                    continue;
                }
                out.insert(cell);
            }
        }
    }
    out.into_iter().collect()
}

/// Scoring клетки по секции 8.4 init_plan.md с учётом фаз:
///   - **base** = 1000 (обычная) или 1500 (boosted `is_boosted`).
///   - **phase_mul**: boosted-клетки усиливаются в зависимости от фазы
///     (Endgame — 2.0×, Harvest — 1.5×, ранние — 1.2×).
///   - **risk**: бобёр в ≤ 2 клетках, буря по пути, враг в AR.
///   - **bonus**: близость к невзятой boosted-клетке (стратегическая кластеризация).
///   - **penalty**: chebyshev-расстояние от ЦУ (компактность сети).
pub fn score_cell_value(
    cell: Pos,
    state: &GameState,
    params: &DerivedParams,
    cfg: &Config,
    phase: Phase,
) -> f64 {
    let base = if is_boosted(cell) {
        cfg.scoring.cell.base_boosted
    } else {
        cfg.scoring.cell.base_normal
    };
    let phase_mul = match phase {
        Phase::Early => cfg.scoring.cell.boost_factor_early,
        Phase::Growth => cfg.scoring.cell.boost_factor_growth,
        Phase::Harvest => cfg.scoring.cell.boost_factor_harvest,
        Phase::Endgame => cfg.scoring.cell.boost_factor_endgame,
    };
    let boosted_bonus = if is_boosted(cell) { phase_mul } else { 1.0 };

    let mut risk = 0.0;
    for b in &state.beavers {
        if chebyshev(cell, b.pos) <= 2 {
            risk += cfg.scoring.risk.beaver_in_range;
        }
    }
    let storm_preds: Vec<_> = state
        .meteo
        .iter()
        .filter_map(|m| predict_storm(m, 5))
        .collect();
    if storm_threatens(cell, &storm_preds) {
        risk += cfg.scoring.risk.storm_in_path;
    }
    for e in &state.enemies {
        if chebyshev(cell, e.pos) <= params.ar {
            risk += cfg.scoring.risk.enemy_nearby;
        }
    }

    // Бонус: близость к ещё-не-занятой boosted-клетке. Радиус поиска 4
    // клетки, делим на manhattan — дальние клетки дают меньше.
    let mut bonus = 0.0;
    let our_positions: HashSet<Pos> = state.plantations.iter().map(|p| p.pos).collect();
    let our_constructions: HashSet<Pos> = state.construction.iter().map(|c| c.pos).collect();
    for dx in -4..=4 {
        for dy in -4..=4 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let bp = Pos::new(cell.x + dx, cell.y + dy);
            if !is_boosted(bp) {
                continue;
            }
            if our_positions.contains(&bp) {
                continue;
            }
            let d = manhattan(cell, bp).max(1);
            bonus += cfg.scoring.bonus.nearby_boost_weight / (d as f64);
        }
    }

    // Compactness bonus считает **только стабильных** соседей: стройки и
    // плантации, не изолированные от сети (is_isolated=false — авторитетный
    // серверный флаг). Terraform progress не является признаком гибели
    // плантации: она умирает только от изоляции, а не от завершения terraform.
    let is_stable_neighbor = |p: Pos| -> bool {
        if our_constructions.contains(&p) {
            return true;
        }
        state.plantation_at(p).map(|pl| !pl.is_isolated).unwrap_or(false)
    };
    let stable_neighbors: i32 = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .filter(|(ddx, ddy)| is_stable_neighbor(Pos::new(cell.x + ddx, cell.y + ddy)))
        .count() as i32;
    // Все 4-adj соседи (включая обречённых) — для подсчёта «хрупких» клеток.
    let any_neighbors: i32 = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .filter(|(ddx, ddy)| {
            let np = Pos::new(cell.x + ddx, cell.y + ddy);
            our_positions.contains(&np) || our_constructions.contains(&np)
        })
        .count() as i32;
    // Бонус: 2 стабильных — +1000, 3 — +3000, 4 — +6000.
    // Квадратичная, чтобы ЖЁСТКО предпочитать «запечатывание» плотных узлов.
    let compact_bonus = if stable_neighbors >= 2 {
        let extra = stable_neighbors - 1;
        (extra * extra) as f64 * 1000.0
    } else if stable_neighbors == 1 && any_neighbors >= 2 {
        // Маленький плюс: есть 1 стабильный + 1 обречённый → клетка
        // переживёт смерть обречённого (стабильный останется).
        300.0
    } else {
        0.0
    };
    // Жёсткий штраф за клетку «висящую на обречённом» — единственный сосед
    // в сети на клетке с terraform ≥ 80% или is_main. После его смерти new
    // plantation окажется isolated и умрёт по DS.
    let fragile_penalty = if any_neighbors == 1 && stable_neighbors == 0 {
        -2_000.0
    } else {
        0.0
    };

    let main_pos = state.main().map(|p| p.pos).unwrap_or(cell);
    let compactness_penalty =
        chebyshev(cell, main_pos) as f64 * cfg.scoring.penalty.distance_from_main;

    let mut main_adj_bonus = 0.0;
    if let Some(main) = state.main() {
        if manhattan(cell, main.pos) == 1 {
            let mut escape_routes_count = 0;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let adj_pos = Pos::new(main.pos.x + dx, main.pos.y + dy);
                if adj_pos == cell {
                    continue;
                }
                if state.plantation_at(adj_pos).is_some() || our_constructions.contains(&adj_pos) {
                    escape_routes_count += 1;
                }
            }

            if escape_routes_count == 0 {
                main_adj_bonus = 5000.0; // Priority to secure 1st 4-adj to main for safe relocation
            } else {
                main_adj_bonus = -5000.0; // Penalty to avoid trapping the main base with more than 1 surrounding base
            }
        }
    }

    base * boosted_bonus - risk + bonus + compact_bonus + fragile_penalty + main_adj_bonus - compactness_penalty
}

/// Полный generator build-задач (Step 11):
///   1. Добиваем свои стройки. Если на стройку идёт урон (бобёр/землетрясение)
///      больше, чем прогресс за ход — скипаем (Fix 11).
///   2. Если лимит позволяет (`safe_to_start_new_build`) — стартуем новые
///      стройки по числу свободных `useful_authors`, с предпочтением клеток
///      по `score_cell_value`. Сортировка по score desc, детерминированно.
pub fn generate_build_tasks(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
    phase: Phase,
) -> Vec<Task> {
    let mut tasks = Vec::new();

    // 1. Собираем свои активные стройки (не под ударом).
    let mut our_builds: Vec<&crate::model::state::Construction> = Vec::new();
    for c in &state.construction {
        if !memory.is_our_construction(c.pos) {
            continue;
        }
        let exp_dmg = predict_construction_damage(c, state, params);
        // Бросаем только если стройка умрёт в этот ход даже с нашей помощью.
        // Строить при exp_dmg > cs всё равно выгоднее (net хуже на 5, но не 0),
        // и build-команда предотвращает деградацию по DS (task.md: деградация
        // только если «не было прогресса стройки», т.е. не было команды).
        if c.progress + params.cs <= exp_dmg {
            continue;
        }
        our_builds.push(c);
    }

    let useful_count = state.useful_authors(params).count();

    // 2. Combined build strategy:
    // Сначала обязательно генерируем задачи для ВСЕХ наших активных строек, чтобы они не деградировали.
    for c in &our_builds {
        let remaining = (50 - c.progress).max(1);
        let cell_score = score_cell_value(c.pos, state, params, cfg, phase);
        // Если пропустить этот ход — сервер применяет DS немедленно (шаг 8).
        // При progress ≤ DS стройка умрёт в тот же ход. Поднимаем приоритет
        // до уровня критического ремонта, чтобы конкурировать на равных.
        let at_risk = c.progress <= params.ds;
        let (urgency, extra) = if at_risk {
            (cfg.urgency.critical_repair, 100_000.0)
        } else {
            (cfg.urgency.unfinished_construction, 0.0)
        };
        
        // Генерируем до 3 задач на одну стройку, чтобы позволить фокус-огонь (совместную постройку)
        for _ in 0..3 {
            tasks.push(Task {
                kind: TaskKind::Build,
                target: c.pos,
                utility: cell_score + extra + 2_000.0 + remaining as f64 * 10.0,
                urgency,
                required_effort: (remaining as f64 / params.cs as f64).ceil().max(1.0),
            });
        }
    }

    // 3. Лимит: если мы уже на границе и старейшая плантация — ЦУ,
    //    запускать новую стройку нельзя (task.md §Основание: потеряем ЦУ).
    let la = analyze_limit(state, memory, params);
    if !safe_to_start_new_build(&la) {
        return tasks;
    }

    // 4. Новые стройки — утилизируем 100% авторов (снимаем лимиты)
    let slots = useful_count;
    if slots == 0 {
        return tasks;
    }
    let mut candidates: Vec<(Pos, f64)> = find_buildable_cells(state, memory, params)
        .into_iter()
        .map(|cell| (cell, score_cell_value(cell, state, params, cfg, phase)))
        .filter(|(_, s)| *s > 0.0)
        .collect();
    candidates.sort_by(|(pa, sa), (pb, sb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (pa.x, pa.y).cmp(&(pb.x, pb.y)))
    });

    let mut current_projected = la.projected;

    for (cell, score) in candidates.into_iter().take(slots) {
        if current_projected >= la.limit && la.oldest_is_main {
            break;
        }

        let urgency = if is_boosted(cell) {
            cfg.urgency.new_build_boost
        } else {
            cfg.urgency.new_build_normal
        };
        tasks.push(Task {
            kind: TaskKind::Build,
            target: cell,
            utility: score,
            urgency,
            required_effort: (50.0 / params.cs as f64).ceil(),
        });
        current_projected += 1;
    }

    tasks
}

/// Алиас для обратной совместимости с текущим `plan_turn`.
/// После Step 11 старая mvp-функция уходит — `plan_turn` вызывает полную.
pub fn generate_build_tasks_mvp(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
    phase: Phase,
) -> Vec<Task> {
    generate_build_tasks(state, memory, params, cfg, phase)
}
