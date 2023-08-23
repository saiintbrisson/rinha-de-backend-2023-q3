// use std::net::ToSocketAddrs;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};
use std::{future::Future, io, net::SocketAddr, sync::Arc};

use crate::http::codec::ConnectionCodec;
use crate::http::{Request, Response};
use futures_util::{SinkExt, StreamExt};
use http::header::USER_AGENT;
use http::{header::CONNECTION, HeaderValue};
use tokio::net::ToSocketAddrs;
use tokio::sync::OwnedSemaphorePermit;
use tokio::{net::TcpStream, sync::Semaphore};
use tokio_util::codec::Decoder;

type Handler<A, F> = fn(Request, A) -> F;

pub struct Server<A, F> {
    state: A,
    handler: Handler<A, F>,
    semaphore: Arc<Semaphore>,
}

const PERMITS: usize = 1_000;

impl<S, F> Server<S, F>
where
    S: Clone + Send + Sync + 'static,
    F: Future<Output = Response> + Send + 'static,
{
    pub fn new(state: S, handler: Handler<S, F>) -> Self {
        Self {
            state,
            handler,
            semaphore: Arc::new(Semaphore::new(PERMITS)),
        }
    }

    pub async fn bind<A: ToSocketAddrs>(self, addr: A) -> io::Result<()> {
        let server = Arc::new(self);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        tracing::info!(target: "listener", ?addr, "server is running");

        let (tx, mut rx) = tokio::sync::mpsc::channel(10_000);
        let pending = Arc::new(AtomicUsize::new(0));
        tokio::spawn({
            let pending = Arc::clone(&pending);
            async move {
                loop {
                    let (socket, addr) = listener.accept().await.unwrap();
                    pending.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    tx.send((socket, addr)).await.unwrap();
                }
            }
        });

        let mut now = Instant::now();
        let mut connections = 0usize;

        loop {
            let (socket, addr) = rx.recv().await.unwrap();
            let permit = server.acquire_permit().await;

            connections += 1;
            let pending = pending.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) - 1;
            if now.elapsed() > Duration::from_secs(1) {
                tracing::debug!(
                    target: "listener",
                    "{connections}/s with {} tasks running, pending connections: {pending}",
                    PERMITS - server.semaphore.available_permits()
                );
                now = Instant::now();
                connections = 0;
            }

            let server = server.clone();
            tokio::spawn(async move {
                let _ = tokio::time::timeout(
                    crate::TIMEOUT_DURATION,
                    server.handle_request(socket, addr, permit),
                )
                .await;
            });
        }
    }

    #[tracing::instrument(skip(self, socket, permit))]
    async fn handle_request(
        self: Arc<Self>,
        socket: TcpStream,
        addr: SocketAddr,
        permit: OwnedSemaphorePermit,
    ) {
        let mut codec = ConnectionCodec::default().framed(socket);
        let req = match codec.next().await.transpose() {
            Ok(Some(req)) => {
                tracing::debug!(?req, "received request");
                req
            }
            Ok(None) => {
                tracing::error!("connection ended before request");
                return;
            }
            Err(err) => {
                tracing::warn!(%err, "failed to read request");
                return;
            }
        };

        let user = req.headers().get(USER_AGENT).unwrap_or_else(|| {
            static UNKNOWN_AGENT: HeaderValue = HeaderValue::from_static("Unknown");
            &UNKNOWN_AGENT
        });

        let path = req.uri().to_string();
        tracing::info!(
            target: "requests",
            method = %req.method(),
            %path,
            ?user,
            r#""{} {path}" by {user:?}"#, req.method()
        );

        let now = Instant::now();
        let mut resp = (self.handler)(req, self.state.clone()).await;
        tracing::debug!(?resp, "handled in {:?}, sending response", now.elapsed());

        const CLOSE: HeaderValue = HeaderValue::from_static("close");
        resp.headers_mut().append(CONNECTION, CLOSE);

        drop(permit);

        if let Err(err) = codec.send(resp).await {
            tracing::warn!(%err, "failed to send response");
        }
    }

    async fn acquire_permit(&self) -> OwnedSemaphorePermit {
        loop {
            if let Ok(permit) = Arc::clone(&self.semaphore).try_acquire_owned() {
                break permit;
            }

            let mut factor = 1;
            loop {
                const BACKOFF: Duration = Duration::from_millis(50);
                tokio::time::sleep(factor * BACKOFF).await;
                factor *= 2;
                let available_permits = self.semaphore.available_permits();
                if available_permits >= PERMITS / 100 {
                    break;
                }
            }
        }
    }
}
