mod common;

use arbor_core::{
    ForceFailure, ForceSuccess, Inverter, Node, Parallel, ParallelPolicy, ReactiveSelector,
    ReactiveSequence, Repeat, Retry, Selector, Sequence, Status,
};
use proptest::prelude::*;
use std::{future::Future, pin::Pin};

use common::{LeafProbe, ScriptedLeaf, TickCtx, block_on};

#[derive(Clone, Copy, Debug)]
enum PolicyExpr {
    AllFailureOnAny,
    AnyFailureOnAll,
    Threshold(usize),
}

#[derive(Clone, Debug)]
enum Expr {
    Leaf {
        script: Vec<Status>,
        rewind_on_reset: bool,
    },
    Sequence(Box<Expr>, Box<Expr>),
    Selector(Box<Expr>, Box<Expr>),
    ReactiveSequence(Box<Expr>, Box<Expr>),
    ReactiveSelector(Box<Expr>, Box<Expr>),
    Parallel {
        policy: PolicyExpr,
        a: Box<Expr>,
        b: Box<Expr>,
        c: Box<Expr>,
    },
    Inverter(Box<Expr>),
    Retry {
        child: Box<Expr>,
        max_failures: usize,
    },
    Repeat {
        child: Box<Expr>,
        max_successes: usize,
    },
    ForceSuccess(Box<Expr>),
    ForceFailure(Box<Expr>),
}

fn leaf_expr_strategy() -> impl Strategy<Value = Expr> {
    (
        prop::collection::vec(
            prop_oneof![
                Just(Status::Success),
                Just(Status::Failure),
                Just(Status::Running),
            ],
            1..=4,
        ),
        any::<bool>(),
    )
        .prop_map(|(script, rewind_on_reset)| Expr::Leaf {
            script,
            rewind_on_reset,
        })
}

fn expr_strategy() -> impl Strategy<Value = Expr> {
    leaf_expr_strategy().prop_recursive(4, 64, 3, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone())
                .prop_map(|(a, b)| Expr::Sequence(Box::new(a), Box::new(b))),
            (inner.clone(), inner.clone())
                .prop_map(|(a, b)| Expr::Selector(Box::new(a), Box::new(b))),
            (inner.clone(), inner.clone())
                .prop_map(|(a, b)| Expr::ReactiveSequence(Box::new(a), Box::new(b))),
            (inner.clone(), inner.clone())
                .prop_map(|(a, b)| Expr::ReactiveSelector(Box::new(a), Box::new(b))),
            (
                prop_oneof![
                    Just(PolicyExpr::AllFailureOnAny),
                    Just(PolicyExpr::AnyFailureOnAll),
                    (1usize..=3).prop_map(PolicyExpr::Threshold),
                ],
                inner.clone(),
                inner.clone(),
                inner.clone()
            )
                .prop_map(|(policy, a, b, c)| Expr::Parallel {
                    policy,
                    a: Box::new(a),
                    b: Box::new(b),
                    c: Box::new(c)
                }),
            inner
                .clone()
                .prop_map(|child| Expr::Inverter(Box::new(child))),
            (inner.clone(), 0usize..=3).prop_map(|(child, max_failures)| Expr::Retry {
                child: Box::new(child),
                max_failures
            }),
            (inner.clone(), 0usize..=3).prop_map(|(child, max_successes)| Expr::Repeat {
                child: Box::new(child),
                max_successes
            }),
            inner
                .clone()
                .prop_map(|child| Expr::ForceSuccess(Box::new(child))),
            inner
                .clone()
                .prop_map(|child| Expr::ForceFailure(Box::new(child))),
        ]
    })
}

struct BoxRuntime(Box<RuntimeTree>);

impl BoxRuntime {
    fn new(node: RuntimeTree) -> Self {
        Self(Box::new(node))
    }
}

impl Node<TickCtx> for BoxRuntime {
    async fn tick(&mut self, ctx: &mut TickCtx) -> Status {
        self.0.tick_boxed(ctx).await
    }

    fn reset(&mut self) {
        self.0.reset();
    }
}

enum RuntimeTree {
    Leaf(ScriptedLeaf),
    Sequence(Sequence<(BoxRuntime, BoxRuntime)>),
    Selector(Selector<(BoxRuntime, BoxRuntime)>),
    ReactiveSequence(ReactiveSequence<(BoxRuntime, BoxRuntime)>),
    ReactiveSelector(ReactiveSelector<(BoxRuntime, BoxRuntime)>),
    Parallel(Parallel<(BoxRuntime, BoxRuntime, BoxRuntime)>),
    Inverter(Inverter<BoxRuntime>),
    Retry(Retry<BoxRuntime>),
    Repeat(Repeat<BoxRuntime>),
    ForceSuccess(ForceSuccess<BoxRuntime>),
    ForceFailure(ForceFailure<BoxRuntime>),
}

impl RuntimeTree {
    fn tick_boxed<'a>(
        &'a mut self,
        ctx: &'a mut TickCtx,
    ) -> Pin<Box<dyn Future<Output = Status> + 'a>> {
        Box::pin(async move {
            match self {
                RuntimeTree::Leaf(node) => node.tick(ctx).await,
                RuntimeTree::Sequence(node) => node.tick(ctx).await,
                RuntimeTree::Selector(node) => node.tick(ctx).await,
                RuntimeTree::ReactiveSequence(node) => node.tick(ctx).await,
                RuntimeTree::ReactiveSelector(node) => node.tick(ctx).await,
                RuntimeTree::Parallel(node) => node.tick(ctx).await,
                RuntimeTree::Inverter(node) => node.tick(ctx).await,
                RuntimeTree::Retry(node) => node.tick(ctx).await,
                RuntimeTree::Repeat(node) => node.tick(ctx).await,
                RuntimeTree::ForceSuccess(node) => node.tick(ctx).await,
                RuntimeTree::ForceFailure(node) => node.tick(ctx).await,
            }
        })
    }
}

impl Node<TickCtx> for RuntimeTree {
    async fn tick(&mut self, ctx: &mut TickCtx) -> Status {
        self.tick_boxed(ctx).await
    }

    fn reset(&mut self) {
        match self {
            RuntimeTree::Leaf(node) => node.reset(),
            RuntimeTree::Sequence(node) => node.reset(),
            RuntimeTree::Selector(node) => node.reset(),
            RuntimeTree::ReactiveSequence(node) => node.reset(),
            RuntimeTree::ReactiveSelector(node) => node.reset(),
            RuntimeTree::Parallel(node) => node.reset(),
            RuntimeTree::Inverter(node) => node.reset(),
            RuntimeTree::Retry(node) => node.reset(),
            RuntimeTree::Repeat(node) => node.reset(),
            RuntimeTree::ForceSuccess(node) => node.reset(),
            RuntimeTree::ForceFailure(node) => node.reset(),
        }
    }
}

fn to_parallel_policy(policy: PolicyExpr) -> ParallelPolicy {
    match policy {
        PolicyExpr::AllFailureOnAny => ParallelPolicy::SuccessOnAllFailureOnAny,
        PolicyExpr::AnyFailureOnAll => ParallelPolicy::SuccessOnAnyFailureOnAll,
        PolicyExpr::Threshold(m) => ParallelPolicy::SuccessThreshold(m),
    }
}

fn build_runtime(expr: &Expr, next_leaf: &mut usize, probes: &mut Vec<LeafProbe>) -> RuntimeTree {
    match expr {
        Expr::Leaf {
            script,
            rewind_on_reset,
        } => {
            let id = *next_leaf;
            *next_leaf += 1;
            let (leaf, probe) = ScriptedLeaf::with_probe(id, script.clone(), *rewind_on_reset);
            probes.push(probe);
            RuntimeTree::Leaf(leaf)
        }
        Expr::Sequence(a, b) => RuntimeTree::Sequence(Sequence::new((
            BoxRuntime::new(build_runtime(a, next_leaf, probes)),
            BoxRuntime::new(build_runtime(b, next_leaf, probes)),
        ))),
        Expr::Selector(a, b) => RuntimeTree::Selector(Selector::new((
            BoxRuntime::new(build_runtime(a, next_leaf, probes)),
            BoxRuntime::new(build_runtime(b, next_leaf, probes)),
        ))),
        Expr::ReactiveSequence(a, b) => RuntimeTree::ReactiveSequence(ReactiveSequence::new((
            BoxRuntime::new(build_runtime(a, next_leaf, probes)),
            BoxRuntime::new(build_runtime(b, next_leaf, probes)),
        ))),
        Expr::ReactiveSelector(a, b) => RuntimeTree::ReactiveSelector(ReactiveSelector::new((
            BoxRuntime::new(build_runtime(a, next_leaf, probes)),
            BoxRuntime::new(build_runtime(b, next_leaf, probes)),
        ))),
        Expr::Parallel { policy, a, b, c } => RuntimeTree::Parallel(Parallel::with_policy(
            (
                BoxRuntime::new(build_runtime(a, next_leaf, probes)),
                BoxRuntime::new(build_runtime(b, next_leaf, probes)),
                BoxRuntime::new(build_runtime(c, next_leaf, probes)),
            ),
            to_parallel_policy(*policy),
        )),
        Expr::Inverter(child) => RuntimeTree::Inverter(Inverter::new(BoxRuntime::new(
            build_runtime(child, next_leaf, probes),
        ))),
        Expr::Retry {
            child,
            max_failures,
        } => RuntimeTree::Retry(Retry::new(
            BoxRuntime::new(build_runtime(child, next_leaf, probes)),
            *max_failures,
        )),
        Expr::Repeat {
            child,
            max_successes,
        } => RuntimeTree::Repeat(Repeat::new(
            BoxRuntime::new(build_runtime(child, next_leaf, probes)),
            *max_successes,
        )),
        Expr::ForceSuccess(child) => RuntimeTree::ForceSuccess(ForceSuccess::new(BoxRuntime::new(
            build_runtime(child, next_leaf, probes),
        ))),
        Expr::ForceFailure(child) => RuntimeTree::ForceFailure(ForceFailure::new(BoxRuntime::new(
            build_runtime(child, next_leaf, probes),
        ))),
    }
}

#[derive(Debug, Clone)]
struct ModelCtx {
    ticks: Vec<usize>,
    resets: Vec<usize>,
    leaf_statuses: Vec<Vec<Status>>,
}

impl ModelCtx {
    fn new(leaves: usize) -> Self {
        Self {
            ticks: vec![0; leaves],
            resets: vec![0; leaves],
            leaf_statuses: vec![Vec::new(); leaves],
        }
    }
}

enum ModelTree {
    Leaf {
        id: usize,
        script: Vec<Status>,
        cursor: usize,
        rewind_on_reset: bool,
    },
    Sequence {
        left: Box<ModelTree>,
        right: Box<ModelTree>,
        running_index: usize,
    },
    Selector {
        left: Box<ModelTree>,
        right: Box<ModelTree>,
        running_index: usize,
    },
    ReactiveSequence {
        left: Box<ModelTree>,
        right: Box<ModelTree>,
    },
    ReactiveSelector {
        left: Box<ModelTree>,
        right: Box<ModelTree>,
    },
    Parallel {
        policy: PolicyExpr,
        a: Box<ModelTree>,
        b: Box<ModelTree>,
        c: Box<ModelTree>,
    },
    Inverter(Box<ModelTree>),
    Retry {
        child: Box<ModelTree>,
        max_failures: usize,
        failures: usize,
    },
    Repeat {
        child: Box<ModelTree>,
        max_successes: usize,
        successes: usize,
    },
    ForceSuccess(Box<ModelTree>),
    ForceFailure(Box<ModelTree>),
}

impl ModelTree {
    fn tick(&mut self, ctx: &mut ModelCtx) -> Status {
        match self {
            ModelTree::Leaf {
                id, script, cursor, ..
            } => {
                ctx.ticks[*id] += 1;
                let status = script
                    .get(*cursor)
                    .copied()
                    .or_else(|| script.last().copied())
                    .unwrap_or(Status::Failure);
                ctx.leaf_statuses[*id].push(status);
                if *cursor + 1 < script.len() {
                    *cursor += 1;
                }
                status
            }
            ModelTree::Sequence {
                left,
                right,
                running_index,
            } => {
                let mut index = *running_index;
                while index < 2 {
                    let status = if index == 0 {
                        left.tick(ctx)
                    } else {
                        right.tick(ctx)
                    };

                    match status {
                        Status::Success => index += 1,
                        Status::Failure => {
                            *running_index = 0;
                            left.reset(ctx);
                            right.reset(ctx);
                            return Status::Failure;
                        }
                        Status::Running => {
                            *running_index = index;
                            return Status::Running;
                        }
                    }
                }
                *running_index = 0;
                left.reset(ctx);
                right.reset(ctx);
                Status::Success
            }
            ModelTree::Selector {
                left,
                right,
                running_index,
            } => {
                let mut index = *running_index;
                while index < 2 {
                    let status = if index == 0 {
                        left.tick(ctx)
                    } else {
                        right.tick(ctx)
                    };

                    match status {
                        Status::Success => {
                            *running_index = 0;
                            left.reset(ctx);
                            right.reset(ctx);
                            return Status::Success;
                        }
                        Status::Failure => index += 1,
                        Status::Running => {
                            *running_index = index;
                            return Status::Running;
                        }
                    }
                }
                *running_index = 0;
                left.reset(ctx);
                right.reset(ctx);
                Status::Failure
            }
            ModelTree::ReactiveSequence { left, right } => match left.tick(ctx) {
                Status::Success => match right.tick(ctx) {
                    Status::Success => {
                        left.reset(ctx);
                        right.reset(ctx);
                        Status::Success
                    }
                    Status::Failure => Status::Failure,
                    Status::Running => Status::Running,
                },
                Status::Failure => {
                    right.reset(ctx);
                    Status::Failure
                }
                Status::Running => {
                    right.reset(ctx);
                    Status::Running
                }
            },
            ModelTree::ReactiveSelector { left, right } => match left.tick(ctx) {
                Status::Success => {
                    right.reset(ctx);
                    Status::Success
                }
                Status::Failure => match right.tick(ctx) {
                    Status::Success => Status::Success,
                    Status::Failure => {
                        left.reset(ctx);
                        right.reset(ctx);
                        Status::Failure
                    }
                    Status::Running => Status::Running,
                },
                Status::Running => {
                    right.reset(ctx);
                    Status::Running
                }
            },
            ModelTree::Parallel { policy, a, b, c } => {
                let mut successes = 0usize;
                let mut failures = 0usize;

                for status in [a.tick(ctx), b.tick(ctx), c.tick(ctx)] {
                    match status {
                        Status::Success => successes += 1,
                        Status::Failure => failures += 1,
                        Status::Running => {}
                    }
                }

                let n = 3usize;
                let m = match policy {
                    PolicyExpr::AllFailureOnAny => n,
                    PolicyExpr::AnyFailureOnAll => 1,
                    PolicyExpr::Threshold(m) => *m,
                };

                let status = if successes >= m {
                    Status::Success
                } else if failures > n - m {
                    Status::Failure
                } else {
                    Status::Running
                };

                if status != Status::Running {
                    a.reset(ctx);
                    b.reset(ctx);
                    c.reset(ctx);
                }

                status
            }
            ModelTree::Inverter(child) => match child.tick(ctx) {
                Status::Success => {
                    child.reset(ctx);
                    Status::Failure
                }
                Status::Failure => {
                    child.reset(ctx);
                    Status::Success
                }
                Status::Running => Status::Running,
            },
            ModelTree::Retry {
                child,
                max_failures,
                failures,
            } => {
                if *max_failures == 0 {
                    child.reset(ctx);
                    return Status::Failure;
                }

                match child.tick(ctx) {
                    Status::Success => {
                        *failures = 0;
                        child.reset(ctx);
                        Status::Success
                    }
                    Status::Running => Status::Running,
                    Status::Failure => {
                        *failures += 1;
                        child.reset(ctx);
                        if *failures >= *max_failures {
                            *failures = 0;
                            Status::Failure
                        } else {
                            Status::Running
                        }
                    }
                }
            }
            ModelTree::Repeat {
                child,
                max_successes,
                successes,
            } => {
                if *max_successes == 0 {
                    child.reset(ctx);
                    return Status::Success;
                }

                match child.tick(ctx) {
                    Status::Success => {
                        *successes += 1;
                        child.reset(ctx);
                        if *successes >= *max_successes {
                            *successes = 0;
                            Status::Success
                        } else {
                            Status::Running
                        }
                    }
                    Status::Running => Status::Running,
                    Status::Failure => {
                        *successes = 0;
                        child.reset(ctx);
                        Status::Failure
                    }
                }
            }
            ModelTree::ForceSuccess(child) => match child.tick(ctx) {
                Status::Running => Status::Running,
                Status::Success | Status::Failure => {
                    child.reset(ctx);
                    Status::Success
                }
            },
            ModelTree::ForceFailure(child) => match child.tick(ctx) {
                Status::Running => Status::Running,
                Status::Success | Status::Failure => {
                    child.reset(ctx);
                    Status::Failure
                }
            },
        }
    }

    fn reset(&mut self, ctx: &mut ModelCtx) {
        match self {
            ModelTree::Leaf {
                id,
                cursor,
                rewind_on_reset,
                ..
            } => {
                ctx.resets[*id] += 1;
                if *rewind_on_reset {
                    *cursor = 0;
                }
            }
            ModelTree::Sequence {
                left,
                right,
                running_index,
            }
            | ModelTree::Selector {
                left,
                right,
                running_index,
            } => {
                *running_index = 0;
                left.reset(ctx);
                right.reset(ctx);
            }
            ModelTree::ReactiveSequence { left, right }
            | ModelTree::ReactiveSelector { left, right } => {
                left.reset(ctx);
                right.reset(ctx);
            }
            ModelTree::Parallel { a, b, c, .. } => {
                a.reset(ctx);
                b.reset(ctx);
                c.reset(ctx);
            }
            ModelTree::Inverter(child)
            | ModelTree::ForceSuccess(child)
            | ModelTree::ForceFailure(child) => child.reset(ctx),
            ModelTree::Retry {
                child, failures, ..
            } => {
                *failures = 0;
                child.reset(ctx);
            }
            ModelTree::Repeat {
                child, successes, ..
            } => {
                *successes = 0;
                child.reset(ctx);
            }
        }
    }
}

fn build_model(expr: &Expr, next_leaf: &mut usize) -> ModelTree {
    match expr {
        Expr::Leaf {
            script,
            rewind_on_reset,
        } => {
            let id = *next_leaf;
            *next_leaf += 1;
            ModelTree::Leaf {
                id,
                script: script.clone(),
                cursor: 0,
                rewind_on_reset: *rewind_on_reset,
            }
        }
        Expr::Sequence(a, b) => ModelTree::Sequence {
            left: Box::new(build_model(a, next_leaf)),
            right: Box::new(build_model(b, next_leaf)),
            running_index: 0,
        },
        Expr::Selector(a, b) => ModelTree::Selector {
            left: Box::new(build_model(a, next_leaf)),
            right: Box::new(build_model(b, next_leaf)),
            running_index: 0,
        },
        Expr::ReactiveSequence(a, b) => ModelTree::ReactiveSequence {
            left: Box::new(build_model(a, next_leaf)),
            right: Box::new(build_model(b, next_leaf)),
        },
        Expr::ReactiveSelector(a, b) => ModelTree::ReactiveSelector {
            left: Box::new(build_model(a, next_leaf)),
            right: Box::new(build_model(b, next_leaf)),
        },
        Expr::Parallel { policy, a, b, c } => ModelTree::Parallel {
            policy: *policy,
            a: Box::new(build_model(a, next_leaf)),
            b: Box::new(build_model(b, next_leaf)),
            c: Box::new(build_model(c, next_leaf)),
        },
        Expr::Inverter(child) => ModelTree::Inverter(Box::new(build_model(child, next_leaf))),
        Expr::Retry {
            child,
            max_failures,
        } => ModelTree::Retry {
            child: Box::new(build_model(child, next_leaf)),
            max_failures: *max_failures,
            failures: 0,
        },
        Expr::Repeat {
            child,
            max_successes,
        } => ModelTree::Repeat {
            child: Box::new(build_model(child, next_leaf)),
            max_successes: *max_successes,
            successes: 0,
        },
        Expr::ForceSuccess(child) => {
            ModelTree::ForceSuccess(Box::new(build_model(child, next_leaf)))
        }
        Expr::ForceFailure(child) => {
            ModelTree::ForceFailure(Box::new(build_model(child, next_leaf)))
        }
    }
}

fn build_runtime_and_model(expr: &Expr) -> (RuntimeTree, Vec<LeafProbe>, ModelTree, usize) {
    let mut runtime_next = 0usize;
    let mut probes = Vec::new();
    let runtime = build_runtime(expr, &mut runtime_next, &mut probes);

    let mut model_next = 0usize;
    let model = build_model(expr, &mut model_next);

    assert_eq!(runtime_next, model_next);

    (runtime, probes, model, runtime_next)
}

proptest! {
    #[test]
    fn runtime_matches_reference_model_over_multitick_traces(
        expr in expr_strategy(),
        cycles in 1usize..12,
    ) {
        let (mut runtime, probes, mut model, leaves) = build_runtime_and_model(&expr);

        let mut runtime_ctx = TickCtx::new(leaves);
        let mut model_ctx = ModelCtx::new(leaves);
        let mut model_root_statuses = Vec::with_capacity(cycles);

        for _ in 0..cycles {
            let runtime_status = block_on(async { runtime.tick(&mut runtime_ctx).await });
            runtime_ctx.record_root(runtime_status);

            let model_status = model.tick(&mut model_ctx);
            model_root_statuses.push(model_status);

            prop_assert_eq!(runtime_status, model_status);
        }

        let runtime_snapshot = runtime_ctx.snapshot_with_probes(&probes);

        prop_assert_eq!(runtime_snapshot.ticks, model_ctx.ticks);
        prop_assert_eq!(runtime_snapshot.resets, model_ctx.resets);
        prop_assert_eq!(runtime_snapshot.leaf_statuses, model_ctx.leaf_statuses);
        prop_assert_eq!(runtime_snapshot.root_statuses, model_root_statuses);
    }
}

#[test]
fn differential_smoke_deterministic() {
    let expr = Expr::Sequence(
        Box::new(Expr::Leaf {
            script: vec![Status::Success],
            rewind_on_reset: true,
        }),
        Box::new(Expr::Retry {
            child: Box::new(Expr::Leaf {
                script: vec![Status::Failure, Status::Success],
                rewind_on_reset: false,
            }),
            max_failures: 2,
        }),
    );

    let (mut runtime, probes, mut model, leaves) = build_runtime_and_model(&expr);
    let mut runtime_ctx = TickCtx::new(leaves);
    let mut model_ctx = ModelCtx::new(leaves);

    for _ in 0..6 {
        let runtime_status = block_on(async { runtime.tick(&mut runtime_ctx).await });
        runtime_ctx.record_root(runtime_status);
        let model_status = model.tick(&mut model_ctx);
        assert_eq!(runtime_status, model_status);
    }

    let runtime_snapshot = runtime_ctx.snapshot_with_probes(&probes);
    assert_eq!(runtime_snapshot.ticks, model_ctx.ticks);
    assert_eq!(runtime_snapshot.resets, model_ctx.resets);
    assert_eq!(runtime_snapshot.leaf_statuses, model_ctx.leaf_statuses);
}
