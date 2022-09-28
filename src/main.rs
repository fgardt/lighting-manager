use std::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tokio::runtime::Runtime;

mod api;
mod controller;
mod pixel;
mod state;

static DATA_PIN: i32 = 18;
static LED_COUNT: i32 = 300;
static PORT: u16 = 88;

fn main() -> Result<(), Error> {
    // state storage
    let state = state::init();

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT);

    api::run(Arc::clone(&state), socket);

    let mut controller = controller::init(DATA_PIN, LED_COUNT);

    // signal handling
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    let rt = Runtime::new().expect("Couldn't get runtime for state mutex.");

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
