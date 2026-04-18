#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::model::state::{GameState, Pos};

/// Граф 4-связности наших плантаций. Узлы — плантации, рёбра — соседство
/// по 4 сторонам (task.md §Логистика управления).
pub struct ChainGraph {
    nodes: Vec<String>,
    positions: Vec<Pos>,
    pos_to_idx: HashMap<Pos, usize>,
    adj: Vec<Vec<usize>>,
    main_idx: Option<usize>,
}

impl ChainGraph {
    pub fn build(state: &GameState) -> Self {
        let n = state.plantations.len();
        let nodes: Vec<String> = state.plantations.iter().map(|p| p.id.clone()).collect();
        let positions: Vec<Pos> = state.plantations.iter().map(|p| p.pos).collect();
        let mut pos_to_idx = HashMap::with_capacity(n);
        let mut main_idx = None;
        for (i, p) in state.plantations.iter().enumerate() {
            pos_to_idx.insert(p.pos, i);
            if p.is_main {
                main_idx = Some(i);
            }
        }
        let mut adj = vec![Vec::new(); n];
        for (i, p) in state.plantations.iter().enumerate() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let np = Pos::new(p.pos.x + dx, p.pos.y + dy);
                if let Some(&j) = pos_to_idx.get(&np) {
                    adj[i].push(j);
                }
            }
        }
        Self {
            nodes,
            positions,
            pos_to_idx,
            adj,
            main_idx,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn main_idx(&self) -> Option<usize> {
        self.main_idx
    }

    pub fn pos_of(&self, idx: usize) -> Pos {
        self.positions[idx]
    }

    pub fn id_of(&self, idx: usize) -> &str {
        &self.nodes[idx]
    }

    pub fn idx_of_pos(&self, p: Pos) -> Option<usize> {
        self.pos_to_idx.get(&p).copied()
    }

    pub fn is_adjacent_to_any(&self, p: Pos) -> bool {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dy)| self.pos_to_idx.contains_key(&Pos::new(p.x + dx, p.y + dy)))
    }

    /// Множество индексов плантаций, связанных с ЦУ по цепочке соседств.
    /// Пусто, если ЦУ отсутствует.
    pub fn connected_to_main(&self) -> HashSet<usize> {
        let mut visited = HashSet::new();
        let start = match self.main_idx {
            Some(m) => m,
            None => return visited,
        };
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if !visited.insert(v) {
                continue;
            }
            for &u in &self.adj[v] {
                if !visited.contains(&u) {
                    stack.push(u);
                }
            }
        }
        visited
    }

    /// Cut vertices (articulation points) по алгоритму Тарьяна.
    /// Удаление такой плантации разрывает связность части сети.
    pub fn articulation_points(&self) -> HashSet<usize> {
        let n = self.nodes.len();
        let mut visited = vec![false; n];
        let mut disc = vec![0i32; n];
        let mut low = vec![0i32; n];
        let mut parent = vec![-1i32; n];
        let mut ap = HashSet::new();
        let mut timer = 0;
        for v in 0..n {
            if !visited[v] {
                dfs_ap(
                    v,
                    &self.adj,
                    &mut visited,
                    &mut disc,
                    &mut low,
                    &mut parent,
                    &mut ap,
                    &mut timer,
                );
            }
        }
        ap
    }
}

#[allow(clippy::too_many_arguments)]
fn dfs_ap(
    v: usize,
    adj: &[Vec<usize>],
    visited: &mut [bool],
    disc: &mut [i32],
    low: &mut [i32],
    parent: &mut [i32],
    ap: &mut HashSet<usize>,
    timer: &mut i32,
) {
    visited[v] = true;
    *timer += 1;
    disc[v] = *timer;
    low[v] = *timer;
    let mut children = 0;
    for &u in &adj[v] {
        if !visited[u] {
            children += 1;
            parent[u] = v as i32;
            dfs_ap(u, adj, visited, disc, low, parent, ap, timer);
            low[v] = low[v].min(low[u]);
            if parent[v] == -1 && children > 1 {
                ap.insert(v);
            }
            if parent[v] != -1 && low[u] >= disc[v] {
                ap.insert(v);
            }
        } else if u as i32 != parent[v] {
            low[v] = low[v].min(disc[u]);
        }
    }
}
