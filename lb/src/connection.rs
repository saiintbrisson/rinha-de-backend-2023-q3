use std::os::fd::RawFd;

pub struct Connection {
    downstream: RawFd,
    upstream: RawFd,
}
