#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
mod canvas;
use canvas::*;

mod worlds;
use worlds::*;
mod colors;
use colors::*;
mod materials;
use materials::*;
mod lights;
use lights::*;
mod intersections;
use intersections::*;
mod spheres;
use spheres::*;
mod rays;
use rays::*;
mod transformations;
use transformations::*;
mod matrices;
use matrices::*;
mod tuples;
use tuples::*;

use crate::tuples::external_tuples::{TupleKind, VectorMath};

fn main() -> Result<(), ()> {
    let _ = chapter1();
    let _ = chapter4();
    let _ = chapter5();
    let _ = chapter6();
    Ok(())
}
fn chapter6() {
    let mut sphere = Sphere::unit();
    let mut material = Material::default();
    let color = TupleKind::color(1.0, 0.2, 1.0);
    material.set_color(color);
    sphere.set_material(&material);

    const TRANSFORM: Matrix<4, 4> = Matrix::identity()
        .then(scaling(0.1, 0.1, 0.1))
        .then(rotation_z(PI / 6.0))
        .then(shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    sphere.set_transform(&TRANSFORM);

    let light_position = TupleKind::point(-10.0, 10.0, -10.0);
    let light_color = TupleKind::color(1.0, 1.0, 1.0);
    let light = Light::Point(PointLight::new(light_position, light_color));

    let ray_origin = TupleKind::point(0.0, 0.0, -0.5);
    let wall_z = 10.0;
    let wall_size = 7.0;
    const CANVAS_PIXELS: usize = 1000;
    let pixel_size = wall_size / CANVAS_PIXELS as f32;
    let half = wall_size / 2.0;
    let mut canvas: Canvas<CANVAS_PIXELS, CANVAS_PIXELS> = Canvas::new(255);
    for y in 0..CANVAS_PIXELS {
        let world_y = half - pixel_size * y as f32;
        for x in 0..CANVAS_PIXELS {
            let world_x = -half + pixel_size * x as f32;
            let current_position = TupleKind::point(world_x, world_y, wall_z);
            let ray = Ray::new(ray_origin, (current_position - ray_origin).normalize());
            let xs = sphere.intersect(&ray);
            match xs.hit() {
                None => (),
                Some(hit) => {
                    let point = ray.position(hit.t);
                    let normal = hit.object.normal_at(&point);
                    let eye = -ray.direction;

                    let color = lightning(&hit.object.material, light, point, eye, normal);
                    canvas.write_pixel(color, y, x);
                }
            }
        }
    }
    let filename = "chapter6.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(err) => println!("Something went wrong! {err}"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
fn chapter5() {
    let ray_origin = TupleKind::point(0.0, 0.0, -5.0);
    const WALL_Z: f32 = 10.0;
    const WALL_SIZE: f32 = 7.0;
    const CANVAS_PIXELS: usize = 100;
    const PIXEL_SIZE: f32 = WALL_SIZE / CANVAS_PIXELS as f32;
    const HALF: f32 = WALL_SIZE / 2.0;
    let mut canvas: Canvas<CANVAS_PIXELS, CANVAS_PIXELS> = Canvas::new(255);
    let color = Color::red();
    let mut shape = Sphere::unit();
    const TRANSFORM: Matrix<4, 4> = Matrix::identity()
        .then(scaling(0.5, 1.0, 1.0))
        .then(rotation_z(PI / 6.0))
        .then(shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    shape.set_transform(&TRANSFORM);

    for y in 0..CANVAS_PIXELS - 1 {
        let world_y = HALF - PIXEL_SIZE * y as f32;
        for x in 0..CANVAS_PIXELS - 1 {
            let world_x = -HALF + PIXEL_SIZE * x as f32;
            let position = TupleKind::point(world_x, world_y, WALL_Z);

            let r = Ray::new(ray_origin, (position - ray_origin).normalize());
            let xs = shape.intersect(&r);

            match xs.hit() {
                Some(_) => canvas.set(color, y, x),
                None => (),
            }
        }
    }
    let filename = "chapter5.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P3);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(_) => println!("Succesfully written {filename}!"),
    }
}
fn chapter4() -> Result<(), ()> {
    const WIDTH: usize = 400;
    const HEIGHT: usize = 400;

    let mut c: Canvas<WIDTH, HEIGHT> = Canvas::new(255);

    // 12 o'clock position
    const SCALE: Matrix<4, 4> = scaling(150.0, 150.0, 150.0);
    const TRANSLATE: Matrix<4, 4> = translation(200.0, 200.0, 200.0);
    const START: TupleKind = TupleKind::point(0.0, 0.0, 1.0);

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
        c.set(Color::white(), x, z);
    }
    let filename = "chapter4.ppm";
    let result = c.write_ppm(filename, PpmFormat::P3);
    match result {
        Err(_) => println!("Something went wrong"),
        Ok(_) => println!("Succesfully written {filename}!"),
    }

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

    let mut canvas: Canvas<HEIGHT, WIDTH> = Canvas::new(255);

    for position in positions {
        let y = position.y().round().clamp(0.0, (HEIGHT - 1) as f32) as usize;
        let x = position.x().round().clamp(0.0, (WIDTH - 1) as f32) as usize;
        canvas.set(Color::white(), HEIGHT - y, x);
    }

    let filename = "chapter1.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);

    match result {
        Ok(_) => println!("Succesfully created {}!", filename),
        Err(err) => println!("{:?}", err),
    }

    Ok(())
}
