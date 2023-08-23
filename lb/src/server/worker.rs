use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{Error, ErrorKind, Result},
    os::fd::AsRawFd,
};

use io_uring::{cqueue, squeue, IoUring};
use slab::Slab;
use socket2::SockAddr;

use crate::{
    op::{Accept, Close, Connect, Op, ProvideBuffers, Read, Socket, Write},
    server::{bridge::Direction, Server},
    upstream::ConnectionSlotPool,
};

use super::{bridge::Bridge, buffer::Buffers};

thread_local! {
    static POOL: RefCell<Option<ConnectionSlotPool>> = RefCell::new(None);
}

pub struct ServerWorker {
    server: Server,

    ring: IoUring,
    buffers: Buffers,

    slab: Slab<Op>,
    pre_submission: VecDeque<squeue::Entry>,
    bridges: Vec<Bridge>,
}

impl ServerWorker {
    pub fn new(server: Server, ring: IoUring, cap: usize) -> Self {
        Self {
            server,

            ring,
            buffers: Buffers::new(1000, 4096),

            slab: Slab::with_capacity(cap),
            pre_submission: VecDeque::with_capacity(cap),
            bridges: Vec::with_capacity(cap),
        }
    }

    pub fn init(&mut self) {
        let listener =
            std::net::TcpListener::bind(&self.server.bind_address).expect("failed to bind address");
        let accept = Accept::new(listener.as_raw_fd());
        let provide_buffers = ProvideBuffers::from_buffers(&self.buffers);
        self.bar(Op::Accept(accept));
        self.bar(Op::ProvideBuffers(provide_buffers));
        std::mem::forget(listener);
    }

    fn bar(&mut self, o: Op) {
        let entry = self.slab.vacant_entry();
        let key = entry.key();
        self.pre_submission
            .push_back(o.entry().user_data(key as u64));
        entry.insert(o);
    }

    pub fn foo(&mut self) {
        loop {
            let mut submission = self.ring.submission();
            unsafe {
                let (front, back) = self.pre_submission.as_slices();
                submission.push_multiple(&front).unwrap();
                submission.push_multiple(&back).unwrap();
                submission.sync();
            }
            drop(submission);
            self.pre_submission.clear();

            self.ring.submit_and_wait(1).unwrap();
            let mut completion = self.ring.completion();
            let mut handler = Handler {
                server: &self.server,
                buffers: &self.buffers,
                ops: &mut self.slab,
                pre_submission: &mut self.pre_submission,
                bridges: &mut self.bridges,
            };

            for entry in &mut completion {
                handler.handle(entry);
            }

            completion.sync();
        }
    }
}

struct Handler<'a> {
    server: &'a Server,

    buffers: &'a Buffers,

    ops: &'a mut Slab<Op>,
    pre_submission: &'a mut VecDeque<squeue::Entry>,
    // bridges: &'a mut Slab<Bridge>,
    bridges: &'a mut Vec<Bridge>,
}

impl<'a> Handler<'a> {
    fn queue(&mut self, o: Op) {
        let entry = self.ops.vacant_entry();
        let key = entry.key();
        self.pre_submission
            .push_back(o.entry().user_data(key as u64));
        entry.insert(o);
    }

    fn handle(&mut self, entry: cqueue::Entry) {
        let key = entry.user_data() as usize;

        let op;
        let op = if cqueue::more(entry.flags()) {
            self.ops.get(key).unwrap()
        } else {
            op = self.ops.remove(key);
            &op
        };

        let bridge_id = match &op {
            Op::Accept(_) => {
                self.handle_accept(&entry).unwrap();
                return;
            }
            Op::Close(close) => {
                let Some(bridge) = self.bridges.get_mut(close.bridge_id) else {
                    return;
                };

                bridge.close_one();
                if bridge.closed_n == 2 {
                    // self.bridges.remove(close.bridge_id);
                }

                return;
            }
            Op::ProvideBuffers(_) => return,

            Op::Socket(socket) => socket.bridge_id,
            Op::Connect(connect) => connect.bridge_id,
            Op::Read(read) => read.bridge_id,
            Op::Write(write) => write.bridge_id,
        };

        let mut bridges = std::mem::take(self.bridges);
        let Some(bridge) = bridges.get_mut(bridge_id) else {
            *self.bridges = bridges;
            return;
        };

        let res = match &op {
            Op::Socket(_) => self.handle_socket(&entry, bridge),
            Op::Connect(_) => self.handle_connect(&entry, bridge),
            Op::Read(read) => self.handle_read(&entry, bridge, read.origin),
            Op::Write(write) => self.handle_write(&entry, write.clone()),
            _ => Ok(()),
        };

        if let Some(bridge) = res.err().map(|_| bridge) {
            let downstream = bridge.get_fd(Direction::Downstream);
            let upstream = bridge.get_fd(Direction::Upstream);

            self.queue(Op::Close(Close::new(
                bridge.id,
                Direction::Downstream,
                downstream,
            )));
            self.queue(Op::Close(Close::new(
                bridge.id,
                Direction::Upstream,
                upstream,
            )));
        }

        *self.bridges = bridges;
    }
}

impl Handler<'_> {
    fn handle_accept(&mut self, entry: &cqueue::Entry) -> Result<()> {
        if entry.result() < 0 {
            let res = match entry.result().abs() {
                err @ (libc::ECONNABORTED | libc::EPERM | libc::EINTR | libc::EPROTO) => {
                    panic!("should continue {err:?}");
                }

                err @ (libc::ENOTSOCK
                | libc::EBADF
                | libc::EFAULT
                | libc::EINVAL
                | libc::EMFILE
                | libc::ENFILE
                | libc::EOPNOTSUPP) => {
                    let err = Error::from_raw_os_error(err);
                    panic!("should abort {err:?}");
                }

                err => {
                    let err = Error::from_raw_os_error(err);
                    panic!("tf {err}");
                }
            };

            return Err(res);
        }

        if !cqueue::more(entry.flags()) {
            panic!("something went terribly wrong")
        }

        let downstream = entry.result();
        let target = self.server.next_target().unwrap();
        let target = SockAddr::from(target);
        let domain = target.domain();

        let id = self.bridges.len();
        self.bridges.push(Bridge::new(id, downstream, target));
        // let entry = self.bridges.vacant_entry();
        // let id = entry.key();
        // let bridge = Bridge::new(id, downstream, target);
        // entry.insert(bridge);

        self.queue(Op::Socket(Socket::new(id, domain)));

        Ok(())
    }

    fn handle_socket(&mut self, entry: &cqueue::Entry, bridge: &mut Bridge) -> Result<()> {
        if entry.result() < 0 {
            return Err(Error::from_raw_os_error(entry.result().abs()));
        }

        let connect = Connect::new(bridge.id, entry.result(), &bridge.target);
        bridge.upgrade(entry.result());

        self.queue(Op::Connect(connect));
        Ok(())
    }

    fn handle_connect(&mut self, entry: &cqueue::Entry, bridge: &mut Bridge) -> Result<()> {
        if entry.result() < 0 {
            return Err(Error::from_raw_os_error(entry.result().abs()));
        }

        let read_downstream = Read::new(
            bridge.id,
            Direction::Downstream,
            bridge.get_fd(Direction::Downstream),
            0,
        );
        let read_upstream = Read::new(
            bridge.id,
            Direction::Upstream,
            bridge.get_fd(Direction::Upstream),
            0,
        );

        self.queue(Op::Read(read_downstream));
        self.queue(Op::Read(read_upstream));

        Ok(())
    }

    fn handle_read(
        &mut self,
        entry: &cqueue::Entry,
        bridge: &mut Bridge,
        origin: Direction,
    ) -> Result<()> {
        let res = entry.result();
        if res <= 0 {
            let idx = cqueue::buffer_select(entry.flags()).unwrap();
            let buf = self.buffers.get_segment(idx);
            let provide_buffers = ProvideBuffers::new(buf.as_ptr() as *mut _, 1, buf.len(), 0, idx);
            self.queue(Op::ProvideBuffers(provide_buffers));

            if res < 0 {
                return Err(Error::from_raw_os_error(entry.result().abs()));
            } else if res == 0 {
                return Err(ErrorKind::UnexpectedEof.into());
            }
        }

        let idx = cqueue::buffer_select(entry.flags()).unwrap();
        let buf = &self.buffers.get_segment(idx)[..entry.result() as usize];

        let read = Read::new(bridge.id, origin, bridge.get_fd(origin), 0);
        let destination = origin.opposite();
        let write = Write::new(bridge.id, destination, bridge.get_fd(destination), idx, buf);

        self.queue(Op::Read(read));
        self.queue(Op::Write(write));

        Ok(())
    }

    fn handle_write(&mut self, entry: &cqueue::Entry, mut write: Write) -> Result<()> {
        let res = entry.result();
        if res <= 0 {
            let buf = self.buffers.get_segment(write.associated_buffer_segment);
            let provide_buffers = ProvideBuffers::new(
                buf.as_ptr() as *mut _,
                1,
                buf.len(),
                0,
                write.associated_buffer_segment,
            );

            self.queue(Op::ProvideBuffers(provide_buffers));

            if res < 0 {
                return Err(Error::from_raw_os_error(entry.result().abs()));
            } else if res == 0 {
                return Err(ErrorKind::BrokenPipe.into());
            }
        }

        let res = res as u32;
        if res < write.buf_len {
            write.buf_len = write.buf_len - res;
            write.buf = write.buf.wrapping_add(res as usize);
            self.queue(Op::Write(write));
            return Ok(());
        }

        let buf = self.buffers.get_segment(write.associated_buffer_segment);
        let provide_buffers = ProvideBuffers::new(
            buf.as_ptr() as *mut _,
            1,
            buf.len(),
            0,
            write.associated_buffer_segment,
        );

        self.queue(Op::ProvideBuffers(provide_buffers));

        Ok(())
    }
}

#[derive(Clone, Copy)]
#[repr(packed)]
struct ConnInfo {
    bridge_id: u32,
    op_idx: u32,
}

impl ConnInfo {
    fn new(bridge_id: u32, op_idx: u32) -> Self {
        Self { bridge_id, op_idx }
    }

    fn with_op(self, op_idx: u32) -> Self {
        Self {
            bridge_id: self.bridge_id,
            op_idx,
        }
    }
}

impl From<ConnInfo> for u64 {
    fn from(value: ConnInfo) -> Self {
        value.bridge_id as u64 | ((value.op_idx as u64) << 32)
    }
}

impl From<u64> for ConnInfo {
    fn from(value: u64) -> Self {
        Self {
            bridge_id: (value & 0xFFFF) as u32,
            op_idx: (value >> 32) as u32,
        }
    }
}
