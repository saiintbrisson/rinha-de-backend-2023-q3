use std::sync::atomic::{AtomicUsize, Ordering};

pub enum LoadBalancerStrategy {
    RoundRobin(RoundRobin),
    LeastConnection,
}

#[derive(Default)]
pub struct RoundRobin {
    current: AtomicUsize,
}

impl RoundRobin {
    pub fn next_wrapping(&self, ceil: usize) -> usize {
        self.current
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |f| Some((f + 1) % ceil))
            .expect("failed to update")
    }
}

#[derive(Default)]
pub struct LeastConnection {
    _f: (),
}
