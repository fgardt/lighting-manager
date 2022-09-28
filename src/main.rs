use std::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use clap::Parser;
use tokio::runtime::Runtime;

mod api;
mod controller;
mod pixel;
mod state;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Sets the port to listen on
    #[clap(short, long, value_parser)]
    port: Option<u16>,

    /// Sets the ip address to listen on
    #[clap(short, long, value_parser)]
    address: Option<IpAddr>,

    /// Sets the pin to which the WS281x LED string is connected
    #[clap(long, value_parser)]
    pin: Option<i32>,

    /// Sets the count of LEDs in the string
    #[clap(short, long, value_parser)]
    count: Option<i32>,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let port = match cli.port.as_ref() {
        Some(port) => *port,
        None => 88,
    };

    let address = match cli.address.as_ref() {
        Some(address) => *address,
        None => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
    };

    let pin = match cli.pin.as_ref() {
        Some(pin) => *pin,
        None => 18,
    };

    let count = match cli.count.as_ref() {
        Some(count) => *count,
        None => 300,
    };

    // state storage
    let state = state::init();

    let socket = SocketAddr::new(address, port);
    let rt = Runtime::new().expect("Couldn't get tokio runtime.");

    api::run(Arc::clone(&state), socket, &rt);

    let mut controller = controller::init(pin, count);

    // signal handling
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    while !term.load(Ordering::Relaxed) {
        {
            let safe_state = rt.block_on(state.lock());

            controller.update(safe_state);
        }

        thread::sleep(Duration::from_millis(10));
    }

    //turn all LEDs off
    controller.off();

    Ok(())
}
