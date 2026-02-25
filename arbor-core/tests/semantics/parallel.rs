use std::panic::{AssertUnwindSafe, catch_unwind};

use arbor_core::{Node, Parallel, ParallelPolicy, Status};

use crate::common::{ScriptedLeaf, TickCtx, block_on};

#[tokio::test]
async fn parallel_ticks_all_children_every_tick() {
    let mut tree = Parallel::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Failure);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

#[tokio::test]
async fn parallel_policy_thresholds_work() {
    let mut ctx = TickCtx::new(3);

    let mut all_any = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Failure]),
        ),
        ParallelPolicy::SuccessOnAllFailureOnAny,
    );

    let mut any_all = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Failure]),
        ),
        ParallelPolicy::SuccessOnAnyFailureOnAll,
    );

    let mut threshold_two = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Failure]),
        ),
        ParallelPolicy::SuccessThreshold(2),
    );

    assert_eq!(all_any.tick(&mut ctx).await, Status::Failure);
    assert_eq!(any_all.tick(&mut ctx).await, Status::Success);
    assert_eq!(threshold_two.tick(&mut ctx).await, Status::Success);
}

#[tokio::test]
async fn parallel_all_running_returns_running() {
    let mut tree = Parallel::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Running]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Running);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

#[tokio::test]
async fn parallel_threshold_boundary_values_are_respected() {
    let mut ctx = TickCtx::new(3);

    let mut threshold_one = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Failure]),
            ScriptedLeaf::new(1, vec![Status::Running]),
            ScriptedLeaf::new(2, vec![Status::Success]),
        ),
        ParallelPolicy::SuccessThreshold(1),
    );

    let mut threshold_three = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Failure]),
        ),
        ParallelPolicy::SuccessThreshold(3),
    );

    assert_eq!(threshold_one.tick(&mut ctx).await, Status::Success);
    assert_eq!(threshold_three.tick(&mut ctx).await, Status::Failure);
}

#[test]
fn parallel_panics_with_zero_children() {
    let mut tree: Parallel<[ScriptedLeaf; 0]> = Parallel::new([]);
    let mut ctx = TickCtx::new(0);

    let result = catch_unwind(AssertUnwindSafe(|| {
        block_on(async { tree.tick(&mut ctx).await })
    }));

    assert!(result.is_err());
}

#[test]
fn parallel_panics_with_zero_success_threshold() {
    let mut tree = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Success]),
        ),
        ParallelPolicy::SuccessThreshold(0),
    );
    let mut ctx = TickCtx::new(3);

    let result = catch_unwind(AssertUnwindSafe(|| {
        block_on(async { tree.tick(&mut ctx).await })
    }));

    assert!(result.is_err());
}

#[test]
fn parallel_panics_when_success_threshold_exceeds_child_count() {
    let mut tree = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Success]),
            ScriptedLeaf::new(2, vec![Status::Success]),
        ),
        ParallelPolicy::SuccessThreshold(4),
    );
    let mut ctx = TickCtx::new(3);

    let result = catch_unwind(AssertUnwindSafe(|| {
        block_on(async { tree.tick(&mut ctx).await })
    }));

    assert!(result.is_err());
}
