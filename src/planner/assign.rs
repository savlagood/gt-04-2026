#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::model::params::DerivedParams;
use crate::model::state::GameState;
use crate::model::state::Pos;
use crate::planner::tasks::{Assignment, Task};
use crate::tactics::choose_relay;

pub fn assign_tasks(
    tasks: Vec<Task>,
    state: &GameState,
    params: &DerivedParams,
) -> Vec<Assignment> {
    let mut tasks = tasks;
    tasks.sort_by(|a, b| {
        b.composite_score()
            .partial_cmp(&a.composite_score())
            .unwrap()
    });

    let mut used_authors: HashSet<String> = HashSet::new();
    let mut relay_usage: HashMap<Pos, u32> = HashMap::new();
    let mut assignments = Vec::new();

    for task in tasks {
        let base = task.kind.base_stat(params);

        // Найти лучший вариант (автор, реле)
        let mut best: Option<(String, Pos, Pos, i32)> = None;
        for author in state.controllable() {
            if used_authors.contains(&author.id) {
                continue;
            }
            if let Some(choice) =
                choose_relay(author, task.target, base, state, params, &relay_usage)
            {
                let score = choice.expected_effect;
                if best.as_ref().map(|(_, _, _, s)| score > *s).unwrap_or(true) {
                    best = Some((
                        author.id.clone(),
                        author.pos,
                        choice.relay_pos,
                        score,
                    ));
                }
            }
        }

        if let Some((aid, apos, rpos, eff)) = best {
            used_authors.insert(aid.clone());
            *relay_usage.entry(rpos).or_insert(0) += 1;
            assignments.push(Assignment {
                author_id: aid,
                author_pos: apos,
                relay_pos: rpos,
                target_pos: task.target,
                kind: task.kind,
                expected_effect: eff,
            });
        }
    }
    assignments
}