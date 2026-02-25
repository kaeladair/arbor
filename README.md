# arbor

A minimal, deterministic, async-native behavior tree runtime in Rust, split into:

- `arbor-core`: `no_std` core semantics and node implementations.
- `arbor`: tokio-facing crate that re-exports core and adds runtime helpers.

The implementation follows *Behavior Trees in Robotics and AI* (M. Colledanchise, P. Ogren) as the semantic source of truth.

## Crates

- `/arbor-core`
  - `Status` (`Success`, `Failure`, `Running`)
  - Typed `Node<Ctx>` trait (`Ctx` is the user blackboard/context)
  - Leaf, composite, and decorator nodes
  - `Clock` abstraction for timeout semantics
  - No allocations in the tick hot path
- `/arbor`
  - Re-exports all core types
  - `TokioClock`
  - `tick_until_done(...)`
  - `examples/drone_mission.rs`

## Quick Start

```rust
use std::time::Duration;
use arbor::{Condition, Selector, Sequence, Status, tick_until_done};

#[derive(Default)]
struct Ctx {
    battery_pct: f32,
    comms_healthy: bool,
}

#[derive(Default)]
struct DoWork;

impl arbor::Node<Ctx> for DoWork {
    async fn tick(&mut self, _ctx: &mut Ctx) -> Status {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Status::Success
    }
}

# #[tokio::main(flavor = "current_thread")]
# async fn main() {
let mut tree = Selector::new((
    Sequence::new((
        Condition::new(|ctx: &Ctx| ctx.battery_pct > 20.0),
        Condition::new(|ctx: &Ctx| ctx.comms_healthy),
        DoWork::default(),
    )),
    DoWork::default(),
));

let mut ctx = Ctx { battery_pct: 80.0, comms_healthy: true };
let status = tick_until_done(&mut tree, &mut ctx, Duration::from_millis(20)).await;
assert_eq!(status, Status::Success);
# }
```

## Semantics Summary

- `Sequence` and `Selector` are memory control-flow nodes.
  - They resume from the last `Running` child.
  - Memory is cleared on terminal return (`Success`/`Failure`).
- `ReactiveSequence` and `ReactiveSelector` are memory-less control-flow nodes.
  - They restart from child index `0` each tick.
- `Parallel` ticks all children every tick and applies policy thresholds.
- `Condition` never returns `Running`.
- `Action` may return all three statuses.
- Decorators preserve/transform child status exactly according to their rule.

## Book Mapping Table

| Book definition | Crate type | Notes |
|---|---|---|
| Ch. 1.3, Algorithm 1 (Sequence) | `Sequence` | Left-to-right, stop on first `Failure`/`Running`, succeed only if all succeed |
| Ch. 1.3, Algorithm 2 (Fallback/Selector) | `Selector` | Left-to-right, stop on first `Success`/`Running`, fail only if all fail |
| Ch. 1.3, Algorithm 3 (Parallel with threshold `M`) | `Parallel` + `ParallelPolicy` | `Success` if successes `>= M`; `Failure` if failures `> N-M`; otherwise `Running` |
| Ch. 1.3, Action definition | `Action` or custom `Node` leaf | Async execution allowed; may return `Running`/`Success`/`Failure` |
| Ch. 1.3, Condition definition | `Condition` | Returns `Success`/`Failure` only |
| Ch. 1.3, decorator examples (invert, max-N-tries, max-Tsec) | `Inverter`, `Retry`, `Timeout` | Direct decorator mappings |
| Ch. 1.3.2, control-flow nodes with memory | `Sequence`, `Selector` | Resume from running child and clear memory on terminal return |
| Ch. 1.3 + Ch. 3.6, memory-less reactive ticking behavior | `ReactiveSequence`, `ReactiveSelector` | Re-evaluate from first child every tick |
| Ch. 1.3 decorator status forcing patterns | `ForceSuccess`, `ForceFailure`, `Repeat` | Status override and bounded repetition policies |

## Tests

`arbor-core/tests` includes:

- Formal correctness tests for each required node type and semantics rule.
- Tick-count assertions for short-circuit and memory/resume behavior.
- Property-based tests (proptest) over randomized status configurations and random structure templates.
- Nested composition tests (3+ levels).
- Timeout tests using a mock clock.

Run all checks:

```bash
cargo fmt --all
cargo test --workspace
```

## Example

Run the drone simulation:

```bash
cargo run -p arbor --example drone_mission
```
