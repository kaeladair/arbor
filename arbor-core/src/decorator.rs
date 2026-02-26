use core::time::Duration;

use crate::{Clock, Node, Status};

pub struct Inverter<Child> {
    child: Child,
}

impl<Child> Inverter<Child> {
    pub const fn new(child: Child) -> Self {
        Self { child }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child> Node<Ctx> for Inverter<Child>
where
    Child: Node<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        match self.child.tick(ctx).await {
            Status::Success => {
                self.child.reset();
                Status::Failure
            }
            Status::Failure => {
                self.child.reset();
                Status::Success
            }
            Status::Running => Status::Running,
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

pub struct Retry<Child> {
    child: Child,
    max_failures: usize,
    failures: usize,
}

impl<Child> Retry<Child> {
    pub const fn new(child: Child, max_failures: usize) -> Self {
        Self {
            child,
            max_failures,
            failures: 0,
        }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child> Node<Ctx> for Retry<Child>
where
    Child: Node<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        if self.max_failures == 0 {
            self.child.reset();
            return Status::Failure;
        }

        match self.child.tick(ctx).await {
            Status::Success => {
                self.failures = 0;
                self.child.reset();
                Status::Success
            }
            Status::Running => Status::Running,
            Status::Failure => {
                self.failures += 1;
                self.child.reset();
                if self.failures >= self.max_failures {
                    self.failures = 0;
                    Status::Failure
                } else {
                    Status::Running
                }
            }
        }
    }

    fn reset(&mut self) {
        self.failures = 0;
        self.child.reset();
    }
}

pub struct Repeat<Child> {
    child: Child,
    max_successes: usize,
    successes: usize,
}

impl<Child> Repeat<Child> {
    pub const fn new(child: Child, max_successes: usize) -> Self {
        Self {
            child,
            max_successes,
            successes: 0,
        }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child> Node<Ctx> for Repeat<Child>
where
    Child: Node<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        if self.max_successes == 0 {
            self.child.reset();
            return Status::Success;
        }

        match self.child.tick(ctx).await {
            Status::Success => {
                self.successes += 1;
                self.child.reset();
                if self.successes >= self.max_successes {
                    self.successes = 0;
                    Status::Success
                } else {
                    Status::Running
                }
            }
            Status::Running => Status::Running,
            Status::Failure => {
                self.successes = 0;
                self.child.reset();
                Status::Failure
            }
        }
    }

    fn reset(&mut self) {
        self.successes = 0;
        self.child.reset();
    }
}

pub struct Timeout<Child, Clk: Clock> {
    child: Child,
    clock: Clk,
    duration: Duration,
    started_at: Option<Clk::Instant>,
}

impl<Child, Clk> Timeout<Child, Clk>
where
    Clk: Clock,
{
    pub const fn new(child: Child, clock: Clk, duration: Duration) -> Self {
        Self {
            child,
            clock,
            duration,
            started_at: None,
        }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child, Clk> Node<Ctx> for Timeout<Child, Clk>
where
    Child: Node<Ctx>,
    Clk: Clock,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        if let Some(started_at) = self.started_at
            && self.clock.elapsed(started_at) >= self.duration
        {
            self.started_at = None;
            self.child.reset();
            return Status::Failure;
        }

        match self.child.tick(ctx).await {
            Status::Success => {
                self.started_at = None;
                self.child.reset();
                Status::Success
            }
            Status::Failure => {
                self.started_at = None;
                self.child.reset();
                Status::Failure
            }
            Status::Running => {
                let start = match self.started_at {
                    Some(started_at) => started_at,
                    None => {
                        let now = self.clock.now();
                        self.started_at = Some(now);
                        now
                    }
                };

                if self.clock.elapsed(start) >= self.duration {
                    self.started_at = None;
                    self.child.reset();
                    Status::Failure
                } else {
                    Status::Running
                }
            }
        }
    }

    fn reset(&mut self) {
        self.started_at = None;
        self.child.reset();
    }
}

pub struct ForceSuccess<Child> {
    child: Child,
}

impl<Child> ForceSuccess<Child> {
    pub const fn new(child: Child) -> Self {
        Self { child }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child> Node<Ctx> for ForceSuccess<Child>
where
    Child: Node<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        match self.child.tick(ctx).await {
            Status::Running => Status::Running,
            Status::Success | Status::Failure => {
                self.child.reset();
                Status::Success
            }
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

pub struct ForceFailure<Child> {
    child: Child,
}

impl<Child> ForceFailure<Child> {
    pub const fn new(child: Child) -> Self {
        Self { child }
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}

impl<Ctx, Child> Node<Ctx> for ForceFailure<Child>
where
    Child: Node<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        match self.child.tick(ctx).await {
            Status::Running => Status::Running,
            Status::Success | Status::Failure => {
                self.child.reset();
                Status::Failure
            }
        }
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}
