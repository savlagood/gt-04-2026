#![allow(dead_code)]

#[derive(Debug, Default)]
pub struct Metrics {
    pub turns_processed: u32,
    pub plans_computed: u32,
    pub plan_duration_ms: Vec<u64>,
    pub plantations_peak: i32,
    pub sabotages_done: u32,
    pub beavers_killed: u32,
    pub mains_lost: u32,
    pub relay_overuse_events: u32,
    pub empty_turns: u32,
    pub api_errors: u32,
}

impl Metrics {
    pub fn record_plan_duration(&mut self, ms: u64) {
        self.plan_duration_ms.push(ms);
    }

    pub fn avg_plan_ms(&self) -> u64 {
        if self.plan_duration_ms.is_empty() {
            0
        } else {
            self.plan_duration_ms.iter().sum::<u64>() / self.plan_duration_ms.len() as u64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "turns={} avg_plan_ms={} peak_plantations={} mains_lost={} api_errors={} empty_turns={}",
            self.turns_processed,
            self.avg_plan_ms(),
            self.plantations_peak,
            self.mains_lost,
            self.api_errors,
            self.empty_turns,
        )
    }
}
