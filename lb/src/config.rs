use config::{File, FileFormat};

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub servers: Vec<ConfigServer>,
}

impl Config {
    pub fn from_file() -> Self {
        ::config::Config::builder()
            .add_source(File::new("config.toml", FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigServer {
    pub name: String,
    pub bind: String,
    pub targets: Vec<ConfigTarget>,
    #[serde(default)]
    pub strategy: ConfigLoadBalancerStrategy,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ConfigTarget {
    Address(String),
    Detailed(ConfigTargetDetailed),
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigTargetDetailed {
    pub address: String,
    #[serde(default)]
    pub keep_alive: u64,
    #[serde(default)]
    pub domain_ttl: u64,
}

#[derive(Default, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigLoadBalancerStrategy {
    #[default]
    RoundRobin,
    LeastConnection,
}
