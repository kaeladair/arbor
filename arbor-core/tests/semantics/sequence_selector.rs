use arbor_core::{Node, Selector, Sequence, Status};

use crate::common::{ScriptedLeaf, TickCtx};

#[tokio::test]
async fn sequence_all_children_success_returns_success() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

#[tokio::test]
async fn sequence_first_child_failure_short_circuits() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Failure);
    assert_eq!(ctx.ticks, vec![1, 0, 0]);
}

#[tokio::test]
async fn sequence_middle_running_short_circuits() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Running);
    assert_eq!(ctx.ticks, vec![1, 1, 0]);
}

#[tokio::test]
async fn sequence_with_memory_resumes_running_child() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::with_reset_behavior(1, vec![Status::Running, Status::Success], false),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Running);
    assert_eq!(second, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 2, 1]);
}

#[tokio::test]
async fn selector_all_children_fail_returns_failure() {
    let mut tree = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Failure]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Failure);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

#[tokio::test]
async fn selector_first_child_success_short_circuits() {
    let mut tree = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Failure]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 0, 0]);
}

#[tokio::test]
async fn selector_middle_running_short_circuits() {
    let mut tree = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let status = tree.tick(&mut ctx).await;

    assert_eq!(status, Status::Running);
    assert_eq!(ctx.ticks, vec![1, 1, 0]);
}

#[tokio::test]
async fn selector_with_memory_resumes_running_child() {
    let mut tree = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::with_reset_behavior(1, vec![Status::Running, Status::Success], false),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    let first = tree.tick(&mut ctx).await;
    let second = tree.tick(&mut ctx).await;

    assert_eq!(first, Status::Running);
    assert_eq!(second, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 2, 0]);
}

#[tokio::test]
async fn sequence_exact_tick_count_on_short_circuit() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Failure]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(4);

    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
    assert_eq!(ctx.ticks, vec![1, 1, 0, 0]);
}

#[tokio::test]
async fn selector_exact_tick_count_on_short_circuit() {
    let mut tree = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
        ScriptedLeaf::new(3, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(4);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 0, 0]);
}

#[tokio::test]
async fn sequence_terminal_success_resets_all_children_once() {
    let (leaf0, probe0) = ScriptedLeaf::with_probe(0, vec![Status::Success], true);
    let (leaf1, probe1) =
        ScriptedLeaf::with_probe(1, vec![Status::Running, Status::Success], false);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Success], true);

    let mut tree = Sequence::new((leaf0, leaf1, leaf2));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    let first = ctx.snapshot_with_probes(&[probe0.clone(), probe1.clone(), probe2.clone()]);
    assert_eq!(first.resets, vec![0, 0, 0]);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    let second = ctx.snapshot_with_probes(&[probe0, probe1, probe2]);
    assert_eq!(second.resets, vec![1, 1, 1]);
    assert_eq!(second.ticks, vec![1, 2, 1]);
}

#[tokio::test]
async fn selector_terminal_success_resets_all_children_once() {
    let (leaf0, probe0) = ScriptedLeaf::with_probe(0, vec![Status::Failure], true);
    let (leaf1, probe1) =
        ScriptedLeaf::with_probe(1, vec![Status::Running, Status::Success], false);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Failure], true);

    let mut tree = Selector::new((leaf0, leaf1, leaf2));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    let first = ctx.snapshot_with_probes(&[probe0.clone(), probe1.clone(), probe2.clone()]);
    assert_eq!(first.resets, vec![0, 0, 0]);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    let second = ctx.snapshot_with_probes(&[probe0, probe1, probe2]);
    assert_eq!(second.resets, vec![1, 1, 1]);
    assert_eq!(second.ticks, vec![1, 2, 0]);
}

#[tokio::test]
async fn sequence_terminal_failure_resets_all_children_once() {
    let (leaf0, probe0) = ScriptedLeaf::with_probe(0, vec![Status::Failure], true);
    let (leaf1, probe1) = ScriptedLeaf::with_probe(1, vec![Status::Success], true);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Success], true);

    let mut tree = Sequence::new((leaf0, leaf1, leaf2));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
    let snapshot = ctx.snapshot_with_probes(&[probe0, probe1, probe2]);

    assert_eq!(snapshot.ticks, vec![1, 0, 0]);
    assert_eq!(snapshot.resets, vec![1, 1, 1]);
}
