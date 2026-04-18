#![allow(dead_code)]

use crate::config::Config;
use crate::geom::{adjacent4, is_boosted};
use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Plantation, Pos};
use crate::predict::{predict_hp_next_turn, storm_threatens, turns_until_complete};

/// Решение о переносе ЦУ на соседнюю плантацию.
///
/// Триггеры:
///   * **threat_hp** — прогноз HP падает ниже `safety.main_critical_hp_fraction * MHP`;
///   * **threat_completion** — клетка под ЦУ вот-вот терраформируется до 100%
///     (task.md §Терраформация: «плантация на ней исчезает»);
///   * **threat_storm** — бури next_position в диске ЦУ.
///
/// Кандидат — 4-adj не-ЦУ плантация с HP ≥ MHP/2, не на boosted-клетке
/// (чтобы самому не терраформить клетку до конца и не потерять новый ЦУ),
/// и далёкая от завершения собственной клетки.
///
/// Возвращает `Some([from, to])` если есть кандидат, иначе `None`.
pub fn plan_relocate_main(
    state: &GameState,
    _memory: &Memory,
    params: &DerivedParams,
    cfg: &Config,
) -> Option<Vec<Pos>> {
    let main = state.main()?;
    let predicted_hp = predict_hp_next_turn(main, state, params);
    let threat_hp =
        (predicted_hp as f64) < (params.mhp as f64) * cfg.safety.main_critical_hp_fraction;

    let threat_completion = turns_until_complete(main, state, params)
        <= cfg.safety.main_critical_completion_turns;

    let storm_preds: Vec<_> = state
        .meteo
        .iter()
        .filter_map(|m| crate::predict::predict_storm(m, 2))
        .collect();
    let threat_storm = storm_threatens(main.pos, &storm_preds);

    if !(threat_hp || threat_completion || threat_storm) {
        return None;
    }

    // Минимум «запаса» для кандидата: его клетка не должна завершиться сама
    // раньше, чем текущая клетка ЦУ.

    let candidates: Vec<&Plantation> = state
        .plantations
        .iter()
        .filter(|p| p.id != main.id)
        .filter(|p| !p.is_isolated)
        .filter(|p| adjacent4(p.pos, main.pos))
        .filter(|p| p.hp >= params.mhp / 2)
        .filter(|p| !is_boosted(p.pos)) // не в ловушку (клетка быстро завершится)
        .filter(|p| turns_until_complete(p, state, params) > turns_until_complete(main, state, params))
        .filter(|p| !storm_threatens(p.pos, &storm_preds))
        .collect();

    // Выбираем самого здорового кандидата.
    let best = candidates.into_iter().max_by_key(|p| p.hp)?;
    tracing::info!(
        turn = state.turn_no,
        threat_hp,
        threat_completion,
        threat_storm,
        from = ?main.pos.to_arr(),
        to = ?best.pos.to_arr(),
        predicted_hp,
        "relocating main"
    );
    Some(vec![main.pos, best.pos])
}
