use std::{fmt::Debug, os::fd::RawFd, ptr::null_mut};

use io_uring::{opcode, squeue, types::Fd};
use libc::{sockaddr, socklen_t};
use socket2::{Domain, Protocol, SockAddr, Type};

use crate::server::{bridge::Direction, buffer::Buffers};

type BridgeId = usize;

pub enum Op {
    Accept(Accept),
    Socket(Socket),
    Connect(Connect),
    Read(Read),
    Write(Write),
    Close(Close),
    ProvideBuffers(ProvideBuffers),
}

impl Op {
    pub fn entry(&self) -> squeue::Entry {
        match &self {
            Op::Accept(op) => op.entry(),
            Op::Socket(op) => op.entry(),
            Op::Connect(op) => op.entry(),
            Op::Read(op) => op.entry(),
            Op::Write(op) => op.entry(),
            Op::Close(op) => op.entry(),
            Op::ProvideBuffers(op) => op.entry(),
        }
    }
}

impl Debug for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accept(_) => f.debug_tuple("Accept").finish(),
            Self::Socket(_) => f.debug_tuple("Socket").finish(),
            Self::Connect(_) => f.debug_tuple("Connect").finish(),
            Self::Read(_) => f.debug_tuple("Read").finish(),
            Self::Write(_) => f.debug_tuple("Write").finish(),
            Self::Close(_) => f.debug_tuple("Close").finish(),
            Self::ProvideBuffers(_) => f.debug_tuple("ProvideBuffers").finish(),
        }
    }
}

pub struct Accept {
    fd: RawFd,
}

impl Accept {
    pub fn new(fd: RawFd) -> Self {
        Self { fd }
    }

    pub fn entry(&self) -> squeue::Entry {
        opcode::AcceptMulti::new(Fd(self.fd)).build()
    }
}

pub struct Socket {
    pub bridge_id: BridgeId,
    domain: Domain,
}

impl Socket {
    pub fn new(bridge_id: BridgeId, domain: Domain) -> Self {
        Self { bridge_id, domain }
    }

    fn entry(&self) -> squeue::Entry {
        const TY: Type = Type::STREAM;
        const PROTOCOL: Protocol = Protocol::TCP;

        opcode::Socket::new(self.domain.into(), TY.into(), PROTOCOL.into()).build()
    }
}

pub struct Connect {
    pub bridge_id: BridgeId,
    fd: RawFd,
    addr_ptr: *const sockaddr,
    addr_len: socklen_t,
}

impl Connect {
    pub fn new(bridge_id: BridgeId, fd: RawFd, addr: &SockAddr) -> Self {
        Self {
            bridge_id,
            fd,
            addr_ptr: addr.as_ptr(),
            addr_len: addr.len(),
        }
    }

    fn entry(&self) -> squeue::Entry {
        io_uring::opcode::Connect::new(Fd(self.fd), self.addr_ptr, self.addr_len).build()
    }
}

pub struct Read {
    pub bridge_id: BridgeId,
    pub origin: Direction,
    pub fd: RawFd,
    group: u16,
}

impl Read {
    pub fn new(bridge_id: BridgeId, origin: Direction, fd: RawFd, group: u16) -> Self {
        Self {
            bridge_id,
            origin,
            fd,
            group,
        }
    }

    fn entry(&self) -> squeue::Entry {
        io_uring::opcode::Read::new(Fd(self.fd), null_mut(), 0)
            .buf_group(self.group)
            .build()
            .flags(io_uring::squeue::Flags::BUFFER_SELECT)
    }
}

#[derive(Clone)]
pub struct Write {
    pub bridge_id: BridgeId,
    pub destination: Direction,
    fd: RawFd,
    pub associated_buffer_segment: u16,
    pub buf: *const u8,
    pub buf_len: u32,
}

impl Write {
    pub fn new(
        bridge_id: BridgeId,
        destination: Direction,
        fd: RawFd,
        associated_buffer_segment: u16,
        buf: &[u8],
    ) -> Self {
        Self {
            bridge_id,
            destination,
            fd,
            associated_buffer_segment,
            buf: buf.as_ptr(),
            buf_len: buf.len() as u32,
        }
    }

    fn entry(&self) -> squeue::Entry {
        io_uring::opcode::Write::new(Fd(self.fd), self.buf, self.buf_len).build()
    }
}

pub struct Close {
    pub bridge_id: BridgeId,
    pub direction: Direction,
    fd: RawFd,
}

impl Close {
    pub fn new(bridge_id: BridgeId, direction: Direction, fd: RawFd) -> Self {
        Self {
            bridge_id,
            direction,
            fd,
        }
    }

    fn entry(&self) -> squeue::Entry {
        io_uring::opcode::Close::new(Fd(self.fd)).build()
    }
}

pub struct ProvideBuffers {
    buffer_base: *mut u8,
    buffer_segment_count: u16,
    buffer_segment_len: i32,
    buffer_group_id: u16,
    buffer_segment_idx: u16,
}

impl ProvideBuffers {
    pub fn new(
        buffer_base: *mut u8,
        buffer_segment_count: u16,
        buffer_segment_len: usize,
        buffer_group_id: u16,
        buffer_segment_idx: u16,
    ) -> Self {
        Self {
            buffer_base,
            buffer_segment_count,
            buffer_segment_len: buffer_segment_len as i32,
            buffer_group_id,
            buffer_segment_idx,
        }
    }

    pub fn from_buffers(buffers: &Buffers) -> Self {
        Self::new(
            buffers.buffer_base,
            buffers.buffer_segment_count,
            buffers.buffer_segment_len,
            0,
            0,
        )
    }

    fn entry(&self) -> squeue::Entry {
        io_uring::opcode::ProvideBuffers::new(
            self.buffer_base,
            self.buffer_segment_len,
            self.buffer_segment_count,
            self.buffer_group_id,
            self.buffer_segment_idx,
        )
        .build()
    }
}
