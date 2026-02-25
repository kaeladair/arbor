mod common;

use arbor_core::{
    Node, Parallel, ParallelPolicy, ReactiveSelector, ReactiveSequence, Selector, Sequence, Status,
};
use proptest::prelude::*;

use common::{ScriptedLeaf, TickCtx};

fn block_on<F>(future: F) -> F::Output
where
    F: core::future::Future,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime must build");
    runtime.block_on(future)
}

fn status_from_bool(value: bool) -> Status {
    if value {
        Status::Success
    } else {
        Status::Failure
    }
}

proptest! {
    #[test]
    fn sequence_with_at_least_one_failure_never_returns_success(
        a in any::<bool>(),
        b in any::<bool>(),
        c in any::<bool>(),
        d in any::<bool>(),
    ) {
        prop_assume!(!(a && b && c && d));

        let mut tree = Sequence::new((
            ScriptedLeaf::new(0, vec![status_from_bool(a)]),
            ScriptedLeaf::new(1, vec![status_from_bool(b)]),
            ScriptedLeaf::new(2, vec![status_from_bool(c)]),
            ScriptedLeaf::new(3, vec![status_from_bool(d)]),
        ));
        let mut ctx = TickCtx::new(4);

        let status = block_on(async { tree.tick(&mut ctx).await });

        prop_assert_ne!(status, Status::Success);
    }

    #[test]
    fn selector_with_at_least_one_success_never_returns_failure(
        a in any::<bool>(),
        b in any::<bool>(),
        c in any::<bool>(),
        d in any::<bool>(),
    ) {
        prop_assume!(a || b || c || d);

        let mut tree = Selector::new((
            ScriptedLeaf::new(0, vec![status_from_bool(a)]),
            ScriptedLeaf::new(1, vec![status_from_bool(b)]),
            ScriptedLeaf::new(2, vec![status_from_bool(c)]),
            ScriptedLeaf::new(3, vec![status_from_bool(d)]),
        ));
        let mut ctx = TickCtx::new(4);

        let status = block_on(async { tree.tick(&mut ctx).await });

        prop_assert_ne!(status, Status::Failure);
    }

    #[test]
    fn sequence_short_circuits_tick_counts_after_first_failure(
        fail_index in 0usize..4,
        trailing_a in any::<bool>(),
        trailing_b in any::<bool>(),
        trailing_c in any::<bool>(),
    ) {
        let mut leaves = [
            status_from_bool(trailing_a),
            status_from_bool(trailing_b),
            status_from_bool(trailing_c),
            Status::Success,
        ];

        for index in 0..4 {
            if index < fail_index {
                leaves[index] = Status::Success;
            } else if index == fail_index {
                leaves[index] = Status::Failure;
            }
        }

        let mut tree = Sequence::new((
            ScriptedLeaf::new(0, vec![leaves[0]]),
            ScriptedLeaf::new(1, vec![leaves[1]]),
            ScriptedLeaf::new(2, vec![leaves[2]]),
            ScriptedLeaf::new(3, vec![leaves[3]]),
        ));
        let mut ctx = TickCtx::new(4);

        let status = block_on(async { tree.tick(&mut ctx).await });

        prop_assert_eq!(status, Status::Failure);

        for index in 0..4 {
            let expected = if index <= fail_index { 1 } else { 0 };
            prop_assert_eq!(ctx.ticks[index], expected);
        }
    }

    #[test]
    fn all_success_leaves_return_success_across_random_structures(
        structure_id in 0u8..6,
    ) {
        let status = match structure_id {
            0 => {
                let mut tree = Sequence::new((
                    ScriptedLeaf::new(0, vec![Status::Success]),
                    ScriptedLeaf::new(1, vec![Status::Success]),
                    ScriptedLeaf::new(2, vec![Status::Success]),
                ));
                let mut ctx = TickCtx::new(3);
                block_on(async { tree.tick(&mut ctx).await })
            }
            1 => {
                let mut tree = Selector::new((
                    ScriptedLeaf::new(0, vec![Status::Success]),
                    ScriptedLeaf::new(1, vec![Status::Success]),
                    ScriptedLeaf::new(2, vec![Status::Success]),
                ));
                let mut ctx = TickCtx::new(3);
                block_on(async { tree.tick(&mut ctx).await })
            }
            2 => {
                let mut tree = Sequence::new((
                    Selector::new((
                        ScriptedLeaf::new(0, vec![Status::Success]),
                        ScriptedLeaf::new(1, vec![Status::Success]),
                    )),
                    ReactiveSequence::new((
                        ScriptedLeaf::new(2, vec![Status::Success]),
                        ScriptedLeaf::new(3, vec![Status::Success]),
                    )),
                ));
                let mut ctx = TickCtx::new(4);
                block_on(async { tree.tick(&mut ctx).await })
            }
            3 => {
                let mut tree = Selector::new((
                    Sequence::new((
                        ScriptedLeaf::new(0, vec![Status::Success]),
                        ScriptedLeaf::new(1, vec![Status::Success]),
                    )),
                    ReactiveSelector::new((
                        ScriptedLeaf::new(2, vec![Status::Success]),
                        ScriptedLeaf::new(3, vec![Status::Success]),
                        ScriptedLeaf::new(4, vec![Status::Success]),
                    )),
                ));
                let mut ctx = TickCtx::new(5);
                block_on(async { tree.tick(&mut ctx).await })
            }
            4 => {
                let mut tree = Parallel::with_policy(
                    (
                        ScriptedLeaf::new(0, vec![Status::Success]),
                        ScriptedLeaf::new(1, vec![Status::Success]),
                        ScriptedLeaf::new(2, vec![Status::Success]),
                    ),
                    ParallelPolicy::SuccessOnAllFailureOnAny,
                );
                let mut ctx = TickCtx::new(3);
                block_on(async { tree.tick(&mut ctx).await })
            }
            _ => {
                let mut tree = Parallel::with_policy(
                    (
                        ScriptedLeaf::new(0, vec![Status::Success]),
                        ScriptedLeaf::new(1, vec![Status::Success]),
                        ScriptedLeaf::new(2, vec![Status::Success]),
                    ),
                    ParallelPolicy::SuccessThreshold(2),
                );
                let mut ctx = TickCtx::new(3);
                block_on(async { tree.tick(&mut ctx).await })
            }
        };

        prop_assert_eq!(status, Status::Success);
    }
}
