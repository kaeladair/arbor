#![allow(dead_code)]

use std::{cell::Cell, rc::Rc, time::Duration};

use arbor_core::{Clock, Node, Status};

#[derive(Debug, Default)]
pub struct TickCtx {
    pub ticks: Vec<usize>,
}

impl TickCtx {
    pub fn new(leaves: usize) -> Self {
        Self {
            ticks: vec![0; leaves],
        }
    }
}

#[derive(Debug)]
pub struct ScriptedLeaf {
    id: usize,
    script: Vec<Status>,
    cursor: usize,
    rewind_on_reset: bool,
}

impl ScriptedLeaf {
    pub fn new(id: usize, script: Vec<Status>) -> Self {
        Self {
            id,
            script,
            cursor: 0,
            rewind_on_reset: true,
        }
    }

    pub fn with_reset_behavior(id: usize, script: Vec<Status>, rewind_on_reset: bool) -> Self {
        Self {
            id,
            script,
            cursor: 0,
            rewind_on_reset,
        }
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

        if self.cursor + 1 < self.script.len() {
            self.cursor += 1;
        }

        status
    }

    fn reset(&mut self) {
        if self.rewind_on_reset {
            self.cursor = 0;
        }
    }
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
