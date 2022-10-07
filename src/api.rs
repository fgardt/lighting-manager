use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

use error_stack::{IntoReport, Result, ResultExt};
use tokio::{
    runtime::Runtime,
    sync::oneshot::{self, Sender},
};
use warp::serve;

mod handlers;
mod routes;

use crate::state::State;

#[derive(Debug)]
pub struct ApiServerError;

impl fmt::Display for ApiServerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("API server error")
    }
}

impl Error for ApiServerError {}

pub fn run(
    state: State,
    socket: SocketAddr,
    runtime: &Runtime,
) -> Result<Sender<()>, ApiServerError> {
    runtime.block_on(start_api(state, socket, runtime))
}

// needs to run in a tokio runtime
async fn start_api(
    state: State,
    socket: SocketAddr,
    runtime: &Runtime,
) -> Result<Sender<()>, ApiServerError> {
    let (tx, rx) = oneshot::channel();

    let (_, server) = serve(routes::get(state))
        .try_bind_with_graceful_shutdown(socket, async move {
            info!("Starting API on {socket}");
            rx.await.ok();
            info!("API stopped");
        })
        .into_report()
        .attach_printable_lazy(|| format!("could not bind to {}", socket))
        .change_context(ApiServerError)?;

    runtime.spawn(server);

    Ok(tx)
}
