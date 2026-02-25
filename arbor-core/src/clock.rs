use core::time::Duration;

pub trait Clock {
    type Instant: Copy;

    fn now(&self) -> Self::Instant;
    fn elapsed(&self, since: Self::Instant) -> Duration;
}
