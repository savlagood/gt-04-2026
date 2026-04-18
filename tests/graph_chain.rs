use datssol_bot::graph::ChainGraph;
use datssol_bot::model::state::{GameState, Plantation, Pos, UpgradesState};

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

fn state(plants: Vec<Plantation>) -> GameState {
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

#[test]
fn empty_graph() {
    let g = ChainGraph::build(&state(vec![]));
    assert_eq!(g.len(), 0);
    assert!(g.connected_to_main().is_empty());
    assert!(g.articulation_points().is_empty());
}

#[test]
fn single_node_no_ap() {
    let g = ChainGraph::build(&state(vec![plant("m", 0, 0, true)]));
    assert_eq!(g.connected_to_main().len(), 1);
    assert!(g.articulation_points().is_empty());
}

#[test]
fn line_three_middle_is_ap() {
    // 0 - 1 - 2 (main)
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, false),
        plant("b", 1, 0, false),
        plant("c", 2, 0, true),
    ]));
    let ap = g.articulation_points();
    let b_idx = g.idx_of_pos(Pos::new(1, 0)).unwrap();
    assert!(ap.contains(&b_idx), "middle node must be articulation");
    assert_eq!(ap.len(), 1);
    assert_eq!(g.connected_to_main().len(), 3);
}

#[test]
fn line_three_plus_branch_center_is_ap() {
    // 0 - 1 - 2
    //     |
    //     3
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, true),
        plant("b", 1, 0, false),
        plant("c", 2, 0, false),
        plant("d", 1, 1, false),
    ]));
    let ap = g.articulation_points();
    let b_idx = g.idx_of_pos(Pos::new(1, 0)).unwrap();
    assert!(ap.contains(&b_idx));
    assert_eq!(g.connected_to_main().len(), 4);
}

#[test]
fn cycle_of_four_has_no_ap() {
    // (0,0) - (1,0)
    //  |        |
    // (0,1) - (1,1)
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, true),
        plant("b", 1, 0, false),
        plant("c", 0, 1, false),
        plant("d", 1, 1, false),
    ]));
    assert!(g.articulation_points().is_empty());
    assert_eq!(g.connected_to_main().len(), 4);
}

#[test]
fn bowtie_center_is_ap() {
    // Два треугольника через общую вершину (1,0):
    //   (0,0) - (1,0) - (2,0)
    //            /          ^
    // Без диагоналей нельзя сделать треугольник с 4-adj.
    // Делаем два «квадрата», соединённых через одну клетку:
    //   (0,0)-(1,0)         (3,0)
    //    |  \  |             |
    //   (0,1)-(1,1)-(2,1)-(3,1)
    // Вершина (1,1) или (2,1) — точки сочленения цепочки к правому квадрату.
    // Для простоты — линия + цикл слева:
    //   Cycle: (0,0)-(1,0)-(0,1)-(1,1)-...   линия справа: (1,0)-(2,0)-(3,0)
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, false),
        plant("b", 1, 0, true),
        plant("c", 0, 1, false),
        plant("d", 1, 1, false),
        plant("e", 2, 0, false),
        plant("f", 3, 0, false),
    ]));
    let ap = g.articulation_points();
    // b(1,0) и e(2,0) — соединяют цикл abcd с хвостом ef.
    let b_idx = g.idx_of_pos(Pos::new(1, 0)).unwrap();
    let e_idx = g.idx_of_pos(Pos::new(2, 0)).unwrap();
    assert!(ap.contains(&b_idx), "b(1,0) must be articulation");
    assert!(ap.contains(&e_idx), "e(2,0) must be articulation");
}

#[test]
fn two_components_bfs_stays_in_one() {
    // Левый компонент с ЦУ: (0,0)-(1,0). Правый без ЦУ: (5,5)-(6,5).
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, true),
        plant("b", 1, 0, false),
        plant("c", 5, 5, false),
        plant("d", 6, 5, false),
    ]));
    let reachable = g.connected_to_main();
    assert_eq!(reachable.len(), 2);
    // Проверим, что c и d туда не попали.
    let c_idx = g.idx_of_pos(Pos::new(5, 5)).unwrap();
    let d_idx = g.idx_of_pos(Pos::new(6, 5)).unwrap();
    assert!(!reachable.contains(&c_idx));
    assert!(!reachable.contains(&d_idx));
}

#[test]
fn no_main_connected_empty() {
    let g = ChainGraph::build(&state(vec![
        plant("a", 0, 0, false),
        plant("b", 1, 0, false),
    ]));
    assert!(g.connected_to_main().is_empty());
}
