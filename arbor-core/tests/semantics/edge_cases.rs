use arbor_core::{Node, Selector, Sequence, Status};

use crate::common::{LeafProbe, ScriptedLeaf, TickCtx, block_on};

fn build_determinism_tree() -> (impl Node<TickCtx>, Vec<LeafProbe>) {
    let (leaf0, probe0) = ScriptedLeaf::with_probe(0, vec![Status::Success], true);
    let (leaf1, probe1) = ScriptedLeaf::with_probe(1, vec![Status::Running, Status::Success], true);
    let (leaf2, probe2) = ScriptedLeaf::with_probe(2, vec![Status::Success], true);
    let (leaf3, probe3) = ScriptedLeaf::with_probe(3, vec![Status::Success], true);

    let tree = Selector::new((Sequence::new((leaf0, leaf1, leaf2)), leaf3));
    (tree, vec![probe0, probe1, probe2, probe3])
}

fn run_trace(cycles: usize) -> (Vec<Status>, crate::common::TraceSnapshot) {
    let (mut tree, probes) = build_determinism_tree();
    let mut ctx = TickCtx::new(4);

    let mut statuses = Vec::with_capacity(cycles);
    for _ in 0..cycles {
        let status = block_on(async { tree.tick(&mut ctx).await });
        ctx.record_root(status);
        statuses.push(status);
    }

    (statuses, ctx.snapshot_with_probes(&probes))
}

#[test]
fn determinism_replay_produces_identical_statuses_and_leaf_traces() {
    let (statuses_a, trace_a) = run_trace(10);
    let (statuses_b, trace_b) = run_trace(10);

    assert_eq!(statuses_a, statuses_b);
    assert_eq!(trace_a, trace_b);
}

#[test]
fn determinism_trace_has_expected_running_success_pattern() {
    let (statuses, trace) = run_trace(6);

    assert_eq!(
        statuses,
        vec![
            Status::Running,
            Status::Success,
            Status::Running,
            Status::Success,
            Status::Running,
            Status::Success,
        ]
    );
    assert_eq!(trace.root_statuses, statuses);
}
