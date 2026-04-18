use crate::model::state::Pos;

/// Chebyshev distance — `max(|dx|, |dy|)`. Это метрика для AR/SR/VR
/// (task.md §Радиусы: «зона действия радиуса формирует квадратную область»).
pub fn chebyshev(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

pub fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Смежность по 4-связности (без диагоналей). Используется для логистики управления
/// (task.md §Логистика управления: «по диагонали не считается»).
pub fn adjacent4(a: Pos, b: Pos) -> bool {
    manhattan(a, b) == 1
}

pub fn in_range(a: Pos, b: Pos, r: i32) -> bool {
    chebyshev(a, b) <= r
}
