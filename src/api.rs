use std::net::SocketAddr;

use tokio::runtime::Runtime;
use warp::serve;

mod handlers;
mod routes;

use crate::state::State;

pub fn run(state: State, socket: SocketAddr, runtime: &Runtime) {
    runtime.spawn(async move {
        println!("Starting API on {socket}");

        serve(routes::get(state)).run(([0, 0, 0, 0], socket.port())).await;

        println!("API stopped.");
    });
}
