use crate::model::state::Pos;

/// task.md §Терраформация: клетки с координатами кратными 7 — бустовые.
pub const BOOST_MOD: i32 = 7;

pub fn is_boosted(p: Pos) -> bool {
    p.x.rem_euclid(BOOST_MOD) == 0 && p.y.rem_euclid(BOOST_MOD) == 0
}

pub fn in_bounds(p: Pos, size: (i32, i32)) -> bool {
    p.x >= 0 && p.y >= 0 && p.x < size.0 && p.y < size.1
}

/// Евклидов диск — формат бури (openapi.yml: `dx² + dy² ≤ r²`).
pub fn is_in_disk(p: Pos, center: Pos, radius: i32) -> bool {
    let dx = (p.x - center.x) as i64;
    let dy = (p.y - center.y) as i64;
    dx * dx + dy * dy <= (radius as i64) * (radius as i64)
}

pub fn cell_base_points(p: Pos) -> i32 {
    if is_boosted(p) { 1500 } else { 1000 }
}

pub fn cell_per_turn_yield(p: Pos, ts: i32) -> i32 {
    // 10 очков за 1% на обычной клетке, 15 на бустовой (task.md §Терраформация).
    if is_boosted(p) { ts * 15 } else { ts * 10 }
}
