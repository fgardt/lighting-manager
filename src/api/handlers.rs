use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;
use std::time::{Duration, Instant};

use strum::VariantNames;
use strum_macros::EnumString;
use warp::Reply;

use crate::state::{Mode, State};

#[derive(Debug, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum HSVComponent {
    H,
    S,
    V,
}

impl fmt::Display for HSVComponent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::H => write!(f, "HUE"),
            Self::S => write!(f, "SAT"),
            Self::V => write!(f, "VAL"),
        }
    }
}

#[derive(EnumString)]
#[strum(ascii_case_insensitive)]
pub enum PlainTarget {
    H,
    S,
    V,
    Mode,
}

pub async fn static_root() -> Result<impl warp::Reply, Infallible> {
    Ok("RGB Strip Controller API v".to_owned() + env!("CARGO_PKG_VERSION"))
}

pub async fn static_all_modes() -> Result<impl Reply, Infallible> {
    let mut map: BTreeMap<&str, u8> = BTreeMap::new();

    for i in 0..Mode::VARIANTS.len() {
        map.insert(Mode::VARIANTS[i], i as u8);
    }

    Ok(warp::reply::json(&map))
}

pub async fn get_mode(state: State) -> Result<impl Reply, Infallible> {
    let safe_state = state.lock().await;

    Ok(format!("Current mode: {}", safe_state.mode))
}

pub async fn set_mode(new_mode: Mode, state: State) -> Result<String, Infallible> {
    let mut safe_state = state.lock().await;

    match new_mode {
        Mode::ALARM => {
            safe_state.interval = Duration::from_millis(1000);
        }
        Mode::COLORRAPE => {
            safe_state.interval =
                Duration::from_millis(500 + (9500.0 * (safe_state.hue / 360.0)) as u64);
        }
        Mode::STROBE => {
            safe_state.interval =
                Duration::from_millis(50 + (950.0 * (safe_state.hue / 360.0)) as u64);
        }
        _ => {
            safe_state.interval = Duration::from_millis(300_000);
        }
    };

    safe_state.render = true;
    safe_state.start = Instant::now();
    safe_state.mode = new_mode;

    Ok(format!("Updated mode: {}", new_mode))
}

pub async fn set_mode_int(new_mode: u8, state: State) -> Result<impl Reply, Infallible> {
    let mode_option = Mode::from_repr(new_mode);

    match mode_option {
        Some(mode) => set_mode(mode, state).await,
        None => Ok(format!("Unknown mode: {}", new_mode)),
    }
}

pub async fn get_component(
    component: HSVComponent,
    state: State,
) -> Result<impl Reply, Infallible> {
    let safe_state = state.lock().await;

    let value = match component {
        HSVComponent::H => safe_state.hue,
        HSVComponent::S => safe_state.sat,
        HSVComponent::V => safe_state.val,
    };

    Ok(format!("Current {}: {}", component, value))
}

pub async fn set_component(
    component: HSVComponent,
    value: f32,
    state: State,
) -> Result<impl Reply, Infallible> {
    let mut safe_state = state.lock().await;

    let result = match component {
        HSVComponent::H => {
            safe_state.hue = ((value % 360.0) + 360.0) % 360.0;

            // update interval when depending on hue value
            match safe_state.mode {
                Mode::COLORRAPE => {
                    safe_state.interval =
                        Duration::from_millis(500 + (9500.0 * (safe_state.hue / 360.0)) as u64);
                }
                Mode::STROBE => {
                    safe_state.interval =
                        Duration::from_millis(50 + (950.0 * (safe_state.hue / 360.0)) as u64);
                }
                _ => {}
            };

            safe_state.hue
        }
        HSVComponent::S => {
            safe_state.sat = value.clamp(0.0, 1.0);
            safe_state.sat
        }
        HSVComponent::V => {
            safe_state.val = value.clamp(0.0, 1.0);
            safe_state.val
        }
    };

    safe_state.render = true;

    Ok(format!("Updated {}: {}", component, result))
}

pub async fn set_component_int(
    component: HSVComponent,
    value: i16,
    state: State,
) -> Result<impl Reply, Infallible> {
    let result = match component {
        HSVComponent::H => (((value % 360) + 360) % 360) as f32,
        HSVComponent::S | HSVComponent::V => value.clamp(0, 255) as f32 / 255.0,
    };

    set_component(component, result, state).await
}

pub async fn get_plain(target: PlainTarget, state: State) -> Result<impl Reply, Infallible> {
    let safe_state = state.lock().await;

    match target {
        PlainTarget::H => Ok(safe_state.hue.to_string()),
        PlainTarget::S => Ok(safe_state.sat.to_string()),
        PlainTarget::V => Ok(safe_state.val.to_string()),
        PlainTarget::Mode => Ok(safe_state.mode.to_string().to_lowercase()),
    }
}
