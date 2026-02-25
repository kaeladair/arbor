use std::time::Duration;

use arbor::{Condition, Node, Selector, Sequence, Status};

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct Target {
    x: f32,
    y: f32,
}

#[derive(Debug)]
struct DroneCtx {
    battery_pct: f32,
    position: Position,
    target: Option<Target>,
    comms_healthy: bool,
}

#[derive(Debug, Default)]
struct FlyToTarget {
    in_progress: bool,
}

impl Node<DroneCtx> for FlyToTarget {
    async fn tick(&mut self, ctx: &mut DroneCtx) -> Status {
        if !self.in_progress {
            let Some(target) = ctx.target else {
                println!("[action] no target available");
                return Status::Failure;
            };

            self.in_progress = true;
            println!(
                "[action] command: fly to ({:.1}, {:.1})",
                target.x, target.y
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
            return Status::Running;
        }

        let Some(target) = ctx.target else {
            self.in_progress = false;
            return Status::Failure;
        };

        ctx.position = Position {
            x: target.x,
            y: target.y,
        };
        self.in_progress = false;
        println!("[action] target reached");
        Status::Success
    }

    fn reset(&mut self) {
        self.in_progress = false;
    }
}

#[derive(Debug, Default)]
struct ReturnToLaunch {
    in_progress: bool,
}

impl Node<DroneCtx> for ReturnToLaunch {
    async fn tick(&mut self, ctx: &mut DroneCtx) -> Status {
        if !self.in_progress {
            self.in_progress = true;
            println!("[action] command: return to launch");
            tokio::time::sleep(Duration::from_millis(200)).await;
            return Status::Running;
        }

        ctx.position = Position { x: 0.0, y: 0.0 };
        self.in_progress = false;
        println!("[action] landed at launch position");
        Status::Success
    }

    fn reset(&mut self) {
        self.in_progress = false;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut ctx = DroneCtx {
        battery_pct: 65.0,
        position: Position { x: 0.0, y: 0.0 },
        target: Some(Target { x: 10.0, y: 4.0 }),
        comms_healthy: true,
    };

    let mut tree = Selector::new((
        Sequence::new((
            Condition::new(|ctx: &DroneCtx| ctx.battery_pct > 20.0),
            Condition::new(|ctx: &DroneCtx| ctx.comms_healthy),
            FlyToTarget::default(),
        )),
        ReturnToLaunch::default(),
    ));

    let final_status = loop {
        let status = tree.tick(&mut ctx).await;
        println!(
            "tick status: {status:?}, position=({:.1}, {:.1})",
            ctx.position.x, ctx.position.y
        );
        if status != Status::Running {
            break status;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    println!("mission complete with status: {final_status:?}");
}
