use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;
use std::io::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rs_ws281x::{ChannelBuilder, ControllerBuilder, RawColor, StripType};
use serde::{Deserialize, Serialize};
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames, FromRepr};
use tokio::{runtime::Runtime, sync::Mutex};
use warp::{self, Filter};

static DATA_PIN: i32 = 18;
static LED_COUNT: i32 = 300;
static PORT: u16 = 88;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumVariantNames, FromRepr, EnumString)]
#[strum(ascii_case_insensitive)]
enum Mode {
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

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
enum Pixel {
    RGB { r: f32, g: f32, b: f32 },
    HSV { h: f32, s: f32, v: f32 },
    WHITE,
    GREEN,
    BLUE,
    RED,
    OFF,
}

impl Pixel {
    fn to_u8(&self) -> RawColor {
        match self {
            Self::RGB { r, g, b } => {
                let r_u: u8 = (r.clamp(0.0, 1.0) * 255.0) as u8;
                let g_u: u8 = (g.clamp(0.0, 1.0) * 255.0) as u8;
                let b_u: u8 = (b.clamp(0.0, 1.0) * 255.0) as u8;

                [b_u, g_u, r_u, 0]
            }

            Self::HSV { h, s, v } => {
                let hue: f32 = if (h % 360.0) < 0.0 {
                    h % 360.0 + 360.0
                } else {
                    h % 360.0
                };

                let c = v.clamp(0.0, 1.0) * s.clamp(0.0, 1.0);
                let x = c * (1.0 - (((hue / 60.0) % 2.0) - 1.0).abs());
                let m = v.clamp(0.0, 1.0) - c;

                let mut r1 = m;
                let mut g1 = m;
                let mut b1 = m;

                if hue < 60.0 {
                    r1 += c;
                    g1 += x;
                    // b1 += 0.0;
                } else if hue < 120.0 {
                    r1 += x;
                    g1 += c;
                    // b1 += 0.0;
                } else if hue < 180.0 {
                    // r1 += 0.0;
                    g1 += c;
                    b1 += x;
                } else if hue < 240.0 {
                    // r1 += 0.0;
                    g1 += x;
                    b1 += c;
                } else if hue < 300.0 {
                    r1 += x;
                    // g1 += 0.0;
                    b1 += c;
                } else {
                    r1 += c;
                    // g1 += 0.0;
                    b1 += x;
                }

                let r_u: u8 = (r1.clamp(0.0, 1.0) * 255.0) as u8;
                let g_u: u8 = (g1.clamp(0.0, 1.0) * 255.0) as u8;
                let b_u: u8 = (b1.clamp(0.0, 1.0) * 255.0) as u8;

                [b_u, g_u, r_u, 0]
            }

            Self::WHITE => [255, 255, 255, 0],

            Self::GREEN => [255, 0, 0, 0],

            Self::BLUE => [0, 255, 0, 0],

            Self::RED => [0, 0, 255, 0],

            Self::OFF => [0, 0, 0, 0],
        }
    }
}

type State = Arc<Mutex<StateStruct>>;
struct StateStruct {
    hue: f32,
    sat: f32,
    val: f32,
    mode: Mode,
    interval: Duration,
    start: Instant,
    render: bool,
}

struct API;

#[derive(Debug, EnumString)]
#[strum(ascii_case_insensitive)]
enum HSVComponent {
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
enum PlainTarget {
    H,
    S,
    V,
    Mode,
}

impl API {
    async fn run(state: State) {
        let routes = Self::get_routes(state);

        println!("Starting API on port {PORT}");

        warp::serve(routes).run(([0, 0, 0, 0], PORT)).await;

        println!("API stopped.");
    }

    fn get_routes(
        state: State,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        Self::static_routes()
            .or(Self::mode_routes(state.clone()))
            .or(Self::component_routes(state.clone()))
            .or(Self::plain_routes(state))
    }

    fn with_state(state: State) -> impl Filter<Extract = (State,), Error = Infallible> + Clone {
        warp::any().map(move || state.clone())
    }

    async fn static_root_handler() -> Result<impl warp::Reply, Infallible> {
        Ok("RGB Strip Controller API v0.0.0")
    }

    async fn static_all_modes_handler() -> Result<impl warp::Reply, Infallible> {
        let mut map: BTreeMap<&str, u8> = BTreeMap::new();

        for i in 0..Mode::VARIANTS.len() {
            map.insert(Mode::VARIANTS[i], i as u8);
        }

        Ok(warp::reply::json(&map))
    }

    fn static_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let root = warp::path::end().and_then(Self::static_root_handler);

        let all_modes = warp::path!("all_modes").and_then(Self::static_all_modes_handler);

        root.or(all_modes)
    }

    async fn get_mode_handler(state: State) -> Result<impl warp::Reply, Infallible> {
        let safe_state = state.lock().await;

        Ok(format!("Current mode: {}", safe_state.mode))
    }

    async fn set_mode_handler(new_mode: Mode, state: State) -> Result<String, Infallible> {
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

    async fn set_mode_handler_int(
        new_mode: u8,
        state: State,
    ) -> Result<impl warp::Reply, Infallible> {
        let mode_option = Mode::from_repr(new_mode);

        match mode_option {
            Some(mode) => Self::set_mode_handler(mode, state).await,
            None => Ok(format!("Unknown mode: {}", new_mode)),
        }
    }

    fn mode_routes(
        state: State,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let get_mode = warp::path!("mode")
            .and(Self::with_state(state.clone()))
            .and_then(Self::get_mode_handler);
        let set_mode = warp::path!("mode" / Mode)
            .and(Self::with_state(state.clone()))
            .and_then(Self::set_mode_handler);
        let set_mode_int = warp::path!("mode" / u8)
            .and(Self::with_state(state))
            .and_then(Self::set_mode_handler_int);

        get_mode.or(set_mode).or(set_mode_int)
    }

    async fn get_component_handler(
        component: HSVComponent,
        state: State,
    ) -> Result<impl warp::Reply, Infallible> {
        let safe_state = state.lock().await;

        let value = match component {
            HSVComponent::H => safe_state.hue,
            HSVComponent::S => safe_state.sat,
            HSVComponent::V => safe_state.val,
        };

        Ok(format!("Current {}: {}", component, value))
    }

    async fn set_component_handler(
        component: HSVComponent,
        value: f32,
        state: State,
    ) -> Result<impl warp::Reply, Infallible> {
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

    async fn set_component_handler_int(
        component: HSVComponent,
        value: i16,
        state: State,
    ) -> Result<impl warp::Reply, Infallible> {
        let result = match component {
            HSVComponent::H => (((value % 360) + 360) % 360) as f32,
            HSVComponent::S | HSVComponent::V => value.clamp(0, 255) as f32 / 255.0,
        };

        Self::set_component_handler(component, result, state).await
    }

    fn component_routes(
        state: State,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let get_component = warp::path!(HSVComponent)
            .and(Self::with_state(state.clone()))
            .and_then(Self::get_component_handler);
        let set_component_int = warp::path!(HSVComponent / i16)
            .and(Self::with_state(state.clone()))
            .and_then(Self::set_component_handler_int);
        let set_component = warp::path!(HSVComponent / f32)
            .and(Self::with_state(state))
            .and_then(Self::set_component_handler);

        get_component.or(set_component_int).or(set_component)
    }

    async fn get_plain_handler(
        target: PlainTarget,
        state: State,
    ) -> Result<impl warp::Reply, Infallible> {
        let safe_state = state.lock().await;

        match target {
            PlainTarget::H => Ok(safe_state.hue.to_string()),
            PlainTarget::S => Ok(safe_state.sat.to_string()),
            PlainTarget::V => Ok(safe_state.val.to_string()),
            PlainTarget::Mode => Ok(safe_state.mode.to_string().to_lowercase()),
        }
    }

    fn plain_routes(
        state: State,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("plain" / PlainTarget)
            .and(Self::with_state(state))
            .and_then(Self::get_plain_handler)
    }
}

fn main() -> Result<(), Error> {
    // state storage
    let state = Arc::new(Mutex::new(StateStruct {
        hue: 0.0,
        sat: 1.0,
        val: 1.0,
        mode: Mode::OFF,
        interval: Duration::from_millis(300_000),
        start: Instant::now(),
        render: true,
    }));

    let api_state = Arc::clone(&state);

    let tr = Runtime::new().expect("Couldn't get tokio runtime.");
    tr.spawn(async move {
        API::run(api_state).await;
    });

    // LED strip
    let mut controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(DATA_PIN)
                .count(LED_COUNT)
                .strip_type(StripType::Ws2811Grb)
                .brightness(255)
                .build(),
        )
        .build()
        .unwrap();

    let leds = controller.leds_mut(0);

    println!("Number of LEDs: {}", leds.len());

    // turn all LEDs off
    for led in leds {
        *led = Pixel::OFF.to_u8();
    }

    controller.render().unwrap();

    // signal handling
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    let mut progress_old = 0.0;

    let rt = Runtime::new().expect("Couldn't get tokio runtime.");

    while !term.load(Ordering::Relaxed) {
        {
            let mut safe_state = rt.block_on(state.lock());

            // nightly has div_duration_f32/f64
            let delta_time = safe_state.start.elapsed();
            let progress = ((delta_time.as_millis() % safe_state.interval.as_millis()) as f32)
                / (safe_state.interval.as_millis() as f32);

            let leds = controller.leds_mut(0);

            match safe_state.mode {
                Mode::OFF => {
                    let pixel_u8 = Pixel::OFF.to_u8();
                    for led in leds {
                        *led = pixel_u8;
                    }
                }
                Mode::STATIC => {
                    let pixel_u8 = Pixel::HSV {
                        h: safe_state.hue,
                        s: safe_state.sat,
                        v: safe_state.val,
                    }
                    .to_u8();
                    for led in leds {
                        *led = pixel_u8;
                    }
                }
                Mode::RAINBOW => {
                    for (i, led) in leds.iter_mut().enumerate() {
                        let rainbow_hue = 6000.0f32.mul_add(progress, i as f32) * (360.0 / 150.0);

                        let pixel = Pixel::HSV {
                            h: rainbow_hue,
                            s: safe_state.sat,
                            v: safe_state.val,
                        };

                        *led = pixel.to_u8();
                    }

                    safe_state.render = true;
                }
                Mode::SLEEP => {
                    if progress <= progress_old && progress_old > 0.0 {
                        safe_state.mode = Mode::OFF;
                        progress_old = 0.0;
                    } else {
                        progress_old = progress;
                    }

                    let sleep_sat = safe_state.sat - progress * (safe_state.sat / 2.0);
                    let sleep_val = safe_state.val - safe_state.val * progress;

                    for (i, led) in leds.iter_mut().enumerate() {
                        let sleep_hue = 6000.0f32.mul_add(progress, i as f32) * (360.0 / 150.0);

                        let pixel = Pixel::HSV {
                            h: sleep_hue,
                            s: sleep_sat,
                            v: sleep_val,
                        };

                        *led = pixel.to_u8();
                    }

                    safe_state.render = true;
                }
                Mode::ALARM => {
                    let pixel_u8 = if progress >= 0.5 {
                        Pixel::RED.to_u8()
                    } else {
                        Pixel::OFF.to_u8()
                    };

                    for led in leds {
                        *led = pixel_u8;
                    }

                    safe_state.render = true;
                }
                Mode::COLORRAPE => {
                    let pixel_u8 = Pixel::HSV {
                        h: progress * 360.0,
                        s: safe_state.sat,
                        v: safe_state.val,
                    }
                    .to_u8();

                    for led in leds {
                        *led = pixel_u8;
                    }

                    safe_state.render = true;
                }
                Mode::STROBE => {
                    let pixel_u8 = if progress >= 0.5 {
                        Pixel::WHITE.to_u8()
                    } else {
                        Pixel::OFF.to_u8()
                    };

                    for led in leds {
                        *led = pixel_u8;
                    }

                    safe_state.render = true;
                }
                Mode::IDENTIFY => {
                    let id_pixel = Pixel::RED.to_u8();
                    let other_pixel = Pixel::WHITE.to_u8();

                    for (i, led) in leds.iter_mut().enumerate() {
                        *led = if i == safe_state.hue as usize {
                            id_pixel
                        } else {
                            other_pixel
                        };
                    }
                }
            }

            if safe_state.render {
                controller.render().unwrap();
                safe_state.render = false;
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    //turn all LEDs off
    let leds = controller.leds_mut(0);
    for led in leds {
        *led = Pixel::OFF.to_u8();
    }
    controller.render().unwrap();

    Ok(())
}
