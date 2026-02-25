use std::{cell::Cell, rc::Rc};

use arbor_core::{
    Condition, Node, Parallel, ParallelPolicy, ReactiveSequence, Selector, Sequence, Status,
};

use crate::common::{ScriptedLeaf, TickCtx};

#[tokio::test]
async fn deeply_nested_tree_obeys_composition_rules() {
    let should_try_primary = Rc::new(Cell::new(true));
    let primary_check = should_try_primary.clone();

    let mut tree = Selector::new((
        Sequence::new((
            Condition::new(move |_: &TickCtx| primary_check.get()),
            ReactiveSequence::new((
                ScriptedLeaf::new(0, vec![Status::Success]),
                ScriptedLeaf::with_reset_behavior(1, vec![Status::Running, Status::Success], false),
            )),
        )),
        Sequence::new((
            ScriptedLeaf::new(2, vec![Status::Success]),
            ScriptedLeaf::new(3, vec![Status::Success]),
        )),
    ));

    let mut ctx = TickCtx::new(4);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    should_try_primary.set(false);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);

    assert_eq!(ctx.ticks, vec![2, 2, 0, 0]);
}

#[tokio::test]
async fn three_level_composition_short_circuit_and_parallel_policy() {
    let mut tree = Sequence::new((
        Selector::new((
            ScriptedLeaf::new(0, vec![Status::Failure]),
            ScriptedLeaf::new(1, vec![Status::Success]),
        )),
        Parallel::with_policy(
            (
                ScriptedLeaf::new(2, vec![Status::Success]),
                ScriptedLeaf::new(3, vec![Status::Running]),
                ScriptedLeaf::new(4, vec![Status::Success]),
            ),
            ParallelPolicy::SuccessThreshold(2),
        ),
    ));

    let mut ctx = TickCtx::new(5);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 1, 1, 1]);
}
