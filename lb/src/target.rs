use self::timeout::Timeout;

pub mod timeout;

struct Target {
    address: String,
    timeout: Timeout,
}

struct TargetPool {
    targets: Vec<Target>,
}

impl TargetPool {}
