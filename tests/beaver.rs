use datssol_bot::config::Config;
use datssol_bot::model::state::{Beaver, GameState, Plantation, Pos, UpgradesState};
use datssol_bot::model::DerivedParams;
use datssol_bot::tactics::plan_beaver_kill;

fn plant(id: &str, x: i32, y: i32, hp: i32) -> Plantation {
    Plantation {
        id: id.into(),
        pos: Pos::new(x, y),
        hp,
        is_main: id == "m",
        is_isolated: false,
        immunity_until_turn: 0,
    }
}

fn state_with(plants: Vec<Plantation>, beaver: Beaver) -> GameState {
    GameState {
        turn_no: 200,
        next_turn_in: 1.0,
        map_size: (100, 100),
        action_range: 2,
        plantations: plants,
        enemies: vec![],
        mountains: vec![],
        cells: vec![],
        construction: vec![],
        beavers: vec![beaver],
        upgrades: UpgradesState::default(),
        meteo: vec![],
    }
}

#[test]
fn reward_is_20x_base_points() {
    // Fix 1: reward = 20 × cell_base_points. На boosted (7,7) = 30000.
    let beaver = Beaver {
        id: "b".into(),
        pos: Pos::new(7, 7),
        hp: 100,
    };
    // 3 атакующих на HP=50 каждый — минимум для конфига, выживают.
    // HP=1000 — чтобы Fix 12 (survive check) не отсёк (слабейший ×15 ≥ turns_to_kill).
    let plants = vec![
        plant("m", 7, 5, 1000),
        plant("p1", 6, 6, 1000),
        plant("p2", 8, 6, 1000),
    ];
    let state = state_with(plants, beaver.clone());
    let cfg = Config::default_embedded();
    let params = DerivedParams::from_state(&state);
    let plan = plan_beaver_kill(&beaver, &state, &params, &cfg).expect("should plan");
    assert_eq!(plan.expected_reward, 20.0 * 1500.0);
}

#[test]
fn reward_non_boosted_base_1000() {
    let beaver = Beaver {
        id: "b".into(),
        pos: Pos::new(10, 10),
        hp: 100,
    };
    let plants = vec![
        plant("m", 10, 8, 1000),
        plant("p1", 9, 9, 1000),
        plant("p2", 11, 9, 1000),
    ];
    let state = state_with(plants, beaver.clone());
    let cfg = Config::default_embedded();
    let params = DerivedParams::from_state(&state);
    let plan = plan_beaver_kill(&beaver, &state, &params, &cfg).expect("should plan");
    assert_eq!(plan.expected_reward, 20.0 * 1000.0);
}

#[test]
fn survivability_blocks_weak_attackers() {
    // Fix 12: HP=10, beaver_dmg=15 → слабейший проживёт 0 ходов, а убить
    // логово нужно хотя бы 1 ход → отказ.
    let beaver = Beaver {
        id: "b".into(),
        pos: Pos::new(10, 10),
        hp: 100,
    };
    let plants = vec![
        plant("m", 10, 8, 10), // только 10 HP, погибнет в первом же раунде атак бобра
        plant("p1", 9, 9, 50),
        plant("p2", 11, 9, 50),
    ];
    let state = state_with(plants, beaver.clone());
    let cfg = Config::default_embedded();
    let params = DerivedParams::from_state(&state);
    assert!(plan_beaver_kill(&beaver, &state, &params, &cfg).is_none());
}

#[test]
fn too_few_attackers_skip() {
    // min_attackers = 3 из default config. 2 в AR — отказ.
    let beaver = Beaver {
        id: "b".into(),
        pos: Pos::new(10, 10),
        hp: 100,
    };
    let plants = vec![plant("m", 10, 8, 50), plant("p1", 9, 9, 50)];
    let state = state_with(plants, beaver.clone());
    let cfg = Config::default_embedded();
    let params = DerivedParams::from_state(&state);
    assert!(plan_beaver_kill(&beaver, &state, &params, &cfg).is_none());
}
