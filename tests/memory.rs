use datssol_bot::model::state::{GameState, Plantation, Pos, UpgradesState};
use datssol_bot::model::Memory;

fn plant(id: &str, x: i32, y: i32, is_main: bool) -> Plantation {
    Plantation {
        id: id.into(),
        pos: Pos::new(x, y),
        hp: 50,
        is_main,
        is_isolated: false,
        immunity_until_turn: 0,
    }
}

fn empty_state(turn: u32, plantations: Vec<Plantation>) -> GameState {
    GameState {
        turn_no: turn,
        next_turn_in: 1.0,
        map_size: (100, 100),
        action_range: 2,
        plantations,
        enemies: vec![],
        mountains: vec![],
        cells: vec![],
        construction: vec![],
        beavers: vec![],
        upgrades: UpgradesState::default(),
        meteo: vec![],
    }
}

#[test]
fn respawn_clears_state_specific_memory() {
    let mut mem = Memory::default();
    // Первый update с 15 плантациями
    let before_plants: Vec<_> = (0..15)
        .map(|i| plant(&format!("p{i}"), i, 0, i == 0))
        .collect();
    let before = empty_state(10, before_plants);
    mem.update(&before);
    assert_eq!(mem.birth_turn.len(), 15);
    assert_eq!(mem.mains_lost, 0);

    // Теперь внезапно только ЦУ (другой id, на другой позиции) → респавн
    let after = empty_state(60, vec![plant("new_main", 50, 50, true)]);
    mem.handle_respawn_if_detected(&after);
    mem.update(&after);

    assert_eq!(mem.mains_lost, 1);
    // birth_turn для нового main должен быть 60 (только что заполнен в update)
    assert_eq!(mem.birth_turn.len(), 1);
    assert_eq!(mem.birth_turn.get("new_main").copied(), Some(60));
    // old state-specific должен быть чист
    assert!(mem.our_construction_ever.is_empty());
    assert!(mem.last_seen_enemy.is_empty());
    assert!(mem.last_seen_beaver.is_empty());
}

#[test]
fn our_construction_ever_cleaned_when_cell_gone() {
    let mut mem = Memory::default();
    let p_main = plant("m", 0, 0, true);
    let mut state = empty_state(1, vec![p_main.clone()]);
    // Добавляем клетку-кандидата стройки в memory вручную (как это делает main loop)
    let built_pos = Pos::new(0, 1);
    mem.our_construction_ever.insert(built_pos);

    // Стройка есть — значит клетка должна остаться в memory.
    state.construction = vec![datssol_bot::model::state::Construction {
        pos: built_pos,
        progress: 10,
    }];
    mem.update(&state);
    assert!(mem.is_our_construction(built_pos));

    // Строительство пропало (например, убили), плантации там нет — должно очиститься.
    state.construction.clear();
    mem.update(&state);
    assert!(!mem.is_our_construction(built_pos));
}

#[test]
fn respawn_across_empty_round_pause() {
    // Сценарий: раунд 1 закончился, сервер шлёт plantations=[] несколько ходов,
    // затем раунд 2 стартует с новой ЦУ (другой id). Респавн должен быть пойман
    // несмотря на «пустую» паузу.
    let mut mem = Memory::default();

    // Раунд 1: id=A
    let r1 = empty_state(50, vec![plant("A", 10, 10, true)]);
    mem.handle_respawn_if_detected(&r1);
    mem.update(&r1);
    assert_eq!(mem.last_main_id.as_deref(), Some("A"));

    // Межраундная пауза: plantations=[]
    for t in 60..=65 {
        let pause = empty_state(t, vec![]);
        mem.handle_respawn_if_detected(&pause);
        mem.update(&pause);
    }
    // last_main_id должен СОХРАНИТЬСЯ как "A" (не обнулиться)
    assert_eq!(
        mem.last_main_id.as_deref(),
        Some("A"),
        "last_main_id must survive empty-plantations pause"
    );

    // Раунд 2: новая ЦУ с id=B
    let r2 = empty_state(100, vec![plant("B", 50, 50, true)]);
    mem.handle_respawn_if_detected(&r2);
    assert_eq!(mem.mains_lost, 1, "must detect respawn after inter-round pause");
}

#[test]
fn respawn_detected_when_main_id_changes_even_with_one_plant() {
    // Fix 7 сценарий B: у нас всегда 1 плантация (ЦУ), но id сменился —
    // значит старую ЦУ убили, а это место заняла новая. Должен быть респавн.
    let mut mem = Memory::default();

    // turn 1: id=A
    let s1 = empty_state(1, vec![plant("A", 10, 10, true)]);
    mem.handle_respawn_if_detected(&s1);
    assert_eq!(mem.mains_lost, 0, "first-time observation is not a respawn");
    mem.update(&s1);

    // turn 2: тот же id=A — норма, не респавн
    let s2 = empty_state(2, vec![plant("A", 10, 10, true)]);
    mem.handle_respawn_if_detected(&s2);
    assert_eq!(mem.mains_lost, 0, "same main id on same pos is not a respawn");
    mem.update(&s2);

    // turn 20: другой id=B, другая позиция — сервер убил A, спавнил B.
    let s3 = empty_state(20, vec![plant("B", 50, 50, true)]);
    mem.handle_respawn_if_detected(&s3);
    assert_eq!(mem.mains_lost, 1, "main id change must be detected as respawn");
}

#[test]
fn suspected_enemy_immunity_first_seen_only() {
    let mem = Memory::default();
    // Никогда не видели — с HP=50 считаем иммунитетом.
    assert!(mem.suspected_enemy_immunity("enemy1", 50));
    // С HP=49 — уже считаем, что его ранили, иммунитета нет.
    assert!(!mem.suspected_enemy_immunity("enemy1", 49));

    // После того как update занесёт id в enemy_first_seen — никогда больше true.
    let mut mem = Memory::default();
    let mut state = empty_state(5, vec![plant("m", 0, 0, true)]);
    state.enemies = vec![datssol_bot::model::state::EnemyPlantation {
        id: "enemy1".into(),
        pos: Pos::new(5, 5),
        hp: 50,
    }];
    mem.update(&state);
    // При повторной встрече — не в иммунитете
    assert!(!mem.suspected_enemy_immunity("enemy1", 50));
}

#[test]
fn plantation_immunity_from_none_defaults_to_zero() {
    // Fix 10: Option<u32>::None → 0 (нет иммунитета).
    use datssol_bot::api::dto::PlantationDTO;
    let dto = PlantationDTO {
        id: "x".into(),
        position: [0, 0],
        hp: 50,
        is_main: true,
        is_isolated: false,
        immunity_until_turn: None,
    };
    let p = Plantation::from_dto(dto);
    assert_eq!(p.immunity_until_turn, 0);
}

#[test]
fn derived_params_cs_is_constant() {
    // Fix 3: repair_power повышает RS но не CS.
    use datssol_bot::model::state::UpgradeTier;
    use datssol_bot::model::DerivedParams;
    let mut state = empty_state(1, vec![]);
    state.upgrades.tiers = vec![UpgradeTier {
        name: "repair_power".into(),
        current: 3,
        max: 3,
    }];
    let p = DerivedParams::from_state(&state);
    assert_eq!(p.cs, 5, "Fix 3: CS must remain 5 irrespective of repair_power");
    assert_eq!(p.rs, 8, "RS = 5 + repair_power");
}
