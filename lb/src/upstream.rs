use std::{
    os::fd::RawFd,
    time::{Duration, Instant},
};

const KEEP_ALIVE_DURATION: Duration = Duration::from_millis(1000);

pub struct ConnectionSlotPool {
    available: crossbeam_queue::ArrayQueue<Available>,
    established: crossbeam_queue::ArrayQueue<EstablishedConnection>,
}

impl ConnectionSlotPool {
    pub fn foo(&self) -> Option<ConnectionSlot> {
        if let Some(conn) = self.established.pop() {
            if conn.established_at.elapsed() > KEEP_ALIVE_DURATION {
                return Some(ConnectionSlot::Reconnect(conn.socket_fd));
            }

            return Some(ConnectionSlot::EstablishedConnection(conn));
        }

        self.available.pop().map(ConnectionSlot::Available)
    }

    // fn provide()
}

pub struct Available;

pub struct EstablishedConnection {
    socket_fd: RawFd,
    established_at: Instant,
}

pub enum ConnectionSlot {
    Available(Available),
    EstablishedConnection(EstablishedConnection),
    Reconnect(RawFd),
}

// impl Drop for Available {
//     fn drop(&mut self) {
//         POOL.with(|c| {
//             if let Some(pool) = c.borrow().as_ref() {
//                 pool.available
//             }
//         })
//     }
// }

// impl Drop for EstablishedConnection {
//     fn drop(&mut self) {
//         POOL.with(|c| {
//             if let Some(pool) = c.borrow().as_ref() {
//                 pool.provide
//             }
//         })
//     }
// }
