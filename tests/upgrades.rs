use datssol_bot::config::Config;
use datssol_bot::model::state::{GameState, UpgradeTier, UpgradesState};
use datssol_bot::tactics::choose_upgrade;

fn state_with_upgrades(points: i32, tiers: Vec<(&str, i32, i32)>) -> GameState {
    GameState {
        turn_no: 30,
        next_turn_in: 1.0,
        map_size: (100, 100),
        action_range: 2,
        plantations: vec![],
        enemies: vec![],
        mountains: vec![],
        cells: vec![],
        construction: vec![],
        beavers: vec![],
        upgrades: UpgradesState {
            points,
            interval_turns: 30,
            turns_until_points: 0,
            max_points: 15,
            tiers: tiers
                .into_iter()
                .map(|(n, cur, max)| UpgradeTier {
                    name: n.into(),
                    current: cur,
                    max,
                })
                .collect(),
        },
        meteo: vec![],
    }
}

#[test]
fn no_points_none() {
    let s = state_with_upgrades(0, vec![("repair_power", 0, 3)]);
    assert!(choose_upgrade(&s, &Config::default_embedded()).is_none());
}

#[test]
fn first_in_priority() {
    // Первый в priority_order — max_hp. У нас есть points и current < max.
    let s = state_with_upgrades(
        1,
        vec![
            ("max_hp", 0, 5),
            ("settlement_limit", 0, 10),
            ("signal_range", 0, 10),
        ],
    );
    assert_eq!(
        choose_upgrade(&s, &Config::default_embedded()).as_deref(),
        Some("max_hp")
    );
}

#[test]
fn skip_maxed_and_take_next() {
    // max_hp на max → пойти к следующему в sequence (settlement_limit).
    let s = state_with_upgrades(
        1,
        vec![
            ("max_hp", 5, 5),
            ("settlement_limit", 0, 10),
        ],
    );
    assert_eq!(
        choose_upgrade(&s, &Config::default_embedded()).as_deref(),
        Some("settlement_limit")
    );
}

#[test]
fn fallback_to_any_available_after_sequence_exhausted() {
    // Все из sequence на max → берём любое доступное, которое есть в tiers.
    let s = state_with_upgrades(
        1,
        vec![
            ("repair_power", 3, 3),
            ("settlement_limit", 10, 10),
            ("vision_range", 5, 5),
            ("signal_range", 10, 10),
            ("decay_mitigation", 0, 3), // не в нашем sequence, но доступно
        ],
    );
    assert_eq!(
        choose_upgrade(&s, &Config::default_embedded()).as_deref(),
        Some("decay_mitigation")
    );
}

#[test]
fn all_maxed_none() {
    let s = state_with_upgrades(
        5,
        vec![("repair_power", 3, 3), ("settlement_limit", 10, 10)],
    );
    assert!(choose_upgrade(&s, &Config::default_embedded()).is_none());
}
