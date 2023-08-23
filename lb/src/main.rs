use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use io_uring::IoUring;

mod config;
mod connection;
mod op;
mod server;
mod target;
mod upstream;

fn main() {
    let mut config = dbg!(config::Config::from_file());
    let server = config.servers.pop().unwrap().into();
    let ring = IoUring::builder().build(10000).unwrap();
    let mut wrk = server::worker::ServerWorker::new(server, ring, 10000);

    // let mutex = Arc::new((Condvar::new(), Mutex::new(false)));
    // let (tx, rx) = std::sync::mpsc::sync_channel(100);
    // for i in 0..100 {
    //     let tx = tx.clone();
    //     let mutex = mutex.clone();
    //     std::thread::spawn(move || {
    //         std::thread::sleep(Duration::from_millis(1000));

    //         // let (cvar, lock) = &*mutex;
    //         // let mut run = lock.lock().unwrap();
    //         // while !*run {
    //         //     run = cvar.wait(run).unwrap();
    //         // }
    //         // drop(run);

    //         let f = ureq::agent()
    //             .get(&format!("http://localhost:9999/"))
    //             .send_string(&format!("{i}"))
    //             .unwrap();
    //         tx.send((i, f.into_string().unwrap())).unwrap();
    //     });
    // }

    // std::thread::spawn(move || {
    //     std::thread::sleep(Duration::from_millis(1000));

    //     let (cvar, lock) = &*mutex;
    //     let mut run = lock.lock().unwrap();
    //     *run = true;
    //     cvar.notify_all();

    //     let mut current = 0;
    //     println!("ff");
    //     loop {
    //         let Ok((id, s)) = rx.recv() else {
    //             break;
    //         };

    //         println!("arrival order {current}, id {id} -> {s}");
    //         current += 1;
    //     }
    // });

    wrk.init();
    wrk.foo();
}
