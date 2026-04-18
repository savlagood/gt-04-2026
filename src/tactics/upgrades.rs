#![allow(dead_code)]

use crate::config::Config;
use crate::model::state::GameState;

/// Выбор апгрейда для текущего хода.
/// Приоритет — из `config.upgrades.priority_order.sequence`. Первый апгрейд
/// из списка, у которого `current < max` и который вообще присутствует в
/// `state.upgrades.tiers`.
///
/// Возвращает `None`, если очков нет или все потенциальные апгрейды уже на max.
pub fn choose_upgrade(state: &GameState, cfg: &Config) -> Option<String> {
    if state.upgrades.points <= 0 {
        return None;
    }
    for name in &cfg.upgrades.priority_order.sequence {
        if let Some(t) = state.upgrades.tiers.iter().find(|t| &t.name == name) {
            if t.current < t.max {
                return Some(name.clone());
            }
        }
    }
    // Fallback: любой апгрейд, у которого есть запас.
    for t in &state.upgrades.tiers {
        if t.current < t.max {
            return Some(t.name.clone());
        }
    }
    None
}
