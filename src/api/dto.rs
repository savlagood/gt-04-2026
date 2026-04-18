#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponse {
    pub turn_no: Option<u32>,
    pub next_turn_in: Option<f64>,
    pub size: Option<[i32; 2]>,
    pub action_range: Option<i32>,
    #[serde(default)]
    pub plantations: Vec<PlantationDTO>,
    #[serde(default)]
    pub enemy: Vec<EnemyPlantationDTO>,
    #[serde(default)]
    pub mountains: Vec<[i32; 2]>,
    #[serde(default)]
    pub cells: Vec<TerraformedCellDTO>,
    #[serde(default)]
    pub construction: Vec<ConstructionDTO>,
    #[serde(default)]
    pub beavers: Vec<PlayerBeaverDTO>,
    pub plantation_upgrades: Option<PlantationUpgradesState>,
    #[serde(default)]
    pub meteo_forecasts: Vec<MeteoForecastDTO>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantationDTO {
    pub id: String,
    pub position: [i32; 2],
    pub hp: i32,
    #[serde(default)]
    pub is_main: bool,
    #[serde(default)]
    pub is_isolated: bool,
    pub immunity_until_turn: Option<u32>,
}

/// EnemyPlantationDTO — по openapi.yml содержит только hp/id/position.
/// Никакого immunityUntilTurn тут нет (Fix 2).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnemyPlantationDTO {
    pub id: String,
    pub position: [i32; 2],
    pub hp: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionDTO {
    pub position: [i32; 2],
    pub progress: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerBeaverDTO {
    pub id: String,
    pub position: [i32; 2],
    pub hp: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerraformedCellDTO {
    pub position: [i32; 2],
    pub terraformation_progress: i32,
    pub turns_until_degradation: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeteoForecastDTO {
    /// "earthquake" | "sandstorm" | ...
    pub kind: String,
    /// earthquake: turns until quake (0 = this turn).
    /// sandstorm: present only while forming.
    pub turns_until: Option<u32>,
    /// sandstorm-only id (changes when new storm spawns).
    pub id: Option<String>,
    /// sandstorm-only: true while gathering, false while moving.
    pub forming: Option<bool>,
    /// sandstorm-only: storm center.
    pub position: Option<[i32; 2]>,
    /// sandstorm-only: next step center (omitted if disk would leave map).
    pub next_position: Option<[i32; 2]>,
    /// sandstorm-only disk radius r (dx² + dy² ≤ r²).
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantationUpgradesState {
    pub points: i32,
    pub interval_turns: Option<u32>,
    pub turns_until_points: Option<u32>,
    pub max_points: Option<i32>,
    #[serde(default)]
    pub tiers: Vec<PlantationUpgradeTierItemDTO>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantationUpgradeTierItemDTO {
    pub name: String,
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDTO {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<PlantationActionDTO>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub plantation_upgrade: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relocate_main: Vec<[i32; 2]>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlantationActionDTO {
    /// [author, relay, target] — ровно 3 координаты.
    pub path: Vec<[i32; 2]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicError {
    pub code: i32,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogMessage {
    pub time: String,
    pub message: String,
}
