use crate::external_tuples::*;
pub trait PrettyPrint {
    fn pp(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Color {
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
}

impl PrettyPrint for Color {
    fn pp(&self) -> String {
        format!("{} {} {}", self.r, self.g, self.b)
    }
}
