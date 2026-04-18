#![allow(dead_code)]

use std::collections::HashMap;

use crate::geom::chebyshev;
use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Plantation, Pos};

#[derive(Debug, Clone)]
pub struct RelayChoice {
    pub relay_pos: Pos,
    pub expected_effect: i32,
}

/// Выбор оптимального реле для передачи команды от `author` к `target`.
/// Учитывает падение эффективности (штраф -1 за каждую команду, проходящую через реле).
pub fn choose_relay(
    author: &Plantation,
    target: Pos,
    base_stat: i32,
    state: &GameState,
    params: &DerivedParams,
    relay_usage: &HashMap<Pos, u32>,
) -> Option<RelayChoice> {
    let mut best: Option<RelayChoice> = None;

    // Вариант A: автор сам себе реле (напрямую, без передачи через других)
    if chebyshev(author.pos, target) <= params.ar {
        let used = *relay_usage.get(&author.pos).unwrap_or(&0) as i32;
        let eff = base_stat - used;
        if eff > 0 {
            best = Some(RelayChoice {
                relay_pos: author.pos,
                expected_effect: eff,
            });
        }
    }

    // Вариант B: через другое реле
    for relay in state.controllable() {
        if relay.id == author.id {
            continue;
        }
        if chebyshev(author.pos, relay.pos) > params.sr {
            continue;
        }
        if chebyshev(relay.pos, target) > params.ar {
            continue;
        }
        let used = *relay_usage.get(&relay.pos).unwrap_or(&0) as i32;
        let eff = base_stat - used;
        if eff <= 0 {
            continue;
        }
        if best
            .as_ref()
            .map(|b| eff > b.expected_effect)
            .unwrap_or(true)
        {
            best = Some(RelayChoice {
                relay_pos: relay.pos,
                expected_effect: eff,
            });
        }
    }
    best
}

pub fn can_reach_via_any_relay(
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