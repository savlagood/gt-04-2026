#![allow(dead_code)]

use crate::api::dto::{PlantationActionDTO, PlayerDTO};
use crate::config::Config;
use crate::model::params::DerivedParams;
use crate::model::state::Pos;

#[derive(Debug, Clone)]
pub enum TaskKind {
    Build,
    Repair { target_id: String },
    Sabotage { target_id: String },
    BeaverAttack { target_id: String },
}

impl TaskKind {
    pub fn base_stat(&self, params: &DerivedParams) -> i32 {
        match self {
            TaskKind::Build => params.cs,
            TaskKind::Repair { .. } => params.rs,
            TaskKind::Sabotage { .. } => params.se,
            TaskKind::BeaverAttack { .. } => params.be,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub kind: TaskKind,
    pub target: Pos,
    pub utility: f64,
    pub urgency: f64,
    pub required_effort: f64,
}

impl Task {
    pub fn composite_score(&self) -> f64 {
        self.urgency * 10_000.0 + self.utility / self.required_effort.max(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub author_id: String,
    pub author_pos: Pos,
    pub relay_pos: Pos,
    pub target_pos: Pos,
    pub kind: TaskKind,
    pub expected_effect: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Early,
    Growth,
    Harvest,
    Endgame,
}

impl Phase {
    pub fn from_turn(turn: u32, cfg: &Config) -> Self {
        if turn < cfg.phases.early_end {
            Phase::Early
        } else if turn < cfg.phases.growth_end {
            Phase::Growth
        } else if turn < cfg.phases.harvest_end {
            Phase::Harvest
        } else {
            Phase::Endgame
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnPlan {
    pub assignments: Vec<Assignment>,
    pub upgrade: Option<String>,
    pub relocate_main: Option<Vec<Pos>>,
}

impl TurnPlan {
    pub fn into_player_dto(self) -> PlayerDTO {
        PlayerDTO {
            command: self
                .assignments
                .into_iter()
                .map(|a| PlantationActionDTO {
                    path: vec![
                        a.author_pos.to_arr(),
                        a.relay_pos.to_arr(),
                        a.target_pos.to_arr(),
                    ],
                })
                .collect(),
            plantation_upgrade: self.upgrade.unwrap_or_default(),
            relocate_main: self
                .relocate_main
                .map(|v| v.into_iter().map(|p| p.to_arr()).collect())
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty() && self.upgrade.is_none() && self.relocate_main.is_none()
    }
}
