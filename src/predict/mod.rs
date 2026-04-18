pub mod completion;
pub mod damage;
pub mod limit;
pub mod storm;

pub use completion::{turns_until_complete, will_complete_this_turn};
pub use damage::{predict_construction_damage, predict_hp_next_turn, predicted_damage};
pub use limit::{analyze_limit, safe_to_start_new_build, LimitAnalysis};
pub use storm::{predict_storm, storm_threatens, StormPrediction};
