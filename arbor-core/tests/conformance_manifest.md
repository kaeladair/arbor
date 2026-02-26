# BehaviorTree.CPP Conformance Manifest

- Upstream repository: https://github.com/BehaviorTree/BehaviorTree.CPP
- Pinned commit: `3ff6a32ba0497a08519c77a1436e3b81eff1bcd6`
- Arbor conformance test file: `arbor-core/tests/conformance_btcpp.rs`

## Mapped tests

| Arbor test | Upstream source |
| --- | --- |
| `btcpp_sequence_condition_true_equivalent` | `tests/gtest_sequence.cpp` `SimpleSequenceTest.ConditionTrue` |
| `btcpp_sequence_with_memory_does_not_retick_previous_success_children` | `tests/gtest_sequence.cpp` `SimpleSequenceWithMemoryTest.ConditionTurnToFalse` |
| `btcpp_fallback_with_memory_resumes_running_branch` | `tests/gtest_fallback.cpp` `SimpleFallbackTest.ConditionChangeWhileRunning` |
| `btcpp_reactive_sequence_rechecks_from_first_child_every_tick` | `tests/gtest_reactive.cpp` `ReactiveSequence_ReEvaluatesOnEveryTick` |
| `btcpp_reactive_fallback_second_child_succeeds_equivalent` | `tests/gtest_reactive.cpp` `ReactiveFallback_SecondChildSucceeds` |
| `btcpp_parallel_threshold_two_equivalent` | `tests/gtest_parallel.cpp` `SimpleParallelTest.Threshold_2` |
| `btcpp_decorator_inverter_child_failure_equivalent` | `tests/gtest_decorator.cpp` `Decorator.Inverter_ChildFailure` |
| `btcpp_decorator_force_success_child_failure_equivalent` | `tests/gtest_decorator.cpp` `Decorator.ForceSuccess_ChildFailure` |
| `btcpp_decorator_force_failure_child_success_equivalent` | `tests/gtest_decorator.cpp` `Decorator.ForceFailure_ChildSuccess` |
| `btcpp_retry_until_limit_equivalent` | `tests/gtest_decorator.cpp` `RetryTest.RetryTestA` |
| `btcpp_timeout_deadline_triggered_equivalent` | `tests/gtest_decorator.cpp` `DeadlineTest.DeadlineTriggeredTest` |

## Intentional omissions

- XML parser / factory registration / ports / blackboard APIs (outside Arbor runtime scope).
- Logger and transport integrations (outside Arbor runtime scope).
- Threading, plugin loading, and coroutine implementation details specific to BehaviorTree.CPP internals.
- Features that rely on node lifecycle states not exposed by Arbor.
