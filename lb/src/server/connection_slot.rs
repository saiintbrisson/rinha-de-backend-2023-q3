use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

type Foo = Arc<(Mutex<usize>, Condvar)>;

fn foo(timeout: Duration) {
    let (tex, cond): &(Mutex<usize>, Condvar) = todo!();
    if let Ok(slots) = tex.lock() {
        cond.wait_timeout(slots, timeout);
    }
}

pub struct ConnectionSlot {
    pub tex: Foo,
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        let (tex, cond) = self.tex.as_ref();
        if let Ok(mut slots) = tex.lock() {
            *slots += 1;
        }
        cond.notify_one();
    }
}
