#![allow(dead_code)]

use std::{cell::Cell, rc::Rc, time::Duration};

use arbor_core::{Clock, Node, Status};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSnapshot {
    pub ticks: Vec<usize>,
    pub resets: Vec<usize>,
    pub leaf_statuses: Vec<Vec<Status>>,
    pub root_statuses: Vec<Status>,
}

#[derive(Debug, Default, Clone)]
pub struct TickCtx {
    pub ticks: Vec<usize>,
    pub leaf_statuses: Vec<Vec<Status>>,
    pub root_statuses: Vec<Status>,
}

impl TickCtx {
    pub fn new(leaves: usize) -> Self {
        Self {
            ticks: vec![0; leaves],
            leaf_statuses: vec![Vec::new(); leaves],
            root_statuses: Vec::new(),
        }
    }

    pub fn record_root(&mut self, status: Status) {
        self.root_statuses.push(status);
    }

    pub fn snapshot(&self) -> TraceSnapshot {
        TraceSnapshot {
            ticks: self.ticks.clone(),
            resets: vec![0; self.ticks.len()],
            leaf_statuses: self.leaf_statuses.clone(),
            root_statuses: self.root_statuses.clone(),
        }
    }

    pub fn snapshot_with_probes(&self, probes: &[LeafProbe]) -> TraceSnapshot {
        let resets = probes.iter().map(LeafProbe::count).collect();
        TraceSnapshot {
            ticks: self.ticks.clone(),
            resets,
            leaf_statuses: self.leaf_statuses.clone(),
            root_statuses: self.root_statuses.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LeafProbe(Rc<Cell<usize>>);

impl LeafProbe {
    pub fn count(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptedLeaf {
    id: usize,
    script: Vec<Status>,
    cursor: usize,
    rewind_on_reset: bool,
    reset_counter: Rc<Cell<usize>>,
}

impl ScriptedLeaf {
    pub fn new(id: usize, script: Vec<Status>) -> Self {
        Self {
            id,
            script,
            cursor: 0,
            rewind_on_reset: true,
            reset_counter: Rc::new(Cell::new(0)),
        }
    }

    pub fn with_reset_behavior(id: usize, script: Vec<Status>, rewind_on_reset: bool) -> Self {
        Self {
            id,
            script,
            cursor: 0,
            rewind_on_reset,
            reset_counter: Rc::new(Cell::new(0)),
        }
    }

    pub fn with_probe(id: usize, script: Vec<Status>, rewind_on_reset: bool) -> (Self, LeafProbe) {
        let counter = Rc::new(Cell::new(0));
        (
            Self {
                id,
                script,
                cursor: 0,
                rewind_on_reset,
                reset_counter: counter.clone(),
            },
            LeafProbe(counter),
        )
    }

    pub fn sticky(id: usize, status: Status) -> Self {
        Self::new(id, vec![status])
    }

    fn current_status(&self) -> Status {
        self.script
            .get(self.cursor)
            .copied()
            .or_else(|| self.script.last().copied())
            .unwrap_or(Status::Failure)
    }
}

impl Node<TickCtx> for ScriptedLeaf {
    async fn tick(&mut self, ctx: &mut TickCtx) -> Status {
        if self.id >= ctx.ticks.len() {
            panic!(
                "leaf id {} out of bounds for tick vector of length {}",
                self.id,
                ctx.ticks.len()
            );
        }

        ctx.ticks[self.id] += 1;

        let status = self.current_status();
        ctx.leaf_statuses[self.id].push(status);

        if self.cursor + 1 < self.script.len() {
            self.cursor += 1;
        }

        status
    }

    fn reset(&mut self) {
        self.reset_counter
            .set(self.reset_counter.get().saturating_add(1));
        if self.rewind_on_reset {
            self.cursor = 0;
        }
    }
}

pub fn leaf(id: usize, script: &[Status]) -> ScriptedLeaf {
    ScriptedLeaf::new(id, script.to_vec())
}

pub fn block_on<F>(future: F) -> F::Output
where
    F: core::future::Future,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime must build");
    runtime.block_on(future)
}

#[derive(Clone, Debug, Default)]
pub struct MockClock {
    now: Rc<Cell<Duration>>,
}

impl MockClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&self, delta: Duration) {
        self.now.set(self.now.get().saturating_add(delta));
    }
}

impl Clock for MockClock {
    type Instant = Duration;

    fn now(&self) -> Self::Instant {
        self.now.get()
    }

    fn elapsed(&self, since: Self::Instant) -> Duration {
        self.now.get().saturating_sub(since)
    }
}
