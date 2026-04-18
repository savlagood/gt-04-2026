#![allow(dead_code)]

use crate::config::Config;
use crate::geom::cell_base_points;
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::{Beaver, GameState, Plantation, Pos};
use crate::planner::tasks::{Phase, Task, TaskKind};

use crate::tactics::can_reach_via_any_relay;

pub struct BeaverKillPlan {
    pub attacker_positions: Vec<Pos>,
    pub turns_to_kill: u32,
    pub expected_reward: f64,
    pub expected_damage_taken: f64,
}

/// Планировщик убийства одного логова бобра.
/// Задействуем всех `useful_authors` в AR=2 от логова.
///
/// Fix 1: reward = 20 × `cell_base_points(pos)`.
/// Fix 12: убиваем только если слабейший атакующий выдержит все ходы
///         атак (`min_hp / beaver_dmg >= turns_to_kill`).
pub fn plan_beaver_kill(
    b: &Beaver,
    state: &GameState,
    params: &DerivedParams,
    cfg: &Config,
) -> Option<BeaverKillPlan> {
    let in_range: Vec<&Plantation> = state
        .useful_authors(params)
        .filter(|p| can_reach_via_any_relay(p, b.pos, state, params))
        .collect();
    if (in_range.len() as i32) < cfg.beaver.min_attackers {
        return None;
    }

    // BE=5 на каждого, но логово регенерирует 5 HP/ход.
    // net_dps = N * 5 - 5.
    let net_dps = (in_range.len() as i32) * params.be - 5;
    if net_dps <= 0 {
        return None;
    }
    let turns_to_kill = ((b.hp as f64) / (net_dps as f64)).ceil() as u32;

    // Fix 12: выживаемость. Суммарный HP пула делим на суммарный урон в ход
    // (бобёр бьёт всех равномерно). Проверка по min дала бы false-negative при
    // 3 атакующих с 30 HP каждый — они суммарно держат удар нормально.
    let bd = params.beaver_dmg.max(1);
    let total_hp: i32 = in_range.iter().map(|p| p.hp).sum();
    let total_dmg_per_turn = (in_range.len() as i32) * bd;
    let turns_survive = (total_hp / total_dmg_per_turn.max(1)) as u32;
    if turns_survive < turns_to_kill {
        return None;
    }

    // Fix 1: награда = 20 × base_points клетки (boosted 1500 или normal 1000).
    let reward = 20.0 * cell_base_points(b.pos) as f64;

    // Opportunity cost: каждый атакующий мог бы тратить ход на очки
    // от терраформирования / стройку. Если награда не окупает — скип.
    let opportunity = (in_range.len() as f64)
        * cfg.beaver.opportunity_cost_per_turn
        * (turns_to_kill as f64);
    if reward < opportunity {
        return None;
    }

    let expected_damage_taken =
        (in_range.len() as f64) * (params.beaver_dmg as f64) * (turns_to_kill as f64);
    Some(BeaverKillPlan {
        attacker_positions: in_range.iter().map(|p| p.pos).collect(),
        turns_to_kill,
        expected_reward: reward,
        expected_damage_taken,
    })
}

/// Генерит по одной задаче `BeaverAttack` на каждого доступного атакующего.
/// В фазе Early не трогаем бобров (слишком рано, слабая сеть).
pub fn generate_beaver_tasks(
    state: &GameState,
    _memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
    phase: Phase,
) -> Vec<Task> {
    if matches!(phase, Phase::Early) {
        return Vec::new();
    }
    let mut tasks = Vec::new();
    for b in &state.beavers {
        let plan = match plan_beaver_kill(b, state, params, cfg) {
            Some(p) => p,
            None => continue,
        };
        for _ in &plan.attacker_positions {
            tasks.push(Task {
                kind: TaskKind::BeaverAttack {
                    target_id: b.id.clone(),
                },
                target: b.pos,
                utility: plan.expected_reward / (plan.turns_to_kill.max(1) as f64),
                urgency: cfg.urgency.beaver_hunt,
                required_effort: plan.turns_to_kill as f64,
            });
        }
    }
    tasks
}
