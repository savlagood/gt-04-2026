use datssol_bot::config::Config;
use datssol_bot::model::memory::Memory;
use datssol_bot::model::state::{GameState, Plantation, Pos, UpgradesState};
use datssol_bot::planner::{plan_turn, tasks::TaskKind};

fn base_state(plants: Vec<Plantation>) -> GameState {
    GameState {
        turn_no: 1,
        next_turn_in: 1.0,
        map_size: (100, 100),
        action_range: 2,
        plantations: plants,
        enemies: vec![],
        mountains: vec![],
        cells: vec![],
        construction: vec![],
        beavers: vec![],
        upgrades: UpgradesState::default(),
        meteo: vec![],
    }
}

fn main_plant(x: i32, y: i32) -> Plantation {
    Plantation {
        id: "m".into(),
        pos: Pos::new(x, y),
        hp: 50,
        is_main: true,
        is_isolated: false,
        immunity_until_turn: 0,
    }
}

#[test]
fn single_main_produces_non_empty_plan() {
    // Fix 8: даже при одной ЦУ должен быть build (нет empty command).
    let state = base_state(vec![main_plant(50, 50)]);
    let mut memory = Memory::default();
    memory.update(&state);
    let cfg = Config::default_embedded();
    let plan = plan_turn(&state, &memory, &cfg);
    assert!(!plan.is_empty(), "single main must not produce empty command");
    // Либо build из generate_build_tasks_mvp, либо fallback build.
    assert!(plan
        .assignments
        .iter()
        .any(|a| matches!(a.kind, TaskKind::Build)));
}

#[test]
fn plan_action_paths_have_three_coords() {
    let state = base_state(vec![main_plant(50, 50)]);
    let mut memory = Memory::default();
    memory.update(&state);
    let cfg = Config::default_embedded();
    let plan = plan_turn(&state, &memory, &cfg);
    let dto = plan.into_player_dto();
    for action in &dto.command {
        assert_eq!(
            action.path.len(),
            3,
            "every path must be [author, relay, target]"
        );
    }
}

#[test]
fn build_candidates_never_isolated() {
    // Изолированная плантация не должна порождать build-кандидатов.
    use datssol_bot::model::DerivedParams;
    use datssol_bot::tactics::find_buildable_cells;
    let mut state = base_state(vec![
        main_plant(50, 50),
        Plantation {
            id: "iso".into(),
            pos: Pos::new(70, 70),
            hp: 50,
            is_main: false,
            is_isolated: true,
            immunity_until_turn: 0,
        },
    ]);
    // Симулируем, что is_isolated=true (сервер так нам скажет).
    state.plantations[1].is_isolated = true;

    let memory = Memory::default();
    let params = DerivedParams::from_state(&state);
    let cells = find_buildable_cells(&state, &memory, &params);
    for c in &cells {
        // Никакой клетки в радиусе AR от isolated (70,70) быть не должно —
        // изолированная не считается ни как author, ни как connected для 4-adj.
        let close_to_iso = (c.x - 70).abs() <= 2 && (c.y - 70).abs() <= 2;
        assert!(
            !close_to_iso,
            "isolated plantation generated candidate at {c:?}"
        );
    }
    // Рядом с main (50,50) — ожидаем 4 кандидата (4-adj).
    let near_main: Vec<_> = cells
        .iter()
        .filter(|p| (p.x - 50).abs() + (p.y - 50).abs() == 1)
        .collect();
    assert_eq!(near_main.len(), 4);
}
