use std::os::fd::RawFd;

use socket2::SockAddr;

pub struct Bridge {
    pub id: usize,
    downstream: RawFd,
    pub target: SockAddr,
    pub stage: BridgeLifecycle,
    pub closed_n: usize,
}

impl Bridge {
    pub fn new(id: usize, downstream: RawFd, target: SockAddr) -> Self {
        Self {
            id,
            downstream,
            target,
            stage: BridgeLifecycle::Accepted,
            closed_n: 0,
        }
    }

    pub fn get_fd(&self, direction: Direction) -> RawFd {
        match (direction, &self.stage) {
            (Direction::Downstream, _) => self.downstream,
            (Direction::Upstream, BridgeLifecycle::Established { upstream }) => *upstream,
            _ => panic!("target not established yet"),
        }
    }

    pub fn upgrade(&mut self, upstream: RawFd) {
        self.stage = BridgeLifecycle::Established { upstream }
    }

    pub fn close_one(&mut self) {
        self.closed_n += 1;
    }
}

pub enum BridgeLifecycle {
    Accepted,
    Established { upstream: RawFd },
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    /// The client
    Downstream,
    /// The server
    Upstream,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Downstream => Direction::Upstream,
            Direction::Upstream => Direction::Downstream,
        }
    }
}
