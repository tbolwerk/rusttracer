use crate::colors::*;
use crate::tuples::*;
use rayon::prelude::*;
use std::fmt::Write as StringWrite;
use std::fs::File;
use std::io::Write;

pub enum PpmFormat {
    P3,
    P6,
}

impl PrettyPrint for PpmFormat {
    fn pp(&self) -> String {
        match self {
            PpmFormat::P3 => "P3".to_string(),
            PpmFormat::P6 => "P6".to_string(),
        }
    }
}

pub struct HeapMatrix<T: std::marker::Copy + PrettyPrint, const ROWS: usize, const COLS: usize> {
    data: Box<[T]>,
}

impl<const ROWS: usize, const COLS: usize> Serialize for Canvas<ROWS, COLS> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(ROWS * COLS * 3);
        for row in 0..ROWS {
            for col in 0..COLS {
                let pixel = self.pixels.get(row, col);
                buffer.push(pixel.r);
                buffer.push(pixel.g);
                buffer.push(pixel.b);
            }
        }
        buffer
    }
}

impl<T: PrettyPrint + Copy + Send, const ROWS: usize, const COLS: usize> HeapMatrix<T, ROWS, COLS> {
    pub fn par_rows_mut(&mut self) -> impl IndexedParallelIterator<Item = &mut [T]> {
        self.data.par_chunks_mut(COLS)
    }
}

impl<T: std::marker::Copy + PrettyPrint, const ROWS: usize, const COLS: usize> PrettyPrint
    for HeapMatrix<T, ROWS, COLS>
{
    fn pp(&self) -> String {
        let mut sb = String::new();
        for row in 0..ROWS {
            for col in 0..COLS {
                let _ = write!(sb, "{} ", self.get(row, col).pp());
            }
            let _ = write!(sb, "{}", "\n");
        }
        sb
    }
}

impl<T: std::marker::Copy + PrettyPrint, const ROWS: usize, const COLS: usize>
    HeapMatrix<T, ROWS, COLS>
{
    fn new(value: T) -> Self {
        Self {
            data: vec![value; ROWS * COLS].into_boxed_slice(),
        }
    }
    fn set(&mut self, value: T, row: usize, col: usize) -> () {
        self.data[row * COLS + col] = value;
    }
    fn get(&self, row: usize, col: usize) -> &T {
        &self.data[row * COLS + col]
    }
}

pub trait Serialize {
    fn to_bytes(&self) -> Vec<u8>;
}
pub struct Canvas<const ROWS: usize, const COLS: usize> {
    pub pixels: HeapMatrix<Pixel, ROWS, COLS>,
    max_color: u8,
}

impl<const ROWS: usize, const COLS: usize> Canvas<ROWS, COLS> {
    pub fn new(max_color: u8) -> Self {
        Self {
            pixels: HeapMatrix::new(Pixel::black()),
            max_color,
        }
    }
    pub fn set(&mut self, value: Pixel, row: usize, col: usize) -> () {
        self.pixels.set(value, row, col);
    }
    // Build a canvas from a row-major 0x00RRGGBB framebuffer (the format the GPU
    // backend returns). `argb` must hold exactly ROWS*COLS pixels, row by row from
    // the top-left, matching the canvas layout.
    pub fn from_argb(argb: &[u32]) -> Self {
        let mut canvas = Self::new(255);
        for row in 0..ROWS {
            for col in 0..COLS {
                let p = argb[row * COLS + col];
                canvas.set(
                    Pixel::new((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    row,
                    col,
                );
            }
        }
        canvas
    }
    pub fn write_pixel(&mut self, color: Color, row: usize, col: usize) -> () {
        let value = Pixel::clamp(0, self.max_color, color);
        self.set(value, row, col)
    }
    pub fn write_ppm(&self, filename: &str, format: PpmFormat) -> Result<(), std::io::Error> {
        let mut file = File::create(filename)?;
        self.write_ppm_to(&mut file, format)
    }
    // Pack the canvas into a 0x00RRGGBB buffer, row-major from the top-left,
    // for a framebuffer window (minifb's `update_with_buffer`).
    pub fn to_argb(&self) -> Vec<u32> {
        let mut buffer = Vec::with_capacity(ROWS * COLS);
        for row in 0..ROWS {
            for col in 0..COLS {
                let p = self.pixels.get(row, col);
                buffer.push((p.r as u32) << 16 | (p.g as u32) << 8 | p.b as u32);
            }
        }
        buffer
    }
    // Serialize a PPM to any writer. `write_ppm` uses it for files; the live
    // flythrough uses it to stream P6 frames to stdout for a piped player.
    pub fn write_ppm_to<W: Write>(
        &self,
        out: &mut W,
        format: PpmFormat,
    ) -> Result<(), std::io::Error> {
        let header = format!("{}\n{} {}\n{}", format.pp(), COLS, ROWS, self.max_color);
        writeln!(out, "{}", header)?;
        match format {
            PpmFormat::P3 => writeln!(out, "{}", self.pixels.pp()),
            PpmFormat::P6 => out.write_all(&self.to_bytes()),
        }
    }
}
