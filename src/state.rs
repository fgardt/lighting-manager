use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, EnumVariantNames, FromRepr};
use tokio::sync::Mutex;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumVariantNames, FromRepr, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Mode {
    OFF = 0,
    STATIC = 1,
    RAINBOW = 2,
    SLEEP = 3,
    ALARM = 4,
    COLORRAPE = 5,
    STROBE = 6,
    IDENTIFY = 7,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OFF => write!(f, "OFF"),
            Self::STATIC => write!(f, "STATIC"),
            Self::RAINBOW => write!(f, "RAINBOW"),
            Self::SLEEP => write!(f, "SLEEP"),
            Self::ALARM => write!(f, "ALARM"),
            Self::COLORRAPE => write!(f, "COLORRAPE"),
            Self::STROBE => write!(f, "STROBE"),
            Self::IDENTIFY => write!(f, "IDENTIFY"),
        }
    }
}

pub struct StateStruct {
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub mode: Mode,
    pub interval: Duration,
    pub start: Instant,
    pub render: bool,
}

pub type State = Arc<Mutex<StateStruct>>;

pub fn init() -> State {
    Arc::new(Mutex::new(StateStruct {
        hue: 0.0,
        sat: 1.0,
        val: 1.0,
        mode: Mode::OFF,
        interval: Duration::from_millis(300_000),
        start: Instant::now(),
        render: true,
    }))
}
