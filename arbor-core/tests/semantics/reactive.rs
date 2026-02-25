use arbor_core::{Node, ReactiveSelector, ReactiveSequence, Status};

use crate::common::{ScriptedLeaf, TickCtx};

#[tokio::test]
async fn reactive_sequence_restarts_from_first_child_every_tick() {
    let mut tree = ReactiveSequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Running);
    assert_eq!(second, Status::Running);
    assert_eq!(ctx.ticks, vec![2, 2, 0]);
}

#[tokio::test]
async fn reactive_sequence_previously_succeeded_condition_can_fail() {
    let mut tree = ReactiveSequence::new((
        ScriptedLeaf::with_reset_behavior(0, vec![Status::Success, Status::Failure], false),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Running);
    assert_eq!(second, Status::Failure);
    assert_eq!(ctx.ticks, vec![2, 1, 0]);
}

#[tokio::test]
async fn reactive_selector_restarts_from_first_child_every_tick() {
    let mut tree = ReactiveSelector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Running);
    assert_eq!(second, Status::Running);
    assert_eq!(ctx.ticks, vec![2, 2, 0]);
}

#[tokio::test]
async fn reactive_selector_falls_through_when_previous_success_turns_failure() {
    let mut tree = ReactiveSelector::new((
        ScriptedLeaf::with_reset_behavior(0, vec![Status::Success, Status::Failure], false),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Success);
    assert_eq!(second, Status::Success);
    assert_eq!(ctx.ticks, vec![2, 1, 0]);
}

#[tokio::test]
async fn reactive_sequence_resets_later_children_when_running() {
    let (leaf0, _probe0) = ScriptedLeaf::with_probe(0, vec![Status::Success], true);
    let (leaf1, _probe1) = ScriptedLeaf::with_probe(1, vec![Status::Running], true);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Success], true);

    let mut tree = ReactiveSequence::new((leaf0, leaf1, leaf2));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    let snapshot = ctx.snapshot_with_probes(&[probe2]);
    assert_eq!(snapshot.resets, vec![2]);
    assert_eq!(ctx.ticks, vec![2, 2, 0]);
}

#[tokio::test]
async fn reactive_selector_resets_later_children_when_success() {
    let (leaf0, _probe0) = ScriptedLeaf::with_probe(0, vec![Status::Failure], true);
    let (leaf1, _probe1) = ScriptedLeaf::with_probe(1, vec![Status::Success], true);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Failure], true);

    let mut tree = ReactiveSelector::new((leaf0, leaf1, leaf2));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);

    let snapshot = ctx.snapshot_with_probes(&[probe2]);
    assert_eq!(snapshot.resets, vec![2]);
    assert_eq!(ctx.ticks, vec![2, 2, 0]);
}
