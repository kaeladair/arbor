# Arbor Testing Guide

This document describes Arbor's testing taxonomy, local commands, and CI gates.

## Test taxonomy

### `arbor-core/tests/semantics.rs`

Semantic correctness split into focused modules:

- `semantics/sequence_selector.rs`
- `semantics/reactive.rs`
- `semantics/parallel.rs`
- `semantics/decorators.rs`
- `semantics/composition.rs`
- `semantics/edge_cases.rs`

These tests cover:

- short-circuit and memory/resume behavior
- reactive restart behavior
- parallel policies and panic contracts
- decorator transformations and counters
- timeout boundary behavior
- deep composition rules
- deterministic replay traces

### `arbor-core/tests/conformance_btcpp.rs`

Conformance tests ported from BehaviorTree.CPP semantics.

- Upstream pin: `3ff6a32ba0497a08519c77a1436e3b81eff1bcd6`
- Mapping manifest: `arbor-core/tests/conformance_manifest.md`

### `arbor-core/tests/property.rs`

Property-based invariants for core correctness, including:

- sequence with at least one failing leaf never succeeds
- selector with at least one succeeding leaf never fails
- short-circuit child tick-count invariants
- all-success structures succeed across randomized compositions

### `arbor-core/tests/differential.rs`

Bounded random tree generator + reference model interpreter.

- Generates trees over `Sequence`, `Selector`, `Reactive*`, `Parallel`, and decorator variants.
- Compares runtime vs model across multi-tick traces:
  - root statuses
  - per-leaf tick counts
  - per-leaf reset counts
  - per-leaf status traces

### `arbor/tests/async_runtime.rs`

Runtime integration checks:

- non-blocking tick loop behavior under Tokio scheduling
- long-run deterministic trace stability

### `arbor/tests/allocation_guard.rs`

Hot-path allocation guard:

- verifies zero allocations during repeated ticks after tree construction/warm-up

## Local commands

Run the full suite:

```bash
cargo test --workspace
```

Run semantic tests only:

```bash
cargo test -p arbor-core --test semantics
```

Run conformance tests only:

```bash
cargo test -p arbor-core --test conformance_btcpp
```

Run property tests only:

```bash
PROPTEST_CASES=128 cargo test -p arbor-core --test property -- --nocapture
```

Run differential tests only:

```bash
PROPTEST_CASES=128 cargo test -p arbor-core --test differential -- --nocapture
```

Run no-std compile check:

```bash
cargo check -p arbor-core --target thumbv7em-none-eabihf --no-default-features
```

Run coverage gate for `arbor-core`:

```bash
cargo +nightly llvm-cov --package arbor-core --tests --summary-only --branch --fail-under-lines 90
```

## CI gates

PR workflow (`.github/workflows/pr.yml`) is blocking and enforces:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p arbor-core --test conformance_btcpp`
- `cargo check -p arbor-core --target thumbv7em-none-eabihf --no-default-features`
- coverage threshold for `arbor-core`:
  - line >= 90%
  - branch >= 85%
  - branch threshold is enforced by parsing the `TOTAL` branch coverage column from `cargo +nightly llvm-cov --summary-only --branch`

Nightly workflow (`.github/workflows/nightly.yml`) runs heavier checks:

- high-case-count property tests (`PROPTEST_CASES=2000`)
- high-case-count differential tests (`PROPTEST_CASES=2000`)
- optional Miri smoke run

## Conformance scope notes

Conformance mirrors runtime semantics only. Out-of-scope items include:

- XML parsing/factory APIs
- logger integrations
- plugin loading/threading features specific to BehaviorTree.CPP internals
