use datssol_bot::model::state::{
    Beaver, Construction, GameState, MeteoForecast, Plantation, Pos, UpgradesState,
};
use datssol_bot::model::DerivedParams;
use datssol_bot::predict::{
    predict_construction_damage, predict_hp_next_turn, will_complete_this_turn,
};

fn plant(id: &str, x: i32, y: i32, hp: i32, immunity_until: u32) -> Plantation {
    Plantation {
        id: id.into(),
        pos: Pos::new(x, y),
        hp,
        is_main: false,
        is_isolated: false,
        immunity_until_turn: immunity_until,
    }
}

fn base_state(turn: u32, plants: Vec<Plantation>) -> GameState {
    GameState {
        turn_no: turn,
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

fn sandstorm(id: &str, center: Pos, next: Pos, r: i32) -> MeteoForecast {
    MeteoForecast {
        kind: "sandstorm".into(),
        turns_until: None,
        id: Some(id.into()),
        forming: Some(false),
        position: Some(center),
        next_position: Some(next),
        radius: Some(r),
    }
}

fn earthquake(turns_until: u32) -> MeteoForecast {
    MeteoForecast {
        kind: "earthquake".into(),
        turns_until: Some(turns_until),
        id: None,
        forming: None,
        position: None,
        next_position: None,
        radius: None,
    }
}

#[test]
fn predict_storm_alone_clamps_to_one() {
    // Буря одна, HP=3, storm_dmg=2 → HP после = 1 (не 0).
    let p = plant("p", 5, 5, 3, 0);
    let mut s = base_state(10, vec![p.clone()]);
    s.meteo = vec![sandstorm("s1", Pos::new(4, 5), Pos::new(5, 5), 2)];
    let params = DerivedParams::from_state(&s);
    assert_eq!(predict_hp_next_turn(&p, &s, &params), 1);
}

#[test]
fn predict_storm_plus_beaver_storm_does_not_save() {
    // Буря + бобр, HP=10, beaver_dmg=15 → HP=-5 (буря не спасает от бобра).
    let p = plant("p", 5, 5, 10, 0);
    let mut s = base_state(10, vec![p.clone()]);
    s.meteo = vec![sandstorm("s1", Pos::new(4, 5), Pos::new(5, 5), 2)];
    s.beavers = vec![Beaver {
        id: "b".into(),
        pos: Pos::new(6, 5),
        hp: 100,
    }];
    let params = DerivedParams::from_state(&s);
    // lethal = 15 (beaver). hp_after_lethal = -5, сразу возвращаем его — буря не в счёт.
    assert_eq!(predict_hp_next_turn(&p, &s, &params), -5);
}

#[test]
fn predict_immunity_blocks_damage() {
    let p = plant("p", 5, 5, 50, 100); // immunity_until=100, turn=10
    let mut s = base_state(10, vec![p.clone()]);
    s.beavers = vec![Beaver {
        id: "b".into(),
        pos: Pos::new(5, 5),
        hp: 100,
    }];
    let params = DerivedParams::from_state(&s);
    assert_eq!(predict_hp_next_turn(&p, &s, &params), 50);
}

#[test]
fn predict_earthquake_plus_beaver_stacks() {
    // Бобёр в 2 клетках + землетряс на 0 ходу. mitigation=0 → 15+10=25 урона.
    let p = plant("p", 5, 5, 50, 0);
    let mut s = base_state(10, vec![p.clone()]);
    s.beavers = vec![Beaver {
        id: "b".into(),
        pos: Pos::new(6, 5), // chebyshev=1, в радиусе 2
        hp: 100,
    }];
    s.meteo = vec![earthquake(0)];
    let params = DerivedParams::from_state(&s);
    assert_eq!(predict_hp_next_turn(&p, &s, &params), 50 - 15 - 10);
}

#[test]
fn predict_storm_forming_does_nothing() {
    let p = plant("p", 5, 5, 50, 0);
    let mut s = base_state(10, vec![p.clone()]);
    s.meteo = vec![MeteoForecast {
        kind: "sandstorm".into(),
        turns_until: Some(3),
        id: Some("s".into()),
        forming: Some(true),
        position: Some(Pos::new(5, 5)),
        next_position: None,
        radius: Some(2),
    }];
    let params = DerivedParams::from_state(&s);
    assert_eq!(predict_hp_next_turn(&p, &s, &params), 50);
}

#[test]
fn predict_construction_damage_beaver_plus_quake() {
    // Fix 11: бобёр в 2 клетках + землетряс на 0 ходу, mitigation=0 → 15+10=25.
    let c = Construction {
        pos: Pos::new(5, 5),
        progress: 10,
    };
    let mut s = base_state(10, vec![]);
    s.construction = vec![c.clone()];
    s.beavers = vec![Beaver {
        id: "b".into(),
        pos: Pos::new(6, 5),
        hp: 100,
    }];
    s.meteo = vec![earthquake(0)];
    let params = DerivedParams::from_state(&s);
    assert_eq!(predict_construction_damage(&c, &s, &params), 25);
}

#[test]
fn will_complete_this_turn_boundary() {
    use datssol_bot::model::state::Cell;
    let p = plant("p", 5, 5, 50, 0);
    let mut s = base_state(10, vec![p.clone()]);
    s.cells = vec![Cell {
        pos: Pos::new(5, 5),
        terraformation_progress: 96,
        turns_until_degradation: None,
    }];
    let params = DerivedParams::from_state(&s);
    // ts=5, 96+5=101 >= 100 → завершит.
    assert!(will_complete_this_turn(&p, &s, &params));

    s.cells[0].terraformation_progress = 94;
    // 94+5=99 < 100 → не завершит.
    assert!(!will_complete_this_turn(&p, &s, &params));
}

#[test]
fn useful_authors_filters_completing_plantation() {
    use datssol_bot::model::state::Cell;
    let a = plant("a", 0, 0, 50, 0); // на клетке, где нет записи — не завершит
    let b = plant("b", 1, 0, 50, 0); // на клетке с progress=96 — завершит в этот ход
    let mut s = base_state(10, vec![a.clone(), b.clone()]);
    s.cells = vec![Cell {
        pos: Pos::new(1, 0),
        terraformation_progress: 96,
        turns_until_degradation: None,
    }];
    let params = DerivedParams::from_state(&s);
    let ids: Vec<_> = s.useful_authors(&params).map(|p| p.id.clone()).collect();
    assert_eq!(ids, vec!["a"]);
}
