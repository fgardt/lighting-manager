use std::convert::Infallible;
use warp::{any, log, path, Filter, Rejection, Reply};

use crate::api::handlers::{self, HSVComponent, PlainTarget};
use crate::state::{Mode, State};

pub fn get(state: State) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    trace!("building routes");
    static_routes()
        .with(log("access-log"))
        .or(mode_routes(state.clone()))
        .or(component_routes(state.clone()))
        .or(plain_routes(state))
}

fn with_state(state: State) -> impl Filter<Extract = (State,), Error = Infallible> + Clone {
    any().map(move || state.clone())
}

fn static_routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let root = path::end().and_then(handlers::static_root);

    let all_modes = path!("all_modes").and_then(handlers::static_all_modes);

    root.or(all_modes)
}

fn mode_routes(state: State) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let get_mode = path!("mode")
        .and(with_state(state.clone()))
        .and_then(handlers::get_mode);
    let set_mode = path!("mode" / Mode)
        .and(with_state(state.clone()))
        .and_then(handlers::set_mode);
    let set_mode_int = path!("mode" / u8)
        .and(with_state(state))
        .and_then(handlers::set_mode_int);

    get_mode.or(set_mode).or(set_mode_int)
}

fn component_routes(state: State) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let get_component = path!(HSVComponent)
        .and(with_state(state.clone()))
        .and_then(handlers::get_component);
    let set_component_int = path!(HSVComponent / i16)
        .and(with_state(state.clone()))
        .and_then(handlers::set_component_int);
    let set_component = path!(HSVComponent / f32)
        .and(with_state(state))
        .and_then(handlers::set_component);

    get_component.or(set_component_int).or(set_component)
}

fn plain_routes(state: State) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path!("plain" / PlainTarget)
        .and(with_state(state))
        .and_then(handlers::get_plain)
}
