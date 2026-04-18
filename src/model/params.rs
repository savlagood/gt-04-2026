#![allow(dead_code)]

use crate::model::state::GameState;

/// Производные параметры — вычисляются каждый ход из `upgrades.tiers`.
///
/// По task.md §Апгрейд плантаций существуют только эти апгрейды,
/// ни один из них не увеличивает CS (Fix 3).
#[derive(Debug, Clone, Copy)]
pub struct DerivedParams {
    pub ts: i32, // terraforming speed (5, не апгрейдится)
    pub cs: i32, // construction speed — Fix 3: константа 5, VERIFY
    pub rs: i32, // repair speed
    pub se: i32, // sabotage efficiency (5, не апгрейдится)
    pub be: i32, // beaver elimination (5, не апгрейдится)
    pub ds: i32, // degradation speed
    pub mhp: i32,
    pub limit: i32,
    pub sr: i32, // signal range
    pub vr: i32, // vision range
    pub ar: i32, // action range (из API)
    pub earthquake_dmg: i32,
    pub beaver_dmg: i32,
    pub storm_dmg: i32,
}

impl DerivedParams {
    pub fn from_state(state: &GameState) -> Self {
        let get = |name: &str| -> i32 {
            state
                .upgrades
                .tiers
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.current)
                .unwrap_or(0)
        };
        let repair = get("repair_power");
        Self {
            ts: 5,
            cs: 5, // Fix 3: VERIFY repair_power не влияет на CS
            rs: 5 + repair,
            se: 5,
            be: 5,
            ds: (10 - 2 * get("decay_mitigation")).max(1),
            mhp: 50 + 10 * get("max_hp"),
            limit: 30 + get("settlement_limit"),
            sr: 3 + get("signal_range"),
            vr: 3 + 2 * get("vision_range"),
            ar: state.action_range.max(1),
            earthquake_dmg: (10 - 2 * get("earthquake_mitigation")).max(0),
            beaver_dmg: (15 - 2 * get("beaver_damage_mitigation")).max(0),
            storm_dmg: 2,
        }
    }
}
