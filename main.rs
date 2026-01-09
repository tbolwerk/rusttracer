#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
mod canvas;
use std::str::Matches;

use canvas::*;

mod patterns;
use patterns::*;
mod planes;
use planes::*;
mod shapes;
use shapes::*;
mod camera;
use camera::*;
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

fn main() -> Result<(), ()> {
    let _ = chapter1();
    let _ = chapter4();
    let _ = chapter5();
    let _ = chapter6();
    let _ = chapter7();
    let _ = chapter9();
    let _ = chapter10();
    let _ = chapter11();
    Ok(())
}

fn chapter11() {
    let mut world = World::default();
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    floor_material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);

    let middle = Shape::with(
        Shape::glass_sphere,
        translation(-0.5, 1.0, 0.5),
        Material::glass(),
    );
    let mut right = Shape::glass_sphere();
    const RIGHT_TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5).then(translation(1.5, 0.5, -0.5));
    right.set_transform(RIGHT_TRANSFORM);
    let mut left = Shape::glass_sphere();
    const LEFT_TRANSFORMATION: Matrix<4, 4> =
        scaling(0.33, 0.33, 0.33).then(translation(-1.5, 0.33, -0.75));
    left.set_transform(LEFT_TRANSFORMATION);

    world.objects = vec![floor, middle, right, left];

    let light_position = Point {
        x: -10.0,
        y: 10.0,
        z: -10.0,
    };
    let light_color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    let light = Light::Point(PointLight::new(light_position, light_color));
    world.light = Some(light);

    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 1.5,
            z: -5.0,
        },
        Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    let canvas = camera.render_par(world);
    let filename = "chapter11.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
fn chapter10() {
    let mut world = World::default();
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    let pattern = Pattern::ring_pattern(
        Color {
            r: 0.9,
            g: 0.3,
            b: 0.1,
        },
        Color {
            r: 0.1,
            g: 0.7,
            b: 0.9,
        },
    );
    floor_material.set_pattern(pattern.clone());
    floor_material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);
    let mut wall = Shape::plane();
    wall.set_transform(
        Matrix::identity()
            .then(rotation_x(PI / 2.0))
            .then(rotation_y(-PI / 6.0))
            .then(translation(0.0, 0.0, 5.0)),
    );
    let mut wall_material = Material::default();
    wall_material.set_pattern(Pattern::stripe_pattern(
        Color {
            r: 0.5,
            g: 0.0,
            b: 0.0,
        },
        Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
        },
    ));
    wall.set_material(wall_material);

    let mut middle = Shape::sphere();
    middle.set_transform(translation(-0.5, 1.0, 0.5));
    let mut middle_material = Material::default();
    middle_material.set_pattern(Pattern::checker_pattern(
        Color {
            r: 0.0,
            g: 0.3,
            b: 0.7,
        },
        Color {
            r: 0.5,
            g: 0.0,
            b: 0.2,
        },
    ));
    middle_material.set_color(Color {
        r: 0.1,
        g: 1.0,
        b: 0.5,
    });
    middle_material.set_diffuse(0.7);
    middle_material.set_specular(0.3);
    middle.set_material(middle_material);

    let mut right = Shape::sphere();
    const RIGHT_TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5).then(translation(1.5, 0.5, -0.5));
    right.set_transform(RIGHT_TRANSFORM);
    let mut right_material = Material::default();
    right_material.set_pattern(Pattern::gradient_pattern(
        Color {
            r: 0.3,
            g: 0.3,
            b: 0.0,
        },
        Color {
            r: 0.7,
            g: 0.7,
            b: 1.0,
        },
    ));
    right_material.set_color(Color {
        r: 0.5,
        g: 1.0,
        b: 0.1,
    });
    right_material.set_diffuse(0.7);
    right_material.set_specular(0.3);
    right.set_material(right_material);

    let mut left = Shape::sphere();
    const LEFT_TRANSFORMATION: Matrix<4, 4> =
        scaling(0.33, 0.33, 0.33).then(translation(-1.5, 0.33, -0.75));
    left.set_transform(LEFT_TRANSFORMATION);
    let mut left_material = Material::default();
    left_material.set_pattern(pattern.clone());
    left_material.set_color(Color {
        r: 1.0,
        g: 0.8,
        b: 0.1,
    });
    left_material.set_diffuse(0.7);
    left_material.set_specular(0.3);
    left.set_material(left_material);

    world.objects = vec![floor, middle, right, left, wall];

    let light_position = Point {
        x: 5.0,
        y: 5.0,
        z: -5.0,
    };
    let light_color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    let light = Light::Point(PointLight::new(light_position, light_color));
    world.light = Some(light);

    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 1.5,
            z: -5.0,
        },
        Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    let canvas = camera.render_par(world);
    let filename = "chapter10.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
fn chapter9() {
    let mut world = World::default();
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    floor_material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);

    let mut middle = Shape::sphere();
    middle.set_transform(translation(-0.5, 1.0, 0.5));
    let mut middle_material = Material::default();
    middle_material.set_color(Color {
        r: 0.1,
        g: 1.0,
        b: 0.5,
    });
    middle_material.set_diffuse(0.7);
    middle_material.set_specular(0.3);
    middle.set_material(middle_material);

    let mut right = Shape::sphere();
    const RIGHT_TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5).then(translation(1.5, 0.5, -0.5));
    right.set_transform(RIGHT_TRANSFORM);
    let mut right_material = Material::default();
    right_material.set_color(Color {
        r: 0.5,
        g: 1.0,
        b: 0.1,
    });
    right_material.set_diffuse(0.7);
    right_material.set_specular(0.3);
    right.set_material(right_material);

    let mut left = Shape::sphere();
    const LEFT_TRANSFORMATION: Matrix<4, 4> =
        scaling(0.33, 0.33, 0.33).then(translation(-1.5, 0.33, -0.75));
    left.set_transform(LEFT_TRANSFORMATION);
    let mut left_material = Material::default();
    left_material.set_color(Color {
        r: 1.0,
        g: 0.8,
        b: 0.1,
    });
    left_material.set_diffuse(0.7);
    left_material.set_specular(0.3);
    left.set_material(left_material);

    world.objects = vec![floor, middle, right, left];

    let light_position = Point {
        x: -10.0,
        y: 10.0,
        z: -10.0,
    };
    let light_color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    let light = Light::Point(PointLight::new(light_position, light_color));
    world.light = Some(light);

    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 1.5,
            z: -5.0,
        },
        Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    let canvas = camera.render_par(world);
    let filename = "chapter9.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}

fn chapter7() {
    let mut floor = Shape::sphere();
    floor.set_transform(scaling(10.0, 0.01, 10.0));
    let mut material = Material::default();
    material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    material.set_specular(0.0);
    floor.set_material(material.clone());

    let mut left_wall = Shape::sphere();
    const LEFT_WALL_TRANSFORMATION: Matrix<4, 4> = scaling(10.0, 0.01, 10.0)
        .then(rotation_x(PI / 2.0))
        .then(rotation_y(-PI / 4.0))
        .then(translation(0.0, 0.0, 5.0));
    left_wall.set_transform(LEFT_WALL_TRANSFORMATION);
    left_wall.set_material(material);

    let mut right_wall = Shape::sphere();
    const RIGHT_WALL_TRANSFORMATION: Matrix<4, 4> = scaling(10.0, 0.01, 10.0)
        .then(rotation_x(PI / 2.0))
        .then(rotation_y(PI / 4.0))
        .then(translation(0.0, 0.0, 5.0));
    right_wall.set_transform(RIGHT_WALL_TRANSFORMATION);

    let mut middle = Shape::sphere();
    middle.set_transform(translation(-0.5, 1.0, 0.5));
    let mut middle_material = Material::default();
    middle_material.set_color(Color {
        r: 0.1,
        g: 1.0,
        b: 0.5,
    });
    middle_material.set_diffuse(0.7);
    middle_material.set_specular(0.3);
    middle.set_material(middle_material);

    let mut right = Shape::sphere();
    const RIGHT_TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5).then(translation(1.5, 0.5, -0.5));
    right.set_transform(RIGHT_TRANSFORM);
    let mut right_material = Material::default();
    right_material.set_color(Color {
        r: 0.5,
        g: 1.0,
        b: 0.1,
    });
    right_material.set_diffuse(0.7);
    right_material.set_specular(0.3);
    right.set_material(right_material);

    let mut left = Shape::sphere();
    const LEFT_TRANSFORMATION: Matrix<4, 4> =
        scaling(0.33, 0.33, 0.33).then(translation(-1.5, 0.33, -0.75));
    left.set_transform(LEFT_TRANSFORMATION);
    let mut left_material = Material::default();
    left_material.set_color(Color {
        r: 1.0,
        g: 0.8,
        b: 0.1,
    });
    left_material.set_diffuse(0.7);
    left_material.set_specular(0.3);
    left.set_material(left_material);

    let mut world = World::default();
    world.objects = vec![floor, left_wall, right_wall, middle, right, left];
    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 1.5,
            z: -5.0,
        },
        Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    let canvas = camera.render_par(world);
    let filename = "chapter7.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
fn chapter6() {
    let mut sphere = Shape::sphere();
    let mut material = Material::default();
    let color = Color {
        r: 1.0,
        g: 0.2,
        b: 1.0,
    };
    material.set_color(color);
    sphere.set_material(material);

    const TRANSFORM: Matrix<4, 4> = Matrix::identity().then(scaling(0.1, 0.1, 0.1));
    // .then(rotation_z(PI / 6.0))
    //.then(shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    sphere.set_transform(TRANSFORM);

    let light_position = Point {
        x: -10.0,
        y: 10.0,
        z: -10.0,
    };
    let light_color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    let light = Light::Point(PointLight::new(light_position, light_color));

    let ray_origin = Point {
        x: 0.0,
        y: 0.0,
        z: -0.5,
    };
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
            let current_position = Point {
                x: world_x,
                y: world_y,
                z: wall_z,
            };
            let ray = Ray {
                origin: ray_origin,
                direction: (current_position - ray_origin).normalize(),
            };
            let xs = sphere.intersect(&ray, 0);
            match xs.hit() {
                None => (),
                Some(hit) => {
                    let point = ray.position(hit.t);
                    let normal = sphere.normal_at(&point);
                    let eye = -ray.direction;

                    let color = lightning(&sphere, light.clone(), point, eye, normal, false);
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
    let ray_origin = Point {
        x: 0.0,
        y: 0.0,
        z: -5.0,
    };
    const WALL_Z: f32 = 10.0;
    const WALL_SIZE: f32 = 7.0;
    const CANVAS_PIXELS: usize = 100;
    const PIXEL_SIZE: f32 = WALL_SIZE / CANVAS_PIXELS as f32;
    const HALF: f32 = WALL_SIZE / 2.0;
    let mut canvas: Canvas<CANVAS_PIXELS, CANVAS_PIXELS> = Canvas::new(255);
    let color = Pixel::red();
    let mut shape = Shape::sphere();
    const TRANSFORM: Matrix<4, 4> = Matrix::identity()
        .then(scaling(0.5, 1.0, 1.0))
        .then(rotation_z(PI / 6.0))
        .then(shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    shape.set_transform(TRANSFORM);

    for y in 0..CANVAS_PIXELS - 1 {
        let world_y = HALF - PIXEL_SIZE * y as f32;
        for x in 0..CANVAS_PIXELS - 1 {
            let world_x = -HALF + PIXEL_SIZE * x as f32;
            let position = Point {
                x: world_x,
                y: world_y,
                z: WALL_Z,
            };

            let r = Ray {
                origin: ray_origin,
                direction: (position - ray_origin).normalize(),
            };
            let xs = shape.intersect(&r, 0);

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
    let start = Point {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

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
        let p = *transform * start;
        let x = p.x().round().clamp(0.0, (WIDTH - 1) as f32) as usize;
        let z = p.z().round().clamp(0.0, (HEIGHT - 1) as f32) as usize;
        c.set(Pixel::white(), x, z);
    }
    let filename = "chapter4.ppm";
    let result = c.write_ppm(filename, PpmFormat::P3);
    match result {
        Err(_) => println!("Something went wrong"),
        Ok(_) => println!("Succesfully written {filename}!"),
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct Projectile {
    position: Point,
    velocity: Vector,
}

impl Projectile {
    fn new(position: Point, velocity: Vector) -> Self {
        Self { position, velocity }
    }
}

#[derive(Clone)]
struct Environment {
    gravity: Vector,
    wind: Vector,
}

impl Environment {
    fn new(gravity: Vector, wind: Vector) -> Self {
        Self { gravity, wind }
    }
}

fn tick(env: Environment, proj: Projectile) -> Projectile {
    let position = proj.position + proj.velocity;
    let velocity = proj.velocity + env.gravity + env.wind;
    Projectile::new(position, velocity)
}

fn chapter1() -> std::io::Result<()> {
    let start = Point {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let velocity = Vector {
        x: 1.0,
        y: 1.8,
        z: 0.0,
    }
    .normalize()
        * 11.25;
    let mut p = Projectile::new(start, velocity);
    let e = Environment::new(
        Vector {
            x: 0.0,
            y: -0.1,
            z: 0.0,
        },
        Vector {
            x: -0.01,
            y: 0.0,
            z: 0.0,
        },
    );

    let mut positions: Vec<Point> = vec![];
    while p.position.y() > 0.0 {
        positions.push(p.position);
        p = tick(e.clone(), p);
    }

    const WIDTH: usize = 900;
    const HEIGHT: usize = 550;

    let mut canvas: Canvas<HEIGHT, WIDTH> = Canvas::new(255);

    for position in positions {
        let y = position.y().round().clamp(0.0, (HEIGHT - 1) as f32) as usize;
        let x = position.x().round().clamp(0.0, (WIDTH - 1) as f32) as usize;
        canvas.set(Pixel::white(), HEIGHT - y, x);
    }

    let filename = "chapter1.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);

    match result {
        Ok(_) => println!("Succesfully created {}!", filename),
        Err(err) => println!("{:?}", err),
    }

    Ok(())
}
