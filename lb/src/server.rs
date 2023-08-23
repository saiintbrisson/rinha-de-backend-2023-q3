use std::net::SocketAddr;

use crate::config::{ConfigLoadBalancerStrategy, ConfigServer};

use self::{
    load_balancer::{LeastConnection, LoadBalancerStrategy, RoundRobin},
    target::Target,
};

pub mod bridge;
pub mod buffer;
mod connection_slot;
mod load_balancer;
mod target;
pub mod worker;

pub struct Server {
    pub bind_address: String,
    targets: Vec<Target>,
    load_balancer: LoadBalancerStrategy,
}

impl Server {
    pub fn available_slots(&self) -> u64 {
        self.targets.iter().map(Target::available_slots).sum()
    }

    pub fn next_target(&self) -> Option<SocketAddr> {
        match &self.load_balancer {
            LoadBalancerStrategy::RoundRobin(lb) => {
                loop {
                    let idx = lb.next_wrapping(self.targets.len());
                    let target = &self.targets[idx];
                    if !target.is_available() {
                        continue;
                    }

                    let addr = target.addr();
                    if addr.is_ipv4() {
                        return Some(addr);
                    }
                }

                return None;
            }
            LoadBalancerStrategy::LeastConnection => todo!(),
        }
    }
}

impl From<ConfigServer> for Server {
    fn from(value: ConfigServer) -> Self {
        Self {
            bind_address: value.bind,
            targets: value.targets.into_iter().map(From::from).collect(),
            load_balancer: match value.strategy {
                ConfigLoadBalancerStrategy::RoundRobin => {
                    LoadBalancerStrategy::RoundRobin(RoundRobin::default())
                }
                ConfigLoadBalancerStrategy::LeastConnection => {
                    LoadBalancerStrategy::LeastConnection
                }
            },
        }
    }
}
