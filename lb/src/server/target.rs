use std::{
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use crate::{config::ConfigTarget, target::timeout::Timeout};

pub struct Target {
    addr: TargetAddress,
    options: TargetOptions,
    timeout: Timeout,
}

impl Target {
    pub fn available_slots(&self) -> u64 {
        0
    }

    pub fn is_available(&self) -> bool {
        self.timeout.is_available()
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr.addr()
    }
}

#[derive(Debug, Default)]
pub struct TargetOptions {
    keep_alive: Duration,
}

impl From<ConfigTarget> for Target {
    fn from(value: ConfigTarget) -> Self {
        match value {
            ConfigTarget::Address(address) => Self {
                addr: TargetAddress::resolve(&address),
                options: TargetOptions::default(),
                timeout: Timeout::default(),
            },
            ConfigTarget::Detailed(detailed) => Self {
                addr: TargetAddress::resolve(&detailed.address),
                options: TargetOptions {
                    keep_alive: Duration::from_millis(detailed.keep_alive),
                },
                timeout: Timeout::default(),
            },
        }
    }
}

pub enum TargetAddress {
    One(SocketAddr),
    Multiple {
        addresses: Vec<SocketAddr>,
        current: AtomicUsize,
    },
}

impl TargetAddress {
    fn addr(&self) -> SocketAddr {
        match &self {
            TargetAddress::One(addr) => *addr,
            TargetAddress::Multiple { addresses, current } => {
                let idx = current
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |f| {
                        Some((f + 1) % addresses.len())
                    })
                    .expect("failed to update");

                addresses[idx]
            }
        }
    }
}

impl TargetAddress {
    fn resolve(s: &str) -> Self {
        if let Ok(addr) = SocketAddr::from_str(s) {
            Self::One(addr)
        } else {
            let resolved_at = Instant::now();
            let addresses = s.to_socket_addrs().unwrap().collect();

            Self::Multiple {
                addresses,
                current: AtomicUsize::new(0),
            }
        }
    }
}
