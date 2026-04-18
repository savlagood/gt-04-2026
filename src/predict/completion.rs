#![allow(dead_code)]

use crate::model::params::DerivedParams;
use crate::model::state::{GameState, Plantation};

pub fn turns_until_complete(p: &Plantation, state: &GameState, params: &DerivedParams) -> u32 {
    let cell = match state.cell_at(p.pos) {
        Some(c) => c,
        None => return u32::MAX,
    };
    let remaining = (100 - cell.terraformation_progress).max(0);
    if params.ts <= 0 {
        return u32::MAX;
    }
    (remaining as f64 / params.ts as f64).ceil() as u32
}

/// task.md §Терраформация: плантация исчезает на 100%. Если этот ход
/// добьёт клетку до ≥100 — плантацию нельзя использовать как автора новых задач
/// (Fix 9).
pub fn will_complete_this_turn(p: &Plantation, state: &GameState, params: &DerivedParams) -> bool {
    match state.cell_at(p.pos) {
        Some(c) => c.terraformation_progress + params.ts >= 100,
        // Нет записи о клетке — значит она 0% или незавершена, точно не финиширует.
        None => false,
    }
}
