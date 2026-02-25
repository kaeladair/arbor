use crate::{Node, NodeList, Status};

pub struct Sequence<Children> {
    children: Children,
    running_index: usize,
}

impl<Children> Sequence<Children> {
    pub const fn new(children: Children) -> Self {
        Self {
            children,
            running_index: 0,
        }
    }

    pub fn into_children(self) -> Children {
        self.children
    }
}

impl<Ctx, Children> Node<Ctx> for Sequence<Children>
where
    Children: NodeList<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        let mut index = self.running_index;

        while index < Children::LEN {
            match self.children.tick_at(index, ctx).await {
                Status::Success => {
                    index += 1;
                }
                Status::Failure => {
                    self.running_index = 0;
                    self.children.reset_all();
                    return Status::Failure;
                }
                Status::Running => {
                    self.running_index = index;
                    return Status::Running;
                }
            }
        }

        self.running_index = 0;
        self.children.reset_all();
        Status::Success
    }

    fn reset(&mut self) {
        self.running_index = 0;
        self.children.reset_all();
    }
}

pub struct Selector<Children> {
    children: Children,
    running_index: usize,
}

impl<Children> Selector<Children> {
    pub const fn new(children: Children) -> Self {
        Self {
            children,
            running_index: 0,
        }
    }

    pub fn into_children(self) -> Children {
        self.children
    }
}

impl<Ctx, Children> Node<Ctx> for Selector<Children>
where
    Children: NodeList<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        let mut index = self.running_index;

        while index < Children::LEN {
            match self.children.tick_at(index, ctx).await {
                Status::Success => {
                    self.running_index = 0;
                    self.children.reset_all();
                    return Status::Success;
                }
                Status::Failure => {
                    index += 1;
                }
                Status::Running => {
                    self.running_index = index;
                    return Status::Running;
                }
            }
        }

        self.running_index = 0;
        self.children.reset_all();
        Status::Failure
    }

    fn reset(&mut self) {
        self.running_index = 0;
        self.children.reset_all();
    }
}

pub struct ReactiveSequence<Children> {
    children: Children,
}

impl<Children> ReactiveSequence<Children> {
    pub const fn new(children: Children) -> Self {
        Self { children }
    }

    pub fn into_children(self) -> Children {
        self.children
    }
}

impl<Ctx, Children> Node<Ctx> for ReactiveSequence<Children>
where
    Children: NodeList<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        let mut index = 0;

        while index < Children::LEN {
            match self.children.tick_at(index, ctx).await {
                Status::Success => {
                    index += 1;
                }
                Status::Failure => {
                    self.children.reset_range(index + 1);
                    return Status::Failure;
                }
                Status::Running => {
                    self.children.reset_range(index + 1);
                    return Status::Running;
                }
            }
        }

        self.children.reset_all();
        Status::Success
    }

    fn reset(&mut self) {
        self.children.reset_all();
    }
}

pub struct ReactiveSelector<Children> {
    children: Children,
}

impl<Children> ReactiveSelector<Children> {
    pub const fn new(children: Children) -> Self {
        Self { children }
    }

    pub fn into_children(self) -> Children {
        self.children
    }
}

impl<Ctx, Children> Node<Ctx> for ReactiveSelector<Children>
where
    Children: NodeList<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        let mut index = 0;

        while index < Children::LEN {
            match self.children.tick_at(index, ctx).await {
                Status::Success => {
                    self.children.reset_range(index + 1);
                    return Status::Success;
                }
                Status::Failure => {
                    index += 1;
                }
                Status::Running => {
                    self.children.reset_range(index + 1);
                    return Status::Running;
                }
            }
        }

        self.children.reset_all();
        Status::Failure
    }

    fn reset(&mut self) {
        self.children.reset_all();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelPolicy {
    SuccessOnAllFailureOnAny,
    SuccessOnAnyFailureOnAll,
    SuccessThreshold(usize),
}

impl Default for ParallelPolicy {
    fn default() -> Self {
        Self::SuccessOnAllFailureOnAny
    }
}

pub struct Parallel<Children> {
    children: Children,
    policy: ParallelPolicy,
}

impl<Children> Parallel<Children> {
    pub const fn new(children: Children) -> Self {
        Self {
            children,
            policy: ParallelPolicy::SuccessOnAllFailureOnAny,
        }
    }

    pub const fn with_policy(children: Children, policy: ParallelPolicy) -> Self {
        Self { children, policy }
    }

    pub fn into_children(self) -> Children {
        self.children
    }

    pub const fn policy(&self) -> ParallelPolicy {
        self.policy
    }
}

impl<Ctx, Children> Node<Ctx> for Parallel<Children>
where
    Children: NodeList<Ctx>,
{
    async fn tick(&mut self, ctx: &mut Ctx) -> Status {
        let n = Children::LEN;
        if n == 0 {
            panic!("parallel nodes require at least one child");
        }

        let m = match self.policy {
            ParallelPolicy::SuccessOnAllFailureOnAny => n,
            ParallelPolicy::SuccessOnAnyFailureOnAll => 1,
            ParallelPolicy::SuccessThreshold(threshold) => {
                if threshold == 0 || threshold > n {
                    panic!(
                        "invalid success threshold {threshold} for parallel node with {n} children"
                    );
                }
                threshold
            }
        };

        let mut successes = 0usize;
        let mut failures = 0usize;

        for index in 0..n {
            match self.children.tick_at(index, ctx).await {
                Status::Success => successes += 1,
                Status::Failure => failures += 1,
                Status::Running => {}
            }
        }

        let status = if successes >= m {
            Status::Success
        } else if failures > n - m {
            Status::Failure
        } else {
            Status::Running
        };

        if status != Status::Running {
            self.children.reset_all();
        }

        status
    }

    fn reset(&mut self) {
        self.children.reset_all();
    }
}
