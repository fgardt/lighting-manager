use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use clap::Parser;
use error_stack::{IntoReport, ResultExt};
use tokio::runtime::Runtime;

#[macro_use]
extern crate log;

mod api;
mod controller;
mod logging;
mod pixel;
mod state;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Sets the port to listen on
    #[clap(short, long, value_parser, default_value_t = 88)]
    port: u16,

    /// Sets the ip address to listen on
    #[clap(short, long, value_parser, default_value_t = IpAddr::V4(Ipv4Addr::new(0,0,0,0)))]
    address: IpAddr,

    /// Sets the pin to which the WS281x LED string is connected
    #[clap(short = 'P', long, value_parser)]
    pin: i32,

    /// Sets the count of LEDs in the string
    #[clap(short, long, value_parser)]
    count: i32,

    /// Sets the used logging level
    /// Possible values: error, warn, info, debug, trace
    /// For no logging don't set this option
    /// Note: the LOG_LEVEL environment variable overrides this option
    #[clap(long, value_parser, verbatim_doc_comment)]
    log_level: Option<log::Level>,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let logger = match cli.log_level.as_ref() {
        Some(level) => logging::init(level.as_str()),
        None => logging::init("Off"),
    };

    match logger {
        Ok(_) => {}
        Err(report) => {
            eprintln!("{report:?}");
            return Err(Error::new(ErrorKind::Other, "logging setup error"));
        }
    }

    // state storage
    let state = state::init();

    let rt = Runtime::new()
        .into_report()
        .attach_printable_lazy(|| "unable to get tokio runtime");

    let rt = match rt {
        Ok(runtime) => runtime,
        Err(report) => {
            error!("{report:?}");
            return Err(Error::new(ErrorKind::Other, "runtime error"));
        }
    };

    let stop_api = match api::run(
        Arc::clone(&state),
        SocketAddr::new(cli.address, cli.port),
        &rt,
    ) {
        Ok(tx) => tx,
        Err(report) => {
            error!("{report:?}");
            return Err(Error::new(ErrorKind::Other, "api server error"));
        }
    };

    let mut controller = match controller::init(cli.pin, cli.count) {
        Ok(data) => data,
        Err(report) => {
            let _ = stop_api.send(());
            error!("{report:?}");
            return Err(Error::new(ErrorKind::Other, "controller error"));
        }
    };

    // signal handling
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    info!("Running");

    while !term.load(Ordering::Relaxed) {
        {
            let safe_state = rt.block_on(state.lock());

            match controller.update(safe_state) {
                Ok(_) => {}
                Err(report) => {
                    warn!("{report:?}");
                }
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    let _ = stop_api.send(());

    //turn all LEDs off
    match controller.off() {
        Ok(_) => {}
        Err(report) => {
            warn!("{report:?}");
        }
    }

    info!("Stopped");

    Ok(())
}
