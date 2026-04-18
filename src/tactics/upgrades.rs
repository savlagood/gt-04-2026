#![allow(dead_code)]

use crate::config::Config;
use crate::model::state::GameState;

/// Выбор апгрейда для текущего хода.
///
/// sequence читается как пошаговый план: i-я покупка = sequence[i].
/// Указатель в sequence = total_bought = сумма всех t.current по тирам.
///
/// Возвращает `None`, если очков нет или все апгрейды уже на max.
pub fn choose_upgrade(state: &GameState, cfg: &Config) -> Option<String> {
    if state.upgrades.points <= 0 {
        return None;
    }

    let total_bought: usize = state.upgrades.tiers.iter().map(|t| t.current as usize).sum();
    let seq = &cfg.upgrades.priority_order.sequence;

    if let Some(name) = seq.get(total_bought) {
        if let Some(t) = state.upgrades.tiers.iter().find(|t| &t.name == name) {
            if t.current < t.max {
                return Some(name.clone());
            }
        }
        tracing::warn!(
            step = total_bought,
            name = %name,
            "sequence item already at max — falling back"
        );
    }

    // Fallback: любой апгрейд с запасом (sequence исчерпан или конфиг-рассинхрон).
    state
        .upgrades
        .tiers
        .iter()
        .find(|t| t.current < t.max)
        .map(|t| t.name.clone())
}
