#![allow(dead_code)]

use crate::geom::is_in_disk;
use crate::model::state::{MeteoForecast, Pos};

#[derive(Debug, Clone)]
pub struct StormPrediction {
    pub id: String,
    pub positions: Vec<Pos>, // центры на следующие N ходов
    pub radius: i32,
}

/// Линейная экстраполяция по вектору (next - current).
/// Возвращает None, если буря в forming-фазе (task.md: «пока не сформирована —
/// не наносит урон») или если нет полных координат.
pub fn predict_storm(m: &MeteoForecast, turns_ahead: u32) -> Option<StormPrediction> {
    if m.kind != "sandstorm" {
        return None;
    }
    if m.forming.unwrap_or(true) {
        return None;
    }
    let pos = m.position?;
    let next = m.next_position?;
    let r = m.radius?;
    let dx = next.x - pos.x;
    let dy = next.y - pos.y;
    let mut positions = Vec::with_capacity(turns_ahead as usize);
    for t in 1..=(turns_ahead as i32) {
        positions.push(Pos::new(pos.x + dx * t, pos.y + dy * t));
    }
    Some(StormPrediction {
        id: m.id.clone().unwrap_or_default(),
        positions,
        radius: r,
    })
}

pub fn storm_threatens(cell: Pos, preds: &[StormPrediction]) -> bool {
    preds
        .iter()
        .any(|p| p.positions.iter().any(|c| is_in_disk(cell, *c, p.radius)))
}
