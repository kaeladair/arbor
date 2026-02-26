use std::time::Duration;

use arbor_core::{ForceFailure, ForceSuccess, Inverter, Node, Repeat, Retry, Status, Timeout};

use crate::common::{MockClock, ScriptedLeaf, TickCtx};

#[tokio::test]
async fn inverter_covers_all_statuses() {
    let mut ctx = TickCtx::new(1);

    let mut success = Inverter::new(ScriptedLeaf::new(0, vec![Status::Success]));
    let mut failure = Inverter::new(ScriptedLeaf::new(0, vec![Status::Failure]));
    let mut running = Inverter::new(ScriptedLeaf::new(0, vec![Status::Running]));

    assert_eq!(success.tick(&mut ctx).await, Status::Failure);
    assert_eq!(failure.tick(&mut ctx).await, Status::Success);
    assert_eq!(running.tick(&mut ctx).await, Status::Running);
}

#[tokio::test]
async fn retry_gives_up_after_n_failures() {
    let mut tree = Retry::new(ScriptedLeaf::new(0, vec![Status::Failure]), 2);
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
}

#[tokio::test]
async fn retry_counter_resets_after_success() {
    let mut tree = Retry::new(
        ScriptedLeaf::with_reset_behavior(
            0,
            vec![Status::Failure, Status::Success, Status::Failure],
            false,
        ),
        2,
    );
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
}

#[tokio::test]
async fn retry_zero_returns_failure_without_ticking_child() {
    let (leaf, probe) = ScriptedLeaf::with_probe(0, vec![Status::Success], true);
    let mut tree = Retry::new(leaf, 0);
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
    assert_eq!(ctx.ticks, vec![0]);
    let snapshot = ctx.snapshot_with_probes(&[probe]);
    assert_eq!(snapshot.resets, vec![1]);
}

#[tokio::test]
async fn repeat_repeats_success_n_times() {
    let mut tree = Repeat::new(
        ScriptedLeaf::with_reset_behavior(
            0,
            vec![Status::Success, Status::Success, Status::Success],
            false,
        ),
        3,
    );
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
}

#[tokio::test]
async fn repeat_zero_returns_success_without_ticking_child() {
    let (leaf, probe) = ScriptedLeaf::with_probe(0, vec![Status::Failure], true);
    let mut tree = Repeat::new(leaf, 0);
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![0]);
    let snapshot = ctx.snapshot_with_probes(&[probe]);
    assert_eq!(snapshot.resets, vec![1]);
}

#[tokio::test]
async fn timeout_fails_after_deadline_with_mock_clock() {
    let clock = MockClock::new();
    let mut tree = Timeout::new(
        ScriptedLeaf::new(0, vec![Status::Running]),
        clock.clone(),
        Duration::from_millis(100),
    );
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    clock.advance(Duration::from_millis(50));
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    clock.advance(Duration::from_millis(60));
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);

    assert_eq!(ctx.ticks, vec![2]);
}

#[tokio::test]
async fn timeout_fails_at_exact_deadline_boundary() {
    let clock = MockClock::new();
    let (leaf, probe) = ScriptedLeaf::with_probe(0, vec![Status::Running], true);
    let mut tree = Timeout::new(leaf, clock.clone(), Duration::from_millis(100));
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    clock.advance(Duration::from_millis(100));
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);

    assert_eq!(ctx.ticks, vec![1]);
    let snapshot = ctx.snapshot_with_probes(&[probe]);
    assert_eq!(snapshot.resets, vec![1]);
}

#[tokio::test]
async fn timeout_resets_child_after_terminal_statuses() {
    let clock = MockClock::new();
    let (leaf, probe) = ScriptedLeaf::with_probe(
        0,
        vec![Status::Running, Status::Success, Status::Failure],
        false,
    );
    let mut tree = Timeout::new(leaf, clock.clone(), Duration::from_millis(500));
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    clock.advance(Duration::from_millis(10));
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);

    let snapshot = ctx.snapshot_with_probes(&[probe]);
    assert_eq!(snapshot.resets, vec![2]);
}

#[tokio::test]
async fn force_success_and_force_failure_behave_as_defined() {
    let mut ctx = TickCtx::new(1);

    let mut force_success = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Failure]));
    let mut force_failure = ForceFailure::new(ScriptedLeaf::new(0, vec![Status::Success]));
    let mut running_passthrough = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Running]));

    assert_eq!(force_success.tick(&mut ctx).await, Status::Success);
    assert_eq!(force_failure.tick(&mut ctx).await, Status::Failure);
    assert_eq!(running_passthrough.tick(&mut ctx).await, Status::Running);
}
