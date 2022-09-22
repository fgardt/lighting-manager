use rs_ws281x::{ChannelBuilder, ControllerBuilder, RawColor, StripType};
use std::io::Error;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

static DATA_PIN: i32 = 18;
static LED_COUNT: i32 = 300;

#[repr(u8)]
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

impl FromStr for Mode {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "OFF" => Ok(Self::OFF),
            "STATIC" => Ok(Self::STATIC),
            "RAINBOW" => Ok(Self::RAINBOW),
            "SLEEP" => Ok(Self::SLEEP),
            "ALARM" => Ok(Self::ALARM),
            "COLORRAPE" => Ok(Self::COLORRAPE),
            "STROBE" => Ok(Self::STROBE),
            "IDENTIFY" => Ok(Self::IDENTIFY),
            _ => Err(()),
        }
    }
}

#[allow(dead_code)]
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

fn main() -> Result<(), Error> {
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

    let mut current_mode = Mode::OFF;
    let mut start_time = Instant::now();
    let mut interval_time = Duration::from_millis(300_000);
    let mut progress_old = 0.0;

    let mut hue = 0.0;
    let mut sat = 1.0;
    let mut val = 1.0;

    // signal handling
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;

    while !term.load(Ordering::Relaxed) {
        // nightly has div_duration_f32/f64
        let delta_time = start_time.elapsed();
        let progress = ((delta_time.as_millis() % interval_time.as_millis()) as f32)
            / (interval_time.as_millis() as f32);

        let leds = controller.leds_mut(0);

        match current_mode {
            Mode::OFF => {
                let pixel_u8 = Pixel::OFF.to_u8();
                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::STATIC => {
                let pixel_u8 = Pixel::HSV {
                    h: hue,
                    s: sat,
                    v: val,
                }
                .to_u8();
                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::RAINBOW => {
                interval_time = Duration::from_millis(300_000);

                for (i, led) in leds.iter_mut().enumerate() {
                    let rainbow_hue = (i as f32 + 6000.0 * progress) * (360.0 / 150.0);

                    let pixel = Pixel::HSV {
                        h: rainbow_hue,
                        s: 1.0,
                        v: 1.0,
                    };

                    *led = pixel.to_u8();
                }
            }
            Mode::SLEEP => {
                interval_time = Duration::from_millis(300_000);

                if progress <= progress_old && progress_old > 0.0 {
                    current_mode = Mode::OFF;
                    progress_old = 0.0;
                } else {
                    progress_old = progress;
                }

                let sleep_sat = sat - progress * (sat / 2.0);
                let sleep_val = val - val * progress;

                for (i, led) in leds.iter_mut().enumerate() {
                    let sleep_hue = (i as f32 + 6000.0 * progress) * (360.0 / 150.0);

                    let pixel = Pixel::HSV {
                        h: sleep_hue,
                        s: sleep_sat,
                        v: sleep_val,
                    };

                    *led = pixel.to_u8();
                }
            }
            Mode::ALARM => {
                interval_time = Duration::from_millis(1000);
                let pixel_u8 = if progress >= 0.5 {
                    Pixel::RED.to_u8()
                } else {
                    Pixel::OFF.to_u8()
                };

                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::COLORRAPE => {
                interval_time = Duration::from_millis(500 + (9500.0 * (hue / 360.0)) as u64);
                let pixel_u8 = Pixel::HSV {
                    h: progress * 360.0,
                    s: sat,
                    v: val,
                }
                .to_u8();

                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::STROBE => {
                interval_time = Duration::from_millis(50 + (950.0 * (hue / 360.0)) as u64);
                let pixel_u8 = if progress >= 0.5 {
                    Pixel::WHITE.to_u8()
                } else {
                    Pixel::OFF.to_u8()
                };

                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::IDENTIFY => {
                let id_pixel = Pixel::RED.to_u8();
                let other_pixel = Pixel::WHITE.to_u8();

                for (i, led) in leds.iter_mut().enumerate() {
                    *led = if i == hue as usize {
                        id_pixel
                    } else {
                        other_pixel
                    };
                }
            }
        }

        controller.render().unwrap();
        thread::sleep(Duration::from_millis(10));
    }

    // turn all LEDs off
    let leds = controller.leds_mut(0);
    for led in leds {
        *led = Pixel::OFF.to_u8();
    }
    controller.render().unwrap();

    Ok(())
}
