use core::time::Duration;

pub use arbor_core::{
    Action, Clock, Condition, Constant, ForceFailure, ForceSuccess, Inverter, Node, NodeList,
    Parallel, ParallelPolicy, ReactiveSelector, ReactiveSequence, Repeat, Retry, Selector,
    Sequence, Status, Timeout,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct TokioClock;

impl Clock for TokioClock {
    type Instant = tokio::time::Instant;

    fn now(&self) -> Self::Instant {
        tokio::time::Instant::now()
    }

    fn elapsed(&self, since: Self::Instant) -> Duration {
        since.elapsed()
    }
}

pub async fn tick_until_done<Ctx, N>(root: &mut N, ctx: &mut Ctx, tick_interval: Duration) -> Status
where
    N: Node<Ctx>,
{
    loop {
        let status = root.tick(ctx).await;
        if status != Status::Running {
            return status;
        }
        tokio::time::sleep(tick_interval).await;
    }
}
