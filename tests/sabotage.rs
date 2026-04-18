use datssol_bot::config::Config;
use datssol_bot::model::memory::Memory;
use datssol_bot::model::state::{EnemyPlantation, GameState, Plantation, Pos, UpgradesState};
use datssol_bot::model::DerivedParams;
use datssol_bot::planner::tasks::Phase;
use datssol_bot::tactics::generate_sabotage_tasks;

fn plant_main(x: i32, y: i32) -> Plantation {
    Plantation {
        id: "m".into(),
        pos: Pos::new(x, y),
        hp: 50,
        is_main: true,
        is_isolated: false,
        immunity_until_turn: 0,
    }
}

fn state_with_enemy(turn: u32, enemy: EnemyPlantation) -> GameState {
    GameState {
        turn_no: turn,
        next_turn_in: 1.0,
        map_size: (100, 100),
        action_range: 2,
        plantations: vec![plant_main(50, 50)],
        enemies: vec![enemy],
        mountains: vec![],
        cells: vec![],
        construction: vec![],
        beavers: vec![],
        upgrades: UpgradesState::default(),
        meteo: vec![],
    }
}

#[test]
fn early_phase_no_sabotage() {
    let e = EnemyPlantation {
        id: "e".into(),
        pos: Pos::new(51, 50),
        hp: 30,
    };
    let state = state_with_enemy(10, e);
    let mut memory = Memory::default();
    memory.update(&state); // mark enemy as seen
    let params = DerivedParams::from_state(&state);
    let cfg = Config::default_embedded();
    assert!(generate_sabotage_tasks(&state, &memory, &params, &cfg, Phase::Early).is_empty());
}

#[test]
fn harvest_phase_generates_task() {
    // Враг с HP < MHP (успели ранить) → второй заход → эвристика не режет.
    let e = EnemyPlantation {
        id: "e".into(),
        pos: Pos::new(51, 50),
        hp: 30,
    };
    let state = state_with_enemy(300, e);
    let mut memory = Memory::default();
    memory.update(&state);
    memory.update(&state); // second observation — enemy is no longer "fresh"
    let params = DerivedParams::from_state(&state);
    let cfg = Config::default_embedded();
    let tasks = generate_sabotage_tasks(&state, &memory, &params, &cfg, Phase::Harvest);
    assert_eq!(tasks.len(), 1, "should generate a sabotage task in Harvest");
}

#[test]
fn fresh_enemy_with_full_hp_skipped() {
    // Fix 2: свежий враг с HP=50 → считаем в иммунитете, не атакуем.
    let e = EnemyPlantation {
        id: "e".into(),
        pos: Pos::new(51, 50),
        hp: 50,
    };
    let state = state_with_enemy(300, e);
    let memory = Memory::default(); // никогда его не видели
    let params = DerivedParams::from_state(&state);
    let cfg = Config::default_embedded();
    assert!(
        generate_sabotage_tasks(&state, &memory, &params, &cfg, Phase::Harvest).is_empty(),
        "fresh enemy at full HP must be skipped as possibly immune"
    );
}

#[test]
fn out_of_range_enemy_skipped() {
    // Враг слишком далеко от наших — нет достижимости, не генерируем.
    let e = EnemyPlantation {
        id: "e".into(),
        pos: Pos::new(90, 90),
        hp: 10,
    };
    let state = state_with_enemy(300, e);
    let mut memory = Memory::default();
    memory.update(&state);
    memory.update(&state);
    let params = DerivedParams::from_state(&state);
    let cfg = Config::default_embedded();
    assert!(generate_sabotage_tasks(&state, &memory, &params, &cfg, Phase::Harvest).is_empty());
}
