#![allow(dead_code)] // поля подключаются по мере реализации шагов 2+.

use serde::Deserialize;

use crate::error::{BotError, Result};

const EMBEDDED_CONFIG: &str = include_str!("../config.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub phases: PhasesCfg,
    pub scoring: ScoringCfg,
    pub urgency: UrgencyCfg,
    pub upgrades: UpgradesCfg,
    pub beaver: BeaverCfg,
    pub sabotage: SabotageCfg,
    pub safety: SafetyCfg,
    pub timing: TimingCfg,
    pub logging: LoggingCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhasesCfg {
    pub early_end: u32,
    pub growth_end: u32,
    pub harvest_end: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringCfg {
    pub cell: CellScoringCfg,
    pub risk: RiskScoringCfg,
    pub bonus: BonusScoringCfg,
    pub penalty: PenaltyScoringCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CellScoringCfg {
    pub base_normal: f64,
    pub base_boosted: f64,
    pub boost_factor_early: f64,
    pub boost_factor_growth: f64,
    pub boost_factor_harvest: f64,
    pub boost_factor_endgame: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskScoringCfg {
    pub beaver_in_range: f64,
    pub storm_in_path: f64,
    pub enemy_nearby: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BonusScoringCfg {
    pub nearby_boost_weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PenaltyScoringCfg {
    pub distance_from_main: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UrgencyCfg {
    pub critical_repair: f64,
    pub unfinished_construction: f64,
    pub new_build_boost: f64,
    pub new_build_normal: f64,
    pub beaver_hunt: f64,
    pub maintenance_repair: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradesCfg {
    pub priority_order: PriorityOrderCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriorityOrderCfg {
    pub sequence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeaverCfg {
    pub min_attackers: i32,
    pub safety_time_budget_frac: f64,
    pub opportunity_cost_per_turn: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SabotageCfg {
    pub allowed_in_early: bool,
    pub allowed_in_growth: bool,
    pub allowed_in_harvest: bool,
    pub only_endgame_vs_leader: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafetyCfg {
    pub main_critical_completion_turns: u32,
    pub main_critical_hp_fraction: f64,
    pub min_buffer_before_limit: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimingCfg {
    pub plan_budget_ms: u64,
    pub sleep_safety_margin_sec: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingCfg {
    pub per_turn_json: bool,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(BotError::Io)?;
        toml::from_str(&text).map_err(|e| BotError::Config(format!("parse {path}: {e}")))
    }

    pub fn default_embedded() -> Self {
        toml::from_str(EMBEDDED_CONFIG)
            .expect("embedded config.toml must always parse (compile-time bundled)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_parses() {
        let cfg = Config::default_embedded();
        assert_eq!(cfg.phases.early_end, 50);
        assert_eq!(cfg.phases.harvest_end, 500);
        assert!(!cfg.upgrades.priority_order.sequence.is_empty());
        assert_eq!(cfg.beaver.min_attackers, 2);
        assert!(cfg.logging.per_turn_json);
    }
}
