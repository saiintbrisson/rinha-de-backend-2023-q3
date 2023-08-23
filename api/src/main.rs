mod domains;
mod error;
mod handler;
mod http;
// mod io_uring;
mod repositories;
mod server;

use std::{process, sync::Arc, time::Duration};

use repositories::{sql::SqlPeopleRepository, PeopleRepository};
use server::Server;
use tracing_subscriber::EnvFilter;

const SERVER_ADDRESS: &str = "0.0.0.0:80";

const TIMEOUT_DURATION: Duration = Duration::from_secs(15);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let server_address =
        std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| SERVER_ADDRESS.to_string());

    let repository = SqlPeopleRepository::connect().await;
    let state = AppState {
        repository: Arc::new(repository),
    };

    let server = Server::new(state, handler::route_request);

    if let Err(err) = server.bind(server_address).await {
        tracing::error!(%err, "server failed");
        process::exit(1);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<dyn PeopleRepository + Send + Sync>,
}
