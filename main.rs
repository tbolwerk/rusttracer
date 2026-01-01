#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
mod canvas;
use canvas::*;

mod transformations;
use transformations::*;
mod matrices;
use matrices::*;
mod tuples;
use tuples::*;

fn main() -> Result<(), ()> {
    const WIDTH: usize = 400;
    const HEIGHT: usize = 400;

    let mut c: Canvas<WIDTH, HEIGHT> = Canvas::new();

    // 12 o'clock position
    const SCALE: Matrix<4, 4> = scaling(150.0, 150.0, 150.0);
    const TRANSLATE: Matrix<4, 4> = translation(200.0, 200.0, 200.0);
    const START: Tuple = Tuple::point(0.0, 0.0, 1.0);

    const HOURS_COUNT: usize = 12;
    const STEP_SIZE: f32 = 360.0 / HOURS_COUNT as f32;
    const HOURS: [Matrix<4, 4>; HOURS_COUNT] = {
        let mut hours: [Matrix<4, 4>; HOURS_COUNT] = [Matrix::identity(); HOURS_COUNT];
        let mut i = 0;
        while i < HOURS_COUNT {
            let angle = radians((i as f32) * STEP_SIZE);
            hours[i] = rotation_y(angle).then(SCALE).then(TRANSLATE);
            i += 1;
        }
        hours
    };

    // Iterate over compile-time calculated transform matrix
    for transform in HOURS.iter() {
        let p = *transform * START;
        let x = p.x().round().clamp(0.0, (WIDTH - 1) as f32) as usize;
        let z = p.z().round().clamp(0.0, (HEIGHT - 1) as f32) as usize;
        println!("transform: {:?} tuple: {:?} x: {x}, z:{z}", p, transform);
        c.set(Color::white(), x, z);
    }

    let _ = c.write_ppm("chapter4.ppm", PpmFormat::P3, 255);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct Projectile {
    position: Tuple,
    velocity: Tuple,
}

impl Projectile {
    fn new(position: Tuple, velocity: Tuple) -> Self {
        Self { position, velocity }
    }
}

#[derive(Copy, Clone)]
struct Environment {
    gravity: Tuple,
    wind: Tuple,
}

impl Environment {
    fn new(gravity: Tuple, wind: Tuple) -> Self {
        Self { gravity, wind }
    }
}

fn tick(env: Environment, proj: Projectile) -> Projectile {
    let position = proj.position + proj.velocity;
    let velocity = proj.velocity + env.gravity + env.wind;
    Projectile::new(position, velocity)
}

fn chapter1() -> std::io::Result<()> {
    let start = Tuple::point(0.0, 1.0, 0.0);
    let velocity = normalize(&Tuple::vector(1.0, 1.8, 0.0)) * 11.25;
    let mut p = Projectile::new(start, velocity);
    let e = Environment::new(
        Tuple::vector(0.0, -0.1, 0.0),
        Tuple::vector(-0.01, 0.0, 0.0),
    );

    let mut positions: Vec<Tuple> = vec![];
    while p.position.y() > 0.0 {
        positions.push(p.position);
        p = tick(e, p);
    }

    const WIDTH: usize = 900;
    const HEIGHT: usize = 550;

    let mut canvas: Canvas<HEIGHT, WIDTH> = Canvas::new();

    for position in positions {
        let y = position.y().round().clamp(0.0, (HEIGHT - 1) as f32) as usize;
        let x = position.x().round().clamp(0.0, (WIDTH - 1) as f32) as usize;
        canvas.set(Color::white(), HEIGHT - y, x);
    }

    let filename = "chapter1.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6, 255);

    match result {
        Ok(_) => println!("Succesfully created {}!", filename),
        Err(err) => println!("{:?}", err),
    }

    Ok(())
}
