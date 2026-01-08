use crate::tuples::Color;

use std::ops::Mul;
pub trait PrettyPrint {
    fn pp(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub const fn new(r: u8, g: u8, b: u8) -> Pixel {
        Self { r, g, b }
    }
    pub const fn red() -> Self {
        Self::new(255, 0, 0)
    }
    pub const fn green() -> Self {
        Self::new(0, 255, 0)
    }
    pub const fn blue() -> Self {
        Self::new(0, 0, 255)
    }
    pub const fn black() -> Self {
        Self::new(0, 0, 0)
    }
    pub const fn white() -> Self {
        Self::new(255, 255, 255)
    }
    pub fn clamp(min: u8, max: u8, color: Color) -> Pixel {
        Pixel {
            r: (color.r.mul(max as f32).round() as u8).max(min).min(max),
            g: (color.g.mul(max as f32).round() as u8).max(min).min(max),
            b: (color.b.mul(max as f32).round() as u8).max(min).min(max),
        }
    }
}

impl PrettyPrint for Pixel {
    fn pp(&self) -> String {
        format!("{} {} {}", self.r, self.g, self.b)
    }
}
