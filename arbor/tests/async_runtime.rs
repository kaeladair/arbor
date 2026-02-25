use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use arbor::{Action, Node, Selector, Sequence, Status, tick_until_done};

#[derive(Debug, Default)]
struct RuntimeCtx {
    remaining_cycles: usize,
}

#[tokio::test(start_paused = true, flavor = "current_thread")]
async fn tick_until_done_allows_other_tasks_to_progress() {
    let beats = Arc::new(AtomicUsize::new(0));
    let beats_task = beats.clone();
    let (first_beat_tx, first_beat_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut first_tx = Some(first_beat_tx);
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            beats_task.fetch_add(1, Ordering::Relaxed);
            if let Some(tx) = first_tx.take() {
                let _ = tx.send(());
            }
        }
    });

    let mut tree = Action::new(|ctx: &mut RuntimeCtx| {
        let running = ctx.remaining_cycles > 0;
        if running {
            ctx.remaining_cycles -= 1;
        }

        async move {
            if running {
                tokio::time::sleep(Duration::from_millis(5)).await;
                Status::Running
            } else {
                Status::Success
            }
        }
    });

    let mut ctx = RuntimeCtx {
        remaining_cycles: 8,
    };

    let done = tick_until_done(&mut tree, &mut ctx, Duration::from_millis(2));
    tokio::pin!(done);

    tokio::select! {
        _ = &mut done => panic!("tree completed before heartbeat could run"),
        _ = first_beat_rx => {}
    }

    let status = done.await;
    assert_eq!(status, Status::Success);
    assert!(beats.load(Ordering::Relaxed) > 0);
}

#[derive(Debug)]
struct ScriptNode {
    script: &'static [Status],
    cursor: usize,
}

impl ScriptNode {
    const fn new(script: &'static [Status]) -> Self {
        Self { script, cursor: 0 }
    }
}

impl Node<()> for ScriptNode {
    async fn tick(&mut self, _ctx: &mut ()) -> Status {
        let status = self
            .script
            .get(self.cursor)
            .copied()
            .or_else(|| self.script.last().copied())
            .unwrap_or(Status::Failure);

        if self.cursor + 1 < self.script.len() {
            self.cursor += 1;
        }

        status
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }
}

fn build_stability_tree() -> impl Node<()> {
    Selector::new((
        Sequence::new((
            ScriptNode::new(&[Status::Success]),
            ScriptNode::new(&[Status::Running, Status::Success]),
        )),
        ScriptNode::new(&[Status::Success]),
    ))
}

async fn collect_status_trace(cycles: usize) -> Vec<Status> {
    let mut tree = build_stability_tree();
    let mut ctx = ();
    let mut trace = Vec::with_capacity(cycles);

    for _ in 0..cycles {
        trace.push(tree.tick(&mut ctx).await);
    }

    trace
}

#[tokio::test(flavor = "current_thread")]
async fn repeated_status_traces_are_stable() {
    let trace_a = collect_status_trace(20).await;
    let trace_b = collect_status_trace(20).await;

    assert_eq!(trace_a, trace_b);
}
