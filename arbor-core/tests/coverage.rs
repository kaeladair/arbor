mod common;

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

use arbor_core::{
    Action, Condition, Constant, ForceFailure, ForceSuccess, Inverter, Node, NodeList, Parallel,
    ParallelPolicy, ReactiveSelector, ReactiveSequence, Repeat, Retry, Selector, Sequence, Status,
    Timeout,
};

use common::{MockClock, ScriptedLeaf, TickCtx, block_on};

#[tokio::test]
async fn leaf_constructor_and_into_inner_paths_are_covered() {
    let mut ctx_value = 0u32;

    let mut action = Action::new(|ctx: &mut u32| {
        *ctx += 1;
        core::future::ready(Status::Success)
    });
    assert_eq!(action.tick(&mut ctx_value).await, Status::Success);
    let action_inner = Action::new(|ctx: &mut u32| {
        *ctx += 1;
        core::future::ready(Status::Success)
    })
    .into_inner();
    assert_eq!(action_inner(&mut ctx_value).await, Status::Success);

    let mut condition = Condition::new(|ctx: &u32| *ctx > 0);
    assert_eq!(condition.tick(&mut ctx_value).await, Status::Success);
    let condition_inner = Condition::new(|ctx: &u32| *ctx > 10).into_inner();
    assert!(!condition_inner(&ctx_value));

    let mut constant = Constant::new(Status::Failure);
    assert_eq!(constant.tick(&mut ctx_value).await, Status::Failure);
}

#[tokio::test]
async fn composite_accessor_policy_and_reset_paths_are_covered() {
    let mut seq = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(2);
    assert_eq!(seq.tick(&mut ctx).await, Status::Running);
    seq.reset();

    let mut sel = Selector::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    ));
    assert_eq!(sel.tick(&mut ctx).await, Status::Running);
    sel.reset();

    let mut rseq = ReactiveSequence::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    ));
    assert_eq!(rseq.tick(&mut ctx).await, Status::Running);
    rseq.reset();

    let mut rsel = ReactiveSelector::new((
        ScriptedLeaf::new(0, vec![Status::Running]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    ));
    assert_eq!(rsel.tick(&mut ctx).await, Status::Running);
    rsel.reset();

    let mut par = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Running]),
            ScriptedLeaf::new(1, vec![Status::Running]),
            ScriptedLeaf::new(2, vec![Status::Running]),
        ),
        ParallelPolicy::SuccessOnAnyFailureOnAll,
    );
    assert_eq!(par.policy(), ParallelPolicy::SuccessOnAnyFailureOnAll);
    assert_eq!(par.tick(&mut TickCtx::new(3)).await, Status::Running);
    par.reset();

    let _ = Sequence::new((ScriptedLeaf::new(0, vec![Status::Success]),)).into_children();
    let _ = Selector::new((ScriptedLeaf::new(0, vec![Status::Success]),)).into_children();
    let _ = ReactiveSequence::new((ScriptedLeaf::new(0, vec![Status::Success]),)).into_children();
    let _ = ReactiveSelector::new((ScriptedLeaf::new(0, vec![Status::Success]),)).into_children();
    let _ = Parallel::new((ScriptedLeaf::new(0, vec![Status::Success]),)).into_children();
}

#[tokio::test]
async fn decorator_accessor_and_reset_paths_are_covered() {
    let mut ctx = TickCtx::new(1);

    let mut inverter = Inverter::new(ScriptedLeaf::new(0, vec![Status::Running]));
    assert_eq!(inverter.tick(&mut ctx).await, Status::Running);
    inverter.reset();
    let _ = Inverter::new(ScriptedLeaf::new(0, vec![Status::Success])).into_child();

    let mut retry = Retry::new(ScriptedLeaf::new(0, vec![Status::Running]), 2);
    assert_eq!(retry.tick(&mut ctx).await, Status::Running);
    retry.reset();
    let _ = Retry::new(ScriptedLeaf::new(0, vec![Status::Success]), 1).into_child();

    let mut repeat = Repeat::new(ScriptedLeaf::new(0, vec![Status::Running]), 2);
    assert_eq!(repeat.tick(&mut ctx).await, Status::Running);
    repeat.reset();
    let _ = Repeat::new(ScriptedLeaf::new(0, vec![Status::Success]), 1).into_child();

    let clock = MockClock::new();
    let mut timeout = Timeout::new(
        ScriptedLeaf::new(0, vec![Status::Running]),
        clock.clone(),
        Duration::from_millis(10),
    );
    assert_eq!(timeout.tick(&mut ctx).await, Status::Running);
    timeout.reset();
    let _ = Timeout::new(
        ScriptedLeaf::new(0, vec![Status::Success]),
        clock,
        Duration::from_millis(1),
    )
    .into_child();

    let mut force_success = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Running]));
    assert_eq!(force_success.tick(&mut ctx).await, Status::Running);
    force_success.reset();
    let _ = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Success])).into_child();

    let mut force_failure = ForceFailure::new(ScriptedLeaf::new(0, vec![Status::Running]));
    assert_eq!(force_failure.tick(&mut ctx).await, Status::Running);
    force_failure.reset();
    let _ = ForceFailure::new(ScriptedLeaf::new(0, vec![Status::Success])).into_child();
}

#[tokio::test]
async fn array_nodelist_paths_are_covered() {
    let mut tree = Sequence::new([
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ]);
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

#[test]
fn nodelist_out_of_bounds_panics_are_covered() {
    let mut array = [ScriptedLeaf::new(0, vec![Status::Success])];
    let mut tuple = (ScriptedLeaf::new(0, vec![Status::Success]),);
    let mut ctx = TickCtx::new(1);

    let array_result = catch_unwind(AssertUnwindSafe(|| {
        block_on(async {
            <[ScriptedLeaf; 1] as NodeList<TickCtx>>::tick_at(&mut array, 1, &mut ctx).await
        })
    }));
    assert!(array_result.is_err());

    let tuple_result = catch_unwind(AssertUnwindSafe(|| {
        block_on(async {
            <(ScriptedLeaf,) as NodeList<TickCtx>>::tick_at(&mut tuple, 1, &mut ctx).await
        })
    }));
    assert!(tuple_result.is_err());
}

macro_rules! tuple_sequence_success_test {
    ($name:ident, $len:expr, $tuple:expr) => {
        #[tokio::test]
        async fn $name() {
            let mut tree = Sequence::new($tuple);
            let mut ctx = TickCtx::new($len);
            assert_eq!(tree.tick(&mut ctx).await, Status::Success);
            assert_eq!(ctx.ticks, vec![1; $len]);
        }
    };
}

tuple_sequence_success_test!(
    tuple_arity_1_covered,
    1,
    (ScriptedLeaf::new(0, vec![Status::Success]),)
);
tuple_sequence_success_test!(
    tuple_arity_2_covered,
    2,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_3_covered,
    3,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_4_covered,
    4,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_5_covered,
    5,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_6_covered,
    6,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_7_covered,
    7,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_8_covered,
    8,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
        ScriptedLeaf::new(7, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_9_covered,
    9,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
        ScriptedLeaf::new(7, vec![Status::Success]),
        ScriptedLeaf::new(8, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_10_covered,
    10,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
        ScriptedLeaf::new(7, vec![Status::Success]),
        ScriptedLeaf::new(8, vec![Status::Success]),
        ScriptedLeaf::new(9, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_11_covered,
    11,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
        ScriptedLeaf::new(7, vec![Status::Success]),
        ScriptedLeaf::new(8, vec![Status::Success]),
        ScriptedLeaf::new(9, vec![Status::Success]),
        ScriptedLeaf::new(10, vec![Status::Success]),
    )
);
tuple_sequence_success_test!(
    tuple_arity_12_covered,
    12,
    (
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Success]),
        ScriptedLeaf::new(3, vec![Status::Success]),
        ScriptedLeaf::new(4, vec![Status::Success]),
        ScriptedLeaf::new(5, vec![Status::Success]),
        ScriptedLeaf::new(6, vec![Status::Success]),
        ScriptedLeaf::new(7, vec![Status::Success]),
        ScriptedLeaf::new(8, vec![Status::Success]),
        ScriptedLeaf::new(9, vec![Status::Success]),
        ScriptedLeaf::new(10, vec![Status::Success]),
        ScriptedLeaf::new(11, vec![Status::Success]),
    )
);
