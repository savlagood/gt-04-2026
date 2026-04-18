#![allow(dead_code)]

use crate::api::dto::{
    ConstructionDTO, EnemyPlantationDTO, MeteoForecastDTO, PlantationDTO,
    PlantationUpgradeTierItemDTO, PlantationUpgradesState, PlayerBeaverDTO, PlayerResponse,
    TerraformedCellDTO,
};
use crate::error::{BotError, Result};
use crate::model::params::DerivedParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn from_arr(a: [i32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
    pub fn to_arr(self) -> [i32; 2] {
        [self.x, self.y]
    }
}

#[derive(Debug, Clone)]
pub struct Plantation {
    pub id: String,
    pub pos: Pos,
    pub hp: i32,
    pub is_main: bool,
    pub is_isolated: bool,
    /// 0 = нет иммунитета (Fix 10: Option<u32> из DTO → u32 через unwrap_or(0)).
    pub immunity_until_turn: u32,
}

#[derive(Debug, Clone)]
pub struct EnemyPlantation {
    pub id: String,
    pub pos: Pos,
    pub hp: i32,
}

#[derive(Debug, Clone)]
pub struct Construction {
    pub pos: Pos,
    pub progress: i32,
}

#[derive(Debug, Clone)]
pub struct Beaver {
    pub id: String,
    pub pos: Pos,
    pub hp: i32,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub pos: Pos,
    pub terraformation_progress: i32,
    pub turns_until_degradation: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct UpgradeTier {
    pub name: String,
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Default)]
pub struct UpgradesState {
    pub points: i32,
    pub interval_turns: u32,
    pub turns_until_points: u32,
    pub max_points: i32,
    pub tiers: Vec<UpgradeTier>,
}

#[derive(Debug, Clone)]
pub struct MeteoForecast {
    pub kind: String,
    pub turns_until: Option<u32>,
    pub id: Option<String>,
    pub forming: Option<bool>,
    pub position: Option<Pos>,
    pub next_position: Option<Pos>,
    pub radius: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub turn_no: u32,
    pub next_turn_in: f64,
    pub map_size: (i32, i32),
    pub action_range: i32,
    pub plantations: Vec<Plantation>,
    pub enemies: Vec<EnemyPlantation>,
    pub mountains: Vec<Pos>,
    pub cells: Vec<Cell>,
    pub construction: Vec<Construction>,
    pub beavers: Vec<Beaver>,
    pub upgrades: UpgradesState,
    pub meteo: Vec<MeteoForecast>,
}

impl GameState {
    pub fn from_api(resp: PlayerResponse) -> Result<Self> {
        let turn_no = resp.turn_no.ok_or_else(|| {
            BotError::Config("arena response missing turnNo (not registered yet?)".into())
        })?;
        Ok(Self {
            turn_no,
            next_turn_in: resp.next_turn_in.unwrap_or(1.0),
            map_size: resp.size.map(|s| (s[0], s[1])).unwrap_or((0, 0)),
            action_range: resp.action_range.unwrap_or(2),
            plantations: resp.plantations.into_iter().map(Plantation::from_dto).collect(),
            enemies: resp.enemy.into_iter().map(EnemyPlantation::from_dto).collect(),
            mountains: resp.mountains.into_iter().map(Pos::from_arr).collect(),
            cells: resp.cells.into_iter().map(Cell::from_dto).collect(),
            construction: resp
                .construction
                .into_iter()
                .map(Construction::from_dto)
                .collect(),
            beavers: resp.beavers.into_iter().map(Beaver::from_dto).collect(),
            upgrades: resp
                .plantation_upgrades
                .map(UpgradesState::from_dto)
                .unwrap_or_default(),
            meteo: resp
                .meteo_forecasts
                .into_iter()
                .map(MeteoForecast::from_dto)
                .collect(),
        })
    }

    pub fn main(&self) -> Option<&Plantation> {
        self.plantations.iter().find(|p| p.is_main)
    }

    pub fn by_id(&self, id: &str) -> Option<&Plantation> {
        self.plantations.iter().find(|p| p.id == id)
    }

    pub fn plantation_at(&self, pos: Pos) -> Option<&Plantation> {
        self.plantations.iter().find(|p| p.pos == pos)
    }

    pub fn cell_at(&self, pos: Pos) -> Option<&Cell> {
        self.cells.iter().find(|c| c.pos == pos)
    }

    pub fn construction_at(&self, pos: Pos) -> Option<&Construction> {
        self.construction.iter().find(|c| c.pos == pos)
    }

    pub fn controllable(&self) -> impl Iterator<Item = &Plantation> + '_ {
        self.plantations.iter().filter(|p| !p.is_isolated)
    }

    /// Fix 9: плантации, которые безопасно использовать как авторов задач —
    /// те, которые не исчезнут в конце этого хода (не завершат клетку).
    pub fn useful_authors<'a>(
        &'a self,
        params: &'a DerivedParams,
    ) -> impl Iterator<Item = &'a Plantation> + 'a {
        self.controllable()
            .filter(move |p| !crate::predict::will_complete_this_turn(p, self, params))
    }
}

impl Plantation {
    pub fn from_dto(dto: PlantationDTO) -> Self {
        Self {
            id: dto.id,
            pos: Pos::from_arr(dto.position),
            hp: dto.hp,
            is_main: dto.is_main,
            is_isolated: dto.is_isolated,
            immunity_until_turn: dto.immunity_until_turn.unwrap_or(0), // Fix 10
        }
    }
}

impl EnemyPlantation {
    pub fn from_dto(dto: EnemyPlantationDTO) -> Self {
        Self {
            id: dto.id,
            pos: Pos::from_arr(dto.position),
            hp: dto.hp,
        }
    }
}

impl Construction {
    pub fn from_dto(dto: ConstructionDTO) -> Self {
        Self {
            pos: Pos::from_arr(dto.position),
            progress: dto.progress,
        }
    }
}

impl Beaver {
    pub fn from_dto(dto: PlayerBeaverDTO) -> Self {
        Self {
            id: dto.id,
            pos: Pos::from_arr(dto.position),
            hp: dto.hp,
        }
    }
}

impl Cell {
    pub fn from_dto(dto: TerraformedCellDTO) -> Self {
        Self {
            pos: Pos::from_arr(dto.position),
            terraformation_progress: dto.terraformation_progress,
            turns_until_degradation: dto.turns_until_degradation,
        }
    }
}

impl UpgradeTier {
    pub fn from_dto(dto: PlantationUpgradeTierItemDTO) -> Self {
        Self {
            name: dto.name,
            current: dto.current,
            max: dto.max,
        }
    }
}

impl UpgradesState {
    pub fn from_dto(dto: PlantationUpgradesState) -> Self {
        Self {
            points: dto.points,
            interval_turns: dto.interval_turns.unwrap_or(30),
            turns_until_points: dto.turns_until_points.unwrap_or(0),
            max_points: dto.max_points.unwrap_or(15),
            tiers: dto.tiers.into_iter().map(UpgradeTier::from_dto).collect(),
        }
    }
}

impl MeteoForecast {
    pub fn from_dto(dto: MeteoForecastDTO) -> Self {
        Self {
            kind: dto.kind,
            turns_until: dto.turns_until,
            id: dto.id,
            forming: dto.forming,
            position: dto.position.map(Pos::from_arr),
            next_position: dto.next_position.map(Pos::from_arr),
            radius: dto.radius,
        }
    }
}
