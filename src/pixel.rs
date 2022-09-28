use serde::{Deserialize, Serialize};

use rs_ws281x::RawColor;

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub enum Pixel {
    RGB { r: f32, g: f32, b: f32 },
    HSV { h: f32, s: f32, v: f32 },
    WHITE,
    GREEN,
    BLUE,
    RED,
    OFF,
}

impl Pixel {
    pub fn to_u8(&self) -> RawColor {
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
