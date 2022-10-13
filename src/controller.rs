use std::error::Error;
use std::fmt;

use error_stack::{IntoReport, Result, ResultExt};
use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder, StripType};
use tokio::sync::MutexGuard;

use crate::pixel::Pixel;
use crate::state::{Mode, StateStruct};

pub struct Data {
    controller: Controller,
    progress_old: f32,
}

#[derive(Debug)]
pub struct ControllerError;

impl fmt::Display for ControllerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Controller error: unable to process")
    }
}

impl Error for ControllerError {}

pub fn init(pin: i32, count: i32) -> Result<Data, ControllerError> {
    let controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(pin)
                .count(count)
                .strip_type(StripType::Ws2811Grb)
                .brightness(255)
                .build(),
        )
        .build()
        .into_report()
        .attach_printable_lazy(|| {
            format!(
                "could not create controller on pin {} with {} leds",
                pin, count
            )
        })
        .change_context(ControllerError)?;

    let mut data = Data {
        controller,
        progress_old: 0.0,
    };

    data.off()?;

    Ok(data)
}

impl Data {
    pub fn update(&mut self, mut state: MutexGuard<StateStruct>) -> Result<(), ControllerError> {
        let delta_time = state.start.elapsed();
        let progress = ((delta_time.as_millis() % state.interval.as_millis()) as f32)
            / (state.interval.as_millis() as f32);

        let leds = self.controller.leds_mut(0);

        match state.mode {
            Mode::OFF => {
                let pixel_u8 = Pixel::OFF.to_u8();
                for led in leds {
                    *led = pixel_u8;
                }
            }
            Mode::STATIC => {
                let pixel_u8 = Pixel::HSV {
                    h: state.hue,
                    s: state.sat,
                    v: state.val,
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
                        s: state.sat,
                        v: state.val,
                    };

                    *led = pixel.to_u8();
                }

                state.render = true;
            }
            Mode::SLEEP => {
                if progress <= self.progress_old && self.progress_old > 0.0 {
                    state.mode = Mode::OFF;
                    self.progress_old = 0.0;
                } else {
                    self.progress_old = progress;
                }

                let sleep_sat = state.sat - progress * (state.sat / 2.0);
                let sleep_val = state.val - state.val * progress;

                for (i, led) in leds.iter_mut().enumerate() {
                    let sleep_hue = 6000.0f32.mul_add(progress, i as f32) * (360.0 / 150.0);

                    let pixel = Pixel::HSV {
                        h: sleep_hue,
                        s: sleep_sat,
                        v: sleep_val,
                    };

                    *led = pixel.to_u8();
                }

                state.render = true;
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

                state.render = true;
            }
            Mode::COLORRAPE => {
                let pixel_u8 = Pixel::HSV {
                    h: progress * 360.0,
                    s: state.sat,
                    v: state.val,
                }
                .to_u8();

                for led in leds {
                    *led = pixel_u8;
                }

                state.render = true;
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

                state.render = true;
            }
            Mode::IDENTIFY => {
                let id_pixel = Pixel::RED.to_u8();
                let other_pixel = Pixel::WHITE.to_u8();

                for (i, led) in leds.iter_mut().enumerate() {
                    *led = if i == state.hue as usize {
                        id_pixel
                    } else {
                        other_pixel
                    };
                }
            }
        }

        if state.render {
            state.render = false;
            self.controller
                .render()
                .into_report()
                .attach_printable_lazy(|| "unable to render new values")
                .change_context(ControllerError)?;
        }

        Ok(())
    }

    pub fn off(&mut self) -> Result<(), ControllerError> {
        let leds = self.controller.leds_mut(0);

        for led in leds {
            *led = Pixel::OFF.to_u8();
        }

        self.controller
            .render()
            .into_report()
            .attach_printable_lazy(|| "unable to turn off all LEDs")
            .change_context(ControllerError)?;

        Ok(())
    }
}
