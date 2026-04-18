use datssol_bot::geom::{
    adjacent4, cell_base_points, chebyshev, in_bounds, in_range, is_boosted, is_in_disk, manhattan,
};
use datssol_bot::model::state::Pos;

#[test]
fn chebyshev_cases() {
    assert_eq!(chebyshev(Pos::new(0, 0), Pos::new(3, 2)), 3);
    assert_eq!(chebyshev(Pos::new(0, 0), Pos::new(-3, 2)), 3);
    assert_eq!(chebyshev(Pos::new(5, 5), Pos::new(5, 5)), 0);
}

#[test]
fn manhattan_cases() {
    assert_eq!(manhattan(Pos::new(0, 0), Pos::new(3, 2)), 5);
    assert_eq!(manhattan(Pos::new(1, 1), Pos::new(1, 1)), 0);
}

#[test]
fn adjacent4_only_four_directions() {
    let a = Pos::new(5, 5);
    assert!(adjacent4(a, Pos::new(6, 5)));
    assert!(adjacent4(a, Pos::new(4, 5)));
    assert!(adjacent4(a, Pos::new(5, 6)));
    assert!(adjacent4(a, Pos::new(5, 4)));
    assert!(!adjacent4(a, Pos::new(6, 6))); // диагональ — нет!
    assert!(!adjacent4(a, Pos::new(5, 5))); // самому себе — нет
}

#[test]
fn in_range_square_area() {
    let c = Pos::new(0, 0);
    // AR=2 → квадрат 5×5
    assert!(in_range(c, Pos::new(2, 2), 2));
    assert!(in_range(c, Pos::new(-2, 2), 2));
    assert!(!in_range(c, Pos::new(3, 0), 2));
    assert!(!in_range(c, Pos::new(0, 3), 2));
}

#[test]
fn is_boosted_cases() {
    assert!(is_boosted(Pos::new(0, 0)));     // граничный
    assert!(is_boosted(Pos::new(7, 7)));
    assert!(is_boosted(Pos::new(7, 14)));
    assert!(is_boosted(Pos::new(14, 7)));
    assert!(!is_boosted(Pos::new(7, 15)));
    assert!(!is_boosted(Pos::new(1, 1)));
    // отрицательные: -7 тоже кратно 7
    assert!(is_boosted(Pos::new(-7, 0)));
    assert!(!is_boosted(Pos::new(-3, 0)));
}

#[test]
fn in_bounds_exclusive_upper() {
    let size = (100, 100);
    assert!(in_bounds(Pos::new(0, 0), size));
    assert!(in_bounds(Pos::new(99, 99), size));
    assert!(!in_bounds(Pos::new(100, 0), size));
    assert!(!in_bounds(Pos::new(0, 100), size));
    assert!(!in_bounds(Pos::new(-1, 0), size));
}

#[test]
fn is_in_disk_euclidean() {
    let c = Pos::new(0, 0);
    assert!(is_in_disk(c, c, 0));
    assert!(is_in_disk(Pos::new(3, 4), c, 5)); // 9+16=25 ≤ 25
    assert!(!is_in_disk(Pos::new(4, 4), c, 5)); // 16+16=32 > 25
    assert!(is_in_disk(Pos::new(5, 0), c, 5));
}

#[test]
fn cell_base_points_values() {
    assert_eq!(cell_base_points(Pos::new(0, 0)), 1500);
    assert_eq!(cell_base_points(Pos::new(7, 7)), 1500);
    assert_eq!(cell_base_points(Pos::new(1, 1)), 1000);
}
