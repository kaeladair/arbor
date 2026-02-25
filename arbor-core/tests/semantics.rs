mod common;

use std::{cell::Cell, rc::Rc, time::Duration};

use arbor_core::{
    Condition, ForceFailure, ForceSuccess, Inverter, Node, Parallel, ParallelPolicy,
    ReactiveSelector, ReactiveSequence, Repeat, Retry, Selector, Sequence, Status, Timeout,
};

use common::{MockClock, ScriptedLeaf, TickCtx};

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
async fn force_success_and_force_failure_behave_as_defined() {
    let mut ctx = TickCtx::new(1);

    let mut force_success = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Failure]));
    let mut force_failure = ForceFailure::new(ScriptedLeaf::new(0, vec![Status::Success]));
    let mut running_passthrough = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Running]));

    assert_eq!(force_success.tick(&mut ctx).await, Status::Success);
    assert_eq!(force_failure.tick(&mut ctx).await, Status::Failure);
    assert_eq!(running_passthrough.tick(&mut ctx).await, Status::Running);
}

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
