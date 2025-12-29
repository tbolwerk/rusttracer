use std::fmt::Write as StringWrite;
use std::fs::File;
use std::io::Write;

pub trait PrettyPrint {
    fn pp(&self) -> String;
}

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

struct HeapMatrix<T: std::marker::Copy + PrettyPrint, const ROWS: usize, const COLS: usize> {
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

#[derive(Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn red() -> Self {
        Self { r: 255, g: 0, b: 0 }
    }
    pub fn green() -> Self {
        Self { r: 0, g: 255, b: 0 }
    }
    pub fn blue() -> Self {
        Self { r: 0, g: 0, b: 255 }
    }
    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }
    pub fn white() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
        }
    }
}

impl PrettyPrint for Color {
    fn pp(&self) -> String {
        format!("{} {} {}", self.r, self.g, self.b)
    }
}

pub struct Canvas<const ROWS: usize, const COLS: usize> {
    pixels: HeapMatrix<Color, ROWS, COLS>,
}

impl<const ROWS: usize, const COLS: usize> Canvas<ROWS, COLS> {
    pub fn new() -> Self {
        Self {
            pixels: HeapMatrix::new(Color::black()),
        }
    }
    pub fn set(&mut self, value: Color, row: usize, col: usize) -> &mut Self {
        self.pixels.set(value, row, col);
        self
    }
    pub fn write_ppm(
        &self,
        filename: &str,
        format: PpmFormat,
        max_color: u8,
    ) -> Result<(), std::io::Error> {
        let mut file = File::create(filename)?;
        let header = format!("{}\n{} {}\n{}", format.pp(), COLS, ROWS, max_color);
        let _ = writeln!(file, "{}", header,);
        match format {
            PpmFormat::P3 => {
                writeln!(file, "{}", self.pixels.pp())
            }

            PpmFormat::P6 => {
                file.write_all(&self.to_bytes())?;
                Ok(())
            }
        }
    }
}
