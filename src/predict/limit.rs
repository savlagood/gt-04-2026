#![allow(dead_code)]

use crate::model::memory::Memory;
use crate::model::params::DerivedParams;
use crate::model::state::GameState;

#[derive(Debug, Clone, Copy)]
pub struct LimitAnalysis {
    pub current_count: i32,
    pub our_constructions: i32,
    pub projected: i32,
    pub limit: i32,
    pub slack: i32, // limit - projected
    pub oldest_is_main: bool,
}

pub fn analyze_limit(
    state: &GameState,
    memory: &Memory,
    params: &DerivedParams,
) -> LimitAnalysis {
    let current = state.plantations.len() as i32;
    let our_cons = state
        .construction
        .iter()
        .filter(|c| memory.is_our_construction(c.pos))
        .count() as i32;
    let projected = current + our_cons;
    let oldest_is_main = memory
        .oldest_plantation(state)
        .map(|p| p.is_main)
        .unwrap_or(false);
    LimitAnalysis {
        current_count: current,
        our_constructions: our_cons,
        projected,
        limit: params.limit,
        slack: params.limit - projected,
        oldest_is_main,
    }
}

/// Можно ли начинать новую стройку без риска потерять ЦУ.
///
/// `task.md` §Основание новой плантации: при прогрессе сверх лимита исчезает
/// самая старая плантация. Если старейшая — ЦУ, мы НЕ начинаем стройку.
pub fn safe_to_start_new_build(la: &LimitAnalysis) -> bool {
    if la.projected < la.limit {
        return true;
    }
    if la.projected == la.limit {
        return !la.oldest_is_main;
    }
    false
}
