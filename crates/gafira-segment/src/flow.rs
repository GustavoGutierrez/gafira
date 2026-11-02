//! Dinic max-flow on integer capacities: exact, so the cut is deterministic.

use std::collections::VecDeque;

use gafira_doc::contract::Cancelled;

use crate::meter::Meter;

/// A capacity no real cut reaches: a pinned node.
pub(crate) const INF: i64 = 1 << 50;

struct Arc {
    to: usize,
    cap: i64,
}

pub(crate) struct Flow {
    arcs: Vec<Arc>,
    out: Vec<Vec<usize>>,
}

impl Flow {
    pub(crate) fn new(nodes: usize) -> Self {
        Self {
            arcs: Vec::new(),
            out: vec![Vec::new(); nodes],
        }
    }

    /// An arc `a -> b` with capacity `ab` and its reverse `b -> a` with `ba`.
    pub(crate) fn add(&mut self, a: usize, b: usize, ab: i64, ba: i64) {
        self.out[a].push(self.arcs.len());
        self.arcs.push(Arc { to: b, cap: ab });
        self.out[b].push(self.arcs.len());
        self.arcs.push(Arc { to: a, cap: ba });
    }

    /// Runs max-flow from `s` to `t`; returns, per node, whether it is still
    /// reachable from `s` (the source side of the minimum cut).
    pub(crate) fn source_side(
        &mut self,
        s: usize,
        t: usize,
        meter: &Meter<'_>,
    ) -> Result<Vec<bool>, Cancelled> {
        loop {
            meter.check()?;
            let level = self.levels(s);
            if level[t] < 0 {
                return Ok(level.iter().map(|&l| l >= 0).collect());
            }
            let mut next = vec![0usize; self.out.len()];
            while self.push(s, t, INF, &level, &mut next) > 0 {}
        }
    }

    fn levels(&self, s: usize) -> Vec<i32> {
        let mut level = vec![-1; self.out.len()];
        level[s] = 0;
        let mut queue = VecDeque::from([s]);
        while let Some(u) = queue.pop_front() {
            for &a in &self.out[u] {
                let arc = &self.arcs[a];
                if arc.cap > 0 && level[arc.to] < 0 {
                    level[arc.to] = level[u] + 1;
                    queue.push_back(arc.to);
                }
            }
        }
        level
    }

    fn push(&mut self, u: usize, t: usize, limit: i64, level: &[i32], next: &mut [usize]) -> i64 {
        if u == t {
            return limit;
        }
        while next[u] < self.out[u].len() {
            let a = self.out[u][next[u]];
            let (to, cap) = (self.arcs[a].to, self.arcs[a].cap);
            if cap > 0 && level[to] == level[u] + 1 {
                let pushed = self.push(to, t, limit.min(cap), level, next);
                if pushed > 0 {
                    self.arcs[a].cap -= pushed;
                    self.arcs[a ^ 1].cap += pushed;
                    return pushed;
                }
            }
            next[u] += 1;
        }
        0
    }
}
