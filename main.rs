#![feature(generic_const_exprs)]
#![feature(f128)]
#![allow(incomplete_features)]
mod canvas;

use canvas::*;

mod bounds;
mod groups;
use groups::*;
mod cones;
use cones::*;
mod cylinders;
use cylinders::*;
mod cubes;
mod patterns;
use patterns::*;
mod planes;
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
mod rays;
mod spheres;
use rays::*;
mod transformations;
use transformations::*;
mod matrices;
use matrices::*;
mod tuples;
use tuples::*;

use std::time::Instant;

fn main() -> Result<(), ()> {
    // `cargo run --release -- fly` opens an interactive window you can fly
    // through, instead of rendering the chapters to files.
    if std::env::args().any(|a| a == "fly") {
        flythrough();
        return Ok(());
    }
    let _ = chapter1();
    let _ = chapter4();
    let _ = chapter5();
    let _ = chapter6();
    let _ = chapter7();
    let _ = chapter9();
    let _ = chapter10();
    let _ = chapter11();
    let _ = chapter12();
    let _ = chapter13();
    let _ = chapter14();
    let _ = chapter15();
    Ok(())
}
// A field of 280 glass and metal marbles on a reflective floor. The marbles are
// organized into a two-level group hierarchy (one group per row, all rows under
// one parent) so that bounding-box culling has structure to exploit: a ray that
// misses a row's box skips its 20 marbles at once, and a background ray that
// misses the whole field skips all 280. The scene is rendered twice, with
// culling off and on, to show the speedup the bounding boxes buy.
fn chapter15() {
    // Output resolution for the marble demo. The benchmark and the saved image use
    // the same camera, so the printed timings describe exactly the image written.
    const MARBLES_HSIZE: usize = 1200;
    const MARBLES_VSIZE: usize = 900;

    println!("chapter15: building a field of 280 marbles...");
    let world = build_marbles_world();

    let mut camera: Camera<MARBLES_HSIZE, MARBLES_VSIZE> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 4.2,
            z: -10.5,
        },
        Point {
            x: 0.0,
            y: 0.3,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));

    // Same world, same camera, bounding-box culling disabled: every ray (and
    // every reflection, refraction and shadow ray) tests all 280 marbles.
    let mut naive_world = world.clone();
    naive_world.use_bounds = false;
    println!("chapter15: rendering {MARBLES_HSIZE}x{MARBLES_VSIZE} without bounding boxes...");
    let start = Instant::now();
    let _ = camera.render_par(naive_world);
    let naive = start.elapsed();
    println!("chapter15:   without bounding boxes: {naive:.2?}");

    // Same scene with culling on. This is the image we keep.
    println!("chapter15: rendering {MARBLES_HSIZE}x{MARBLES_VSIZE} with bounding boxes...");
    let start = Instant::now();
    let canvas = camera.render_par(world);
    let bvh = start.elapsed();
    println!("chapter15:   with bounding boxes:    {bvh:.2?}");
    println!(
        "chapter15: speedup {:.1}x",
        naive.as_secs_f64() / bvh.as_secs_f64()
    );
    let filename = "chapter15.ppm";
    match canvas.write_ppm(filename, PpmFormat::P6) {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}

// Internal render resolution + recursion depth for the interactive viewer. Kept
// small so it stays real-time; the window scales it up. Tune to taste.
const LIVE_W: usize = 320;
const LIVE_H: usize = 180;
const LIVE_DEPTH: usize = 3;

// A selectable scene: a name, a builder, and a camera pose to start from.
struct Scene {
    name: &'static str,
    build: fn() -> World,
    pos: Point,
    yaw: Number,
    pitch: Number,
}

// An interactive fly-through in a single window: render a frame, blit it, read
// the keyboard, repeat. One process, one window for both display and input (via
// minifb), so there is no pipe, player, or terminal to coordinate. Number keys
// 1-4 switch scenes; closing the window or pressing Esc exits.
fn flythrough() {
    use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};

    const MOVE: Number = 0.35; // world units per frame while a move key is held
    const LOOK: Number = 0.04; // radians per frame while a look key is held

    // Each scene starts from a pose that frames it; fly freely from there.
    let scenes = [
        Scene {
            name: "marbles",
            build: build_marbles_world,
            pos: Point { x: 0.0, y: 4.0, z: -11.0 },
            yaw: 0.0,
            pitch: -0.25,
        },
        Scene {
            name: "capitol",
            build: build_capitol_world,
            pos: Point { x: 0.0, y: 5.0, z: -18.0 },
            yaw: 0.0,
            pitch: -0.08,
        },
        Scene {
            name: "hexagon",
            build: build_hexagon_world,
            pos: Point { x: 0.0, y: 2.5, z: -5.0 },
            yaw: 0.0,
            pitch: -0.25,
        },
        Scene {
            name: "glass",
            build: build_glass_world,
            pos: Point { x: 0.0, y: 1.5, z: -5.5 },
            yaw: 0.0,
            pitch: -0.08,
        },
    ];

    let mut camera: Camera<LIVE_W, LIVE_H> = Camera::new(PI / 3.0);
    let title = |name: &str| {
        format!("rusttracer [{name}] - 1-4 scene, N next, WASD/RF move, arrows look, Esc quit")
    };
    let mut window = Window::new(
        &title(scenes[0].name),
        LIVE_W,
        LIVE_H,
        WindowOptions {
            scale: Scale::X4, // 320x180 render shown in a 1280x720 window
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");

    let mut current = 0usize;
    let mut world = (scenes[current].build)();
    let mut pos = scenes[current].pos;
    let mut yaw = scenes[current].yaw;
    let mut pitch = scenes[current].pitch;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Scene switching: digits 1-4 pick a scene, N cycles to the next. Each
        // switch rebuilds the world and resets the camera to that scene's pose.
        let digit_keys = [Key::Key1, Key::Key2, Key::Key3, Key::Key4];
        let mut next = None;
        for (i, key) in digit_keys.iter().enumerate() {
            if i < scenes.len() && window.is_key_pressed(*key, KeyRepeat::No) {
                next = Some(i);
            }
        }
        if window.is_key_pressed(Key::N, KeyRepeat::No) {
            next = Some((current + 1) % scenes.len());
        }
        if let Some(i) = next {
            current = i;
            world = (scenes[current].build)();
            pos = scenes[current].pos;
            yaw = scenes[current].yaw;
            pitch = scenes[current].pitch;
            window.set_title(&title(scenes[current].name));
        }

        let fwd = forward(yaw, pitch);
        let right = Vector {
            x: yaw.cos(),
            y: 0.0,
            z: -yaw.sin(),
        };
        if window.is_key_down(Key::W) {
            pos = pos + fwd * MOVE;
        }
        if window.is_key_down(Key::S) {
            pos = pos + fwd * -MOVE;
        }
        if window.is_key_down(Key::A) {
            pos = pos + right * -MOVE;
        }
        if window.is_key_down(Key::D) {
            pos = pos + right * MOVE;
        }
        if window.is_key_down(Key::R) {
            pos.y += MOVE;
        }
        if window.is_key_down(Key::F) {
            pos.y -= MOVE;
        }
        if window.is_key_down(Key::Left) {
            yaw -= LOOK;
        }
        if window.is_key_down(Key::Right) {
            yaw += LOOK;
        }
        if window.is_key_down(Key::Up) {
            pitch += LOOK;
        }
        if window.is_key_down(Key::Down) {
            pitch -= LOOK;
        }
        pitch = pitch.clamp(-1.5, 1.5);

        let fwd = forward(yaw, pitch);
        camera.set_transform(view_transform(
            pos,
            pos + fwd,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ));
        let canvas = camera.render_live(&world, LIVE_DEPTH);
        if window
            .update_with_buffer(&canvas.to_argb(), LIVE_W, LIVE_H)
            .is_err()
        {
            break; // window closed
        }
    }
}

// Forward (look) direction from yaw (around +y) and pitch. yaw = pitch = 0 looks
// toward +z, which is where the field sits from the default start position.
fn forward(yaw: Number, pitch: Number) -> Vector {
    Vector {
        x: pitch.cos() * yaw.sin(),
        y: pitch.sin(),
        z: pitch.cos() * yaw.cos(),
    }
}

fn build_marbles_world() -> World {
    let mut world = World::new();
    world.lights = vec![Light::Point(PointLight::new(
        Point {
            x: -9.0,
            y: 11.0,
            z: -9.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    ))];

    // A large sphere lit purely by ambient acts as a soft sky, giving the glass
    // and metal marbles something colorful to refract and reflect. It is a
    // top-level object, so it is always tested and is not part of the grid.
    let mut sky = Shape::sphere();
    let mut sky_material = Material::default();
    sky_material.set_color(Color {
        r: 0.55,
        g: 0.7,
        b: 0.95,
    });
    sky_material.set_ambient(0.7);
    sky_material.set_diffuse(0.0);
    sky_material.set_specular(0.0);
    sky.set_material(sky_material);
    sky.set_transform(scaling(1000.0, 1000.0, 1000.0));
    world.add_object(sky);

    // A reflective checkered floor that doubles the marbles in reflection.
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    let mut floor_pattern = Pattern::checker_pattern(
        Color {
            r: 0.18,
            g: 0.18,
            b: 0.2,
        },
        Color {
            r: 0.32,
            g: 0.32,
            b: 0.34,
        },
    );
    floor_pattern.set_transform(scaling(0.75, 0.75, 0.75));
    floor_material.set_pattern(floor_pattern);
    floor_material.set_specular(0.0);
    floor_material.set_reflective(0.35);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Clear glass for half the marbles.
    let glass = Material::glass();

    // A palette of polished metals for the other half: gold, copper, silver and
    // a cool blue steel. Metal is mostly reflection with a tight specular
    // highlight and little diffuse.
    let metal = |color: Color| {
        let mut m = Material::default();
        m.set_color(color);
        m.set_ambient(0.05);
        m.set_diffuse(0.15);
        m.set_specular(1.0);
        m.set_shininess(300.0);
        m.set_reflective(0.85);
        m
    };
    let metals = [
        metal(Color {
            r: 1.0,
            g: 0.78,
            b: 0.34,
        }),
        metal(Color {
            r: 0.95,
            g: 0.55,
            b: 0.35,
        }),
        metal(Color {
            r: 0.95,
            g: 0.95,
            b: 0.97,
        }),
        metal(Color {
            r: 0.5,
            g: 0.6,
            b: 0.78,
        }),
    ];

    const COLS: usize = 20;
    const ROWS: usize = 14; // COLS * ROWS == 280 marbles
    const RADIUS: Number = 0.33;
    const SPACING: Number = 0.75;

    // One parent group holds one sub-group per row; each marble is a child of
    // its row group. compute_bounds() then gives every group a box.
    let grid = world.add_object(Shape::group());
    let mut metal_index = 0;
    for row in 0..ROWS {
        let row_group = world.add_child(grid, Shape::group());
        let z = (row as Number - (ROWS as Number - 1.0) / 2.0) * SPACING;
        for col in 0..COLS {
            let x = (col as Number - (COLS as Number - 1.0) / 2.0) * SPACING;
            let mut marble = Shape::sphere();
            marble.set_transform(scaling(RADIUS, RADIUS, RADIUS).then(translation(x, RADIUS, z)));
            // Checkerboard of glass and metal across the grid.
            if (row + col) % 2 == 0 {
                marble.set_material(glass.clone());
            } else {
                marble.set_material(metals[metal_index % metals.len()].clone());
                metal_index += 1;
            }
            world.add_child(row_group, marble);
        }
    }

    // Build the bounding boxes once, now that the scene is complete.
    world.compute_bounds();
    world
}
// A hexagon assembled from groups, following the book's chapter 14 example.
// Each of the six sides is its own group holding a spherical corner and a
// cylindrical edge; all six sides are children of one parent group, which is
// tilted and lifted so the whole ring faces the camera.
fn build_hexagon_world() -> World {
    let mut world = World::new();
    world.lights = vec![Light::Point(PointLight::new(
        Point {
            x: -10.0,
            y: 10.0,
            z: -10.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    ))];

    // A reflective floor so the hexagon casts and catches a little light.
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    floor_material.set_color(Color {
        r: 0.9,
        g: 0.9,
        b: 0.9,
    });
    floor_material.set_specular(0.0);
    floor_material.set_reflective(0.1);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Shared material for every corner and edge of the hexagon.
    let mut material = Material::default();
    material.set_color(Color {
        r: 0.9,
        g: 0.2,
        b: 0.3,
    });
    material.set_diffuse(0.7);
    material.set_specular(0.3);
    material.set_shininess(150.0);
    material.set_reflective(0.1);

    // A corner is a small sphere; an edge is a unit-tall cylinder rotated into
    // place. Both are built in the side group's local space. Transform order
    // reads as "apply self first, then each .then(...)".
    let corner = || {
        let mut c = Shape::sphere();
        c.set_transform(scaling(0.25, 0.25, 0.25).then(translation(0.0, 0.0, -1.0)));
        c.set_material(material.clone());
        c
    };
    let edge = || {
        let mut e = Shape::Cylinder(Cylinder::new(0.0, 1.0, false));
        e.set_transform(
            scaling(0.25, 1.0, 0.25)
                .then(rotation_z(-PI / 2.0))
                .then(rotation_y(-PI / 6.0))
                .then(translation(0.0, 0.0, -1.0)),
        );
        e.set_material(material.clone());
        e
    };

    // The parent group: tilt the ring forward and lift it above the floor.
    let mut hex = Shape::group();
    hex.set_transform(rotation_x(-PI / 6.0).then(translation(0.0, 1.0, 0.0)));
    let hex = world.add_object(hex);

    // Six sides, each a group rotated a sixth of a turn around y.
    for n in 0..6 {
        let mut side = Shape::group();
        side.set_transform(rotation_y(n as Number * PI / 3.0));
        let side = world.add_child(hex, side);
        world.add_child(side, corner());
        world.add_child(side, edge());
    }

    world.compute_bounds();
    world
}
fn chapter14() {
    let world = build_hexagon_world();

    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 2.5,
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
    let filename = "chapter14.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
// A model of the US Capitol building, assembled entirely from the ray
// tracer's primitives: planes, cubes, cylinders, a sphere (the dome) and a
// cone (the spire under the Statue of Freedom). The building faces -z, toward
// the camera.
fn build_capitol_world() -> World {
    let mut world = World::new();
    // Two lights now that the world supports a Vec<Light>: a bright warm "sun"
    // from the front-left, and a dim cool "sky" fill from the right that lifts
    // the shadowed faces without erasing the shadows.
    world.lights = vec![
        Light::Point(PointLight::new(
            Point {
                x: -7.0,
                y: 14.0,
                z: -14.0,
            },
            Color {
                r: 1.0,
                g: 0.98,
                b: 0.92,
            },
        )),
        Light::Point(PointLight::new(
            Point {
                x: 14.0,
                y: 7.0,
                z: -10.0,
            },
            Color {
                r: 0.3,
                g: 0.36,
                b: 0.45,
            },
        )),
    ];

    // Marble: the warm white stone used for most of the building. Soft sheen
    // and a faint reflectivity give it a polished-stone feel without turning
    // it into a mirror.
    let mut marble = Material::default();
    marble.set_color(Color {
        r: 0.93,
        g: 0.92,
        b: 0.87,
    });
    marble.set_ambient(0.12);
    marble.set_diffuse(0.8);
    marble.set_specular(0.25);
    marble.set_shininess(120.0);
    marble.set_reflective(0.04);

    // Painted cast iron of the dome: a touch more polished than the stone.
    let mut dome_iron = marble.clone();
    dome_iron.set_color(Color {
        r: 0.95,
        g: 0.95,
        b: 0.93,
    });
    dome_iron.set_specular(0.45);
    dome_iron.set_shininess(200.0);
    dome_iron.set_reflective(0.08);

    // Ground: a polished checkered stone plaza that faintly reflects the
    // building.
    let mut ground = Shape::plane();
    let mut ground_material = Material::default();
    let mut plaza = Pattern::checker_pattern(
        Color {
            r: 0.82,
            g: 0.82,
            b: 0.80,
        },
        Color {
            r: 0.52,
            g: 0.52,
            b: 0.52,
        },
    );
    plaza.set_transform(scaling(1.5, 1.5, 1.5));
    ground_material.set_pattern(plaza);
    ground_material.set_ambient(0.12);
    ground_material.set_diffuse(0.8);
    ground_material.set_specular(0.2);
    ground_material.set_shininess(120.0);
    ground_material.set_reflective(0.2);
    ground.set_material(ground_material);

    // A large enclosing sphere acts as a daytime sky. It is lit purely by its
    // high ambient term, so it glows an even blue regardless of the light, and
    // the reflective plaza and dome pick up its color.
    let mut sky = Shape::sphere();
    let mut sky_material = Material::default();
    sky_material.set_color(Color {
        r: 0.55,
        g: 0.75,
        b: 1.0,
    });
    sky_material.set_ambient(0.6);
    sky_material.set_diffuse(0.0);
    sky_material.set_specular(0.0);
    sky.set_material(sky_material);
    sky.set_transform(scaling(1000.0, 1000.0, 1000.0));

    let mut objects: Vec<Shape> = vec![sky, ground];

    // Main facade: a long, low block spanning the full width.
    let mut base = Shape::cube();
    base.set_material(marble.clone());
    base.set_transform(scaling(6.0, 1.2, 2.0).then(translation(0.0, 1.2, 0.0)));
    objects.push(base);

    // The two end wings (House and Senate), raised slightly above the facade.
    for sign in [-1.0, 1.0] {
        let mut wing = Shape::cube();
        wing.set_material(marble.clone());
        wing.set_transform(scaling(1.3, 1.4, 2.0).then(translation(sign * 4.5, 1.4, 0.0)));
        objects.push(wing);
    }

    // Central block that lifts the rotunda above the facade.
    let mut center = Shape::cube();
    center.set_material(marble.clone());
    center.set_transform(scaling(2.0, 1.0, 2.0).then(translation(0.0, 3.4, 0.0)));
    objects.push(center);

    // The rotunda drum: a closed cylinder carrying the dome.
    let mut drum = Shape::Cylinder(Cylinder::new(0.0, 1.0, true));
    drum.set_material(marble.clone());
    drum.set_transform(scaling(1.5, 1.3, 1.5).then(translation(0.0, 4.4, 0.0)));
    objects.push(drum);

    // The dome: a sphere scaled tall, sitting on the drum.
    let mut dome = Shape::sphere();
    dome.set_material(dome_iron);
    dome.set_transform(scaling(1.5, 1.8, 1.5).then(translation(0.0, 5.7, 0.0)));
    objects.push(dome);

    // The lantern/cupola: a small closed cylinder atop the dome.
    let mut lantern = Shape::Cylinder(Cylinder::new(0.0, 1.0, true));
    lantern.set_material(marble.clone());
    lantern.set_transform(scaling(0.35, 0.6, 0.35).then(translation(0.0, 7.3, 0.0)));
    objects.push(lantern);

    // The Statue of Freedom: a bronze cone tapering to a point. Polished
    // metal: dark diffuse, strong specular highlight and real reflectivity.
    let mut statue = Shape::Cone(Cone::new(-1.0, 0.0, true));
    let mut statue_material = Material::default();
    statue_material.set_color(Color {
        r: 0.55,
        g: 0.41,
        b: 0.16,
    });
    statue_material.set_ambient(0.2);
    statue_material.set_diffuse(0.6);
    statue_material.set_specular(0.9);
    statue_material.set_shininess(250.0);
    statue_material.set_reflective(0.35);
    statue.set_material(statue_material);
    statue.set_transform(scaling(0.22, 0.7, 0.22).then(translation(0.0, 8.6, 0.0)));
    objects.push(statue);

    // The east front colonnade: a row of columns under a pediment.
    let column_xs = [-2.4, -1.6, -0.8, 0.0, 0.8, 1.6, 2.4];
    for x in column_xs {
        let mut column = Shape::Cylinder(Cylinder::new(0.0, 1.0, true));
        column.set_material(marble.clone());
        column.set_transform(scaling(0.18, 2.4, 0.18).then(translation(x, 0.0, -2.1)));
        objects.push(column);
    }

    // The pediment resting on the columns.
    let mut pediment = Shape::cube();
    pediment.set_material(marble.clone());
    pediment.set_transform(scaling(2.8, 0.18, 0.35).then(translation(0.0, 2.6, -2.1)));
    objects.push(pediment);

    world.objects = objects;
    world
}
fn chapter13() {
    let world = build_capitol_world();

    let mut camera: Camera<1000, 1000> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 0.0,
            y: 5.0,
            z: -17.0,
        },
        Point {
            x: 0.0,
            y: 3.5,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    let canvas = camera.render_par(world);
    let filename = "chapter13.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}
fn chapter12() {
    let mut world = World::new();
    world.lights = vec![Light::Point(PointLight::new(
        Point {
            x: -10.0,
            y: 10.0,
            z: -10.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    ))];
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
    // Cube
    let mut cube = Shape::cube();
    let mut cube_material = Material::default();
    cube_material.set_color(Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
    });
    cube.set_material(cube_material);
    cube.set_transform(translation(0.0, 1.0, 0.0) * rotation_y(15.0));
    world.objects = vec![floor, wall, cube];
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
    let filename = "chapter12.ppm";
    let result = canvas.write_ppm(filename, PpmFormat::P6);
    match result {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}

fn build_glass_world() -> World {
    let mut world = World::new();

    // Floor - glass material
    let mut floor = Shape::plane();
    floor.set_transform(scaling(10.0, 0.01, 10.0));
    let mut floor_material = Material::default();
    floor_material.set_transparency(0.9);
    floor_material.set_reflective(0.9);
    floor_material.set_diffuse(0.1);
    floor_material.set_ambient(0.1);
    floor_material.set_specular(1.0);
    floor_material.set_shininess(300.0);
    floor_material.set_color(Color {
        r: 1.0,
        g: 1.0,
        b: 0.9,
    });
    floor.set_material(floor_material);

    // Middle sphere
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
    middle_material.set_reflective(0.3);
    middle.set_material(middle_material);

    // Right sphere
    let mut right = Shape::sphere();
    right.set_transform(scaling(0.5, 0.5, 0.5).then(translation(1.5, 0.5, -0.5)));
    let mut right_material = Material::default();
    right_material.set_color(Color {
        r: 0.5,
        g: 1.0,
        b: 0.1,
    });
    right_material.set_diffuse(0.7);
    right_material.set_specular(0.3);
    right.set_material(right_material);

    // Left sphere
    let mut left = Shape::sphere();
    left.set_transform(scaling(0.33, 0.33, 0.33).then(translation(-1.5, 0.33, -0.75)));
    let left_material = Material::glass();
    /*
    left_material.set_color(Color {
    r: 1.0,
    g: 0.8,
    b: 0.1,
    });
    left_material.set_diffuse(0.7);
    left_material.set_specular(0.3);
    */
    left.set_material(left_material);
    world.objects = vec![floor, middle, right, left];

    world.lights = vec![Light::Point(PointLight::new(
        Point {
            x: -10.0,
            y: 10.0,
            z: -10.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    ))];
    world
}
fn chapter11() {
    let world = build_glass_world();

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
    world.lights = vec![light];

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
    world.lights = vec![light];

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
    let pixel_size = wall_size / CANVAS_PIXELS as Number;
    let half = wall_size / 2.0;
    let mut canvas: Canvas<CANVAS_PIXELS, CANVAS_PIXELS> = Canvas::new(255);
    for y in 0..CANVAS_PIXELS {
        let world_y = half - pixel_size * y as Number;
        for x in 0..CANVAS_PIXELS {
            let world_x = -half + pixel_size * x as Number;
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
    const WALL_Z: Number = 10.0;
    const WALL_SIZE: Number = 7.0;
    const CANVAS_PIXELS: usize = 100;
    const PIXEL_SIZE: Number = WALL_SIZE / CANVAS_PIXELS as Number;
    const HALF: Number = WALL_SIZE / 2.0;
    let mut canvas: Canvas<CANVAS_PIXELS, CANVAS_PIXELS> = Canvas::new(255);
    let color = Pixel::red();
    let mut shape = Shape::sphere();
    const TRANSFORM: Matrix<4, 4> = Matrix::identity()
        .then(scaling(0.5, 1.0, 1.0))
        .then(rotation_z(PI / 6.0))
        .then(shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0));

    shape.set_transform(TRANSFORM);

    for y in 0..CANVAS_PIXELS - 1 {
        let world_y = HALF - PIXEL_SIZE * y as Number;
        for x in 0..CANVAS_PIXELS - 1 {
            let world_x = -HALF + PIXEL_SIZE * x as Number;
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
    const STEP_SIZE: Number = 360.0 / HOURS_COUNT as Number;
    const HOURS: [Matrix<4, 4>; HOURS_COUNT] = {
        let mut hours: [Matrix<4, 4>; HOURS_COUNT] = [Matrix::identity(); HOURS_COUNT];
        let mut i = 0;
        while i < HOURS_COUNT {
            let angle = radians((i as Number) * STEP_SIZE);
            hours[i] = rotation_y(angle).then(SCALE).then(TRANSLATE);
            i += 1;
        }
        hours
    };

    // Iterate over compile-time calculated transform matrix
    for transform in HOURS.iter() {
        let p = *transform * start;
        let x = p.x().round().clamp(0.0, (WIDTH - 1) as Number) as usize;
        let z = p.z().round().clamp(0.0, (HEIGHT - 1) as Number) as usize;
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
        let y = position.y().round().clamp(0.0, (HEIGHT - 1) as Number) as usize;
        let x = position.x().round().clamp(0.0, (WIDTH - 1) as Number) as usize;
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
