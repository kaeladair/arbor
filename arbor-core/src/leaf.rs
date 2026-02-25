use core::future::Future;

use crate::{Node, Status};

pub struct Action<F> {
    f: F,
}

impl<F> Action<F> {
    pub const fn new(f: F) -> Self {
        Self { f }
    }

    pub fn into_inner(self) -> F {
        self.f
    }
}

impl<Ctx, F, Fut> Node<Ctx> for Action<F>
where
    F: FnMut(&mut Ctx) -> Fut,
    Fut: Future<Output = Status>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        (self.f)(ctx).await
    }
}

pub struct Condition<F> {
    predicate: F,
}

impl<F> Condition<F> {
    pub const fn new(predicate: F) -> Self {
        Self { predicate }
    }

    pub fn into_inner(self) -> F {
        self.predicate
    }
}

impl<Ctx, F> Node<Ctx> for Condition<F>
where
    F: FnMut(&Ctx) -> bool,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        if (self.predicate)(ctx) {
            Status::Success
        } else {
            Status::Failure
        }
    }
}

pub struct Constant {
    status: Status,
}

impl Constant {
    pub const fn new(status: Status) -> Self {
        Self { status }
    }
}

impl<Ctx> Node<Ctx> for Constant {
    async fn tick(&mut self, _ctx: &mut Ctx) -> Status {
        self.status
    }
}
