//! Conformance suite ported from BehaviorTree.CPP semantics.
//! Upstream pin: 3ff6a32ba0497a08519c77a1436e3b81eff1bcd6

mod common;

use std::time::Duration;

use arbor_core::{
    ForceFailure, ForceSuccess, Inverter, Node, Parallel, ParallelPolicy, ReactiveSelector,
    ReactiveSequence, Retry, Selector, Sequence, Status, Timeout,
};

use common::{MockClock, ScriptedLeaf, TickCtx};

// Source: tests/gtest_sequence.cpp :: SimpleSequenceTest.ConditionTrue
#[tokio::test]
async fn btcpp_sequence_condition_true_equivalent() {
    let mut tree = Sequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(2);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1]);
}

// Source: tests/gtest_sequence.cpp :: SimpleSequenceWithMemoryTest.ConditionTurnToFalse
#[tokio::test]
async fn btcpp_sequence_with_memory_does_not_retick_previous_success_children() {
    let mut tree = Sequence::new((
        ScriptedLeaf::with_reset_behavior(0, vec![Status::Success, Status::Failure], false),
        ScriptedLeaf::with_reset_behavior(1, vec![Status::Running, Status::Success], false),
    ));
    let mut ctx = TickCtx::new(2);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);

    // Child 0 is not re-ticked while child 1 is running.
    assert_eq!(ctx.ticks, vec![1, 2]);
}

// Source: tests/gtest_fallback.cpp :: SimpleFallbackTest.ConditionChangeWhileRunning
#[tokio::test]
async fn btcpp_fallback_with_memory_resumes_running_branch() {
    let mut tree = Selector::new((
        ScriptedLeaf::with_reset_behavior(0, vec![Status::Failure, Status::Success], false),
        ScriptedLeaf::with_reset_behavior(1, vec![Status::Running, Status::Success], false),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);

    // Child 0 is not re-ticked while child 1 is running.
    assert_eq!(ctx.ticks, vec![1, 2, 0]);
}

// Source: tests/gtest_reactive.cpp :: ReactiveSequence_ReEvaluatesOnEveryTick
#[tokio::test]
async fn btcpp_reactive_sequence_rechecks_from_first_child_every_tick() {
    let mut tree = ReactiveSequence::new((
        ScriptedLeaf::new(0, vec![Status::Success]),
        ScriptedLeaf::new(1, vec![Status::Running]),
        ScriptedLeaf::new(2, vec![Status::Success]),
    ));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);

    assert_eq!(ctx.ticks, vec![2, 2, 0]);
}

// Source: tests/gtest_reactive.cpp :: ReactiveFallback_SecondChildSucceeds
#[tokio::test]
async fn btcpp_reactive_fallback_second_child_succeeds_equivalent() {
    let mut tree = ReactiveSelector::new((
        ScriptedLeaf::new(0, vec![Status::Failure]),
        ScriptedLeaf::new(1, vec![Status::Success]),
        ScriptedLeaf::new(2, vec![Status::Failure]),
    ));
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 0]);
}

// Source: tests/gtest_parallel.cpp :: SimpleParallelTest.Threshold_2
#[tokio::test]
async fn btcpp_parallel_threshold_two_equivalent() {
    let mut tree = Parallel::with_policy(
        (
            ScriptedLeaf::new(0, vec![Status::Success]),
            ScriptedLeaf::new(1, vec![Status::Failure]),
            ScriptedLeaf::new(2, vec![Status::Success]),
        ),
        ParallelPolicy::SuccessThreshold(2),
    );
    let mut ctx = TickCtx::new(3);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    assert_eq!(ctx.ticks, vec![1, 1, 1]);
}

// Source: tests/gtest_decorator.cpp :: Decorator.Inverter_ChildFailure
#[tokio::test]
async fn btcpp_decorator_inverter_child_failure_equivalent() {
    let mut tree = Inverter::new(ScriptedLeaf::new(0, vec![Status::Failure]));
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
}

// Source: tests/gtest_decorator.cpp :: Decorator.ForceSuccess_ChildFailure
#[tokio::test]
async fn btcpp_decorator_force_success_child_failure_equivalent() {
    let mut tree = ForceSuccess::new(ScriptedLeaf::new(0, vec![Status::Failure]));
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Success);
}

// Source: tests/gtest_decorator.cpp :: Decorator.ForceFailure_ChildSuccess
#[tokio::test]
async fn btcpp_decorator_force_failure_child_success_equivalent() {
    let mut tree = ForceFailure::new(ScriptedLeaf::new(0, vec![Status::Success]));
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
}

// Source: tests/gtest_decorator.cpp :: RetryTest.RetryTestA
#[tokio::test]
async fn btcpp_retry_until_limit_equivalent() {
    let mut tree = Retry::new(ScriptedLeaf::new(0, vec![Status::Failure]), 3);
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
}

// Source: tests/gtest_decorator.cpp :: DeadlineTest.DeadlineTriggeredTest
#[tokio::test]
async fn btcpp_timeout_deadline_triggered_equivalent() {
    let clock = MockClock::new();
    let mut tree = Timeout::new(
        ScriptedLeaf::new(0, vec![Status::Running]),
        clock.clone(),
        Duration::from_millis(100),
    );
    let mut ctx = TickCtx::new(1);

    assert_eq!(tree.tick(&mut ctx).await, Status::Running);
    clock.advance(Duration::from_millis(120));
    assert_eq!(tree.tick(&mut ctx).await, Status::Failure);
}
