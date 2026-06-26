#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

// The renderer + math now live in the shared `raycore` crate (single source of
// truth for CPU and, after Stage 3, the GPU shader). Re-export its modules so the
// binary's existing `crate::<module>` / unqualified paths keep resolving exactly
// as they did when these were local modules.
pub use raycore::{
    bounds, cones, csg, cubes, cylinders, groups, intersections, lights,
    materials, matrices, patterns, planes, rays, shapes, spheres, texture_maps,
    transformations, triangles, tuples, worlds,
};

// Reproduce the flat import surface the binary used before the split (only the
// modules that were previously glob-imported).
use csg::*;
use lights::*;
use materials::*;
use matrices::*;
use patterns::*;
use rays::*;
use shapes::*;
use texture_maps::*;
use transformations::*;
use tuples::*;
use worlds::*;

// `assert_almost_eq!` lived in the core's `tuples` module and reached the
// binary's camera/viewport tests through `use crate::tuples::*`. After the split
// it is crate-internal to raycore, so redefine it here. Defined before
// `mod camera/viewport` so textual macro scoping makes it visible inside them.
#[cfg(test)]
macro_rules! assert_almost_eq {
    ($a: expr, $b: expr) => {
        assert_almost_eq!($a, $b, 1e-5);
    };
    ($a: expr, $b: expr, $eps: expr) => {
        assert!(
            ($a - $b).abs() <= $eps,
            "assert_almost_eq failed: {:?} != {:?}",
            $a,
            $b
        );
    };
}

// Host-only modules: framebuffer, render driver, interactive viewport, OBJ
// loading and PPM/pixel output.
mod canvas;
use canvas::*;
mod colors;
use colors::*;
mod obj_parser;
mod camera;
use camera::*;
mod viewport;
use viewport::{Scene, Viewport, DISP_H, DISP_W, MOVE_DEPTH, STILL_DEPTH};

use std::time::Instant;

fn main() -> Result<(), ()> {
    // `cargo run --release -- fly` opens an interactive window you can fly
    // through, instead of rendering the chapters to files.
    if std::env::args().any(|a| a == "fly") {
        flythrough();
        return Ok(());
    }
    // `cargo run --release -- teapot` renders a single teapot.obj still.
    if std::env::args().any(|a| a == "teapot") {
        teapot();
        return Ok(());
    }
    // `cargo run --release -- csg` renders just the chapter 16 CSG widget.
    if std::env::args().any(|a| a == "csg") {
        chapter16();
        return Ok(());
    }
    // `cargo run --release -- bench` times single high-res frames of the live
    // scenes, with bounding-box culling on and off, as a CPU benchmark.
    if std::env::args().any(|a| a == "bench") {
        bench();
        return Ok(());
    }
    // `cargo run --release -- bonus` renders the combined bonus-chapters scene
    // (area light, texture mapping, focal blur, divide). Kept out of the default
    // chapter sweep because lens + area-light sampling makes it slow.
    if std::env::args().any(|a| a == "bonus") {
        chapter17();
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
    chapter16();
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

// The iconic CSG widget, which exercises all three operations in one object:
//   Difference( Intersection(cube, sphere), Union(3 cylinders) )
// The intersection of a cube and a slightly larger sphere is a cube with rounded
// edges; subtracting the union of three axis-aligned cylinders bores a hole
// straight through each axis. The rounded shell is red ceramic; the bored walls
// are polished gold, so the holes read clearly.
fn build_csg_world() -> World {
    let mut world = World::new();
    world.lights = vec![Light::point_light(
        Point {
            x: -8.0,
            y: 10.0,
            z: -8.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    )];

    // Ambient sky sphere for soft fill and reflections.
    let mut sky = Primitive::sphere();
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

    // Reflective checkered floor.
    let mut floor = Primitive::plane();
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
    floor_material.set_reflective(0.3);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Red ceramic for the rounded shell.
    let mut shell = Material::default();
    shell.set_color(Color {
        r: 0.8,
        g: 0.2,
        b: 0.2,
    });
    shell.set_diffuse(0.7);
    shell.set_specular(0.4);
    shell.set_shininess(180.0);
    shell.set_reflective(0.1);

    // Polished gold for the surfaces revealed inside the bored holes.
    let mut gold = Material::default();
    gold.set_color(Color {
        r: 1.0,
        g: 0.8,
        b: 0.3,
    });
    gold.set_ambient(0.1);
    gold.set_diffuse(0.4);
    gold.set_specular(0.9);
    gold.set_shininess(300.0);
    gold.set_reflective(0.4);

    // CSG nodes are created before their children so each parent keeps a lower
    // arena id than its children, which `compute_bounds` relies on.
    let hero = world.add_object(Primitive::csg(CsgOperation::Difference));

    // left = rounded cube = Intersection(cube, slightly larger sphere)
    let rounded = world.add_object(Primitive::csg(CsgOperation::Intersection));
    let mut cube = Primitive::cube();
    cube.set_material(shell.clone());
    let cube_id = world.add_object(cube);
    let mut sphere = Primitive::sphere();
    sphere.set_transform(scaling(1.3, 1.3, 1.3));
    sphere.set_material(shell.clone());
    let sphere_id = world.add_object(sphere);
    world.set_csg_children(rounded, cube_id, sphere_id);

    // right = drill = Union of three finite cylinders, one per axis. Finite (not
    // infinite) so the CSG bounding box stays finite and cullable.
    let drill = |transform: Matrix<4, 4>| {
        let mut c = {
        let mut __c = Primitive::cylinder();
        __c.minimum = -1.5;
        __c.maximum = 1.5;
        __c.closed = false;
        __c
    };
        c.set_transform(scaling(0.5, 1.0, 0.5).then(transform));
        c.set_material(gold.clone());
        c
    };
    let bore = world.add_object(Primitive::csg(CsgOperation::Union));
    let bore_xy = world.add_object(Primitive::csg(CsgOperation::Union));
    let cyl_y = world.add_object(drill(Matrix::identity()));
    let cyl_x = world.add_object(drill(rotation_z(PI / 2.0)));
    world.set_csg_children(bore_xy, cyl_y, cyl_x);
    let cyl_z = world.add_object(drill(rotation_x(PI / 2.0)));
    world.set_csg_children(bore, bore_xy, cyl_z);

    world.set_csg_children(hero, rounded, bore);

    // Lift the widget onto the floor and turn it for a three-quarter view.
    world.objects[hero].set_transform(rotation_y(0.5).then(translation(0.0, 1.0, 0.0)));

    world.compute_bounds();
    world
}

// A single scene exercising all four bonus chapters at once:
//   - an area light (soft-edged shadows),
//   - texture mapping (a planar-mapped floor and a spherical-mapped sphere),
//   - focal blur (the front sphere is in focus, the others blur with distance),
//   - a `divide`d cluster of small spheres (bounding-volume hierarchy).
fn build_bonus_world() -> World {
    let mut world = World::new();

    // An overhead area light: a 4x4 grid of samples gives soft shadow edges.
    world.lights = vec![Light::area_light(
        Point {
            x: -3.0,
            y: 5.0,
            z: -5.0,
        },
        Vector {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        },
        4,
        Vector {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        4,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    )];

    // Floor: a planar-mapped UV checker (texture mapping), matte so the area
    // light's soft shadows read clearly.
    let mut floor = Primitive::plane();
    let mut floor_material = Material::default();
    floor_material.set_pattern(Pattern::texture_map(
        UvFace::checkers(
            2.0,
            2.0,
            Color {
                r: 0.85,
                g: 0.85,
                b: 0.85,
            },
            Color {
                r: 0.4,
                g: 0.4,
                b: 0.45,
            },
        ),
        MAPPING_PLANAR,
    ));
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Front sphere: a spherical-mapped checker (texture mapping). It sits at the
    // focal plane, so it stays sharp while the others blur.
    let mut globe = Primitive::sphere();
    globe.set_transform(translation(0.0, 1.0, 0.0));
    let mut globe_material = Material::default();
    globe_material.set_pattern(Pattern::texture_map(
        UvFace::checkers(
            20.0,
            10.0,
            Color {
                r: 0.1,
                g: 0.2,
                b: 0.55,
            },
            Color {
                r: 0.95,
                g: 0.95,
                b: 1.0,
            },
        ),
        MAPPING_SPHERICAL,
    ));
    globe_material.set_diffuse(0.8);
    globe_material.set_specular(0.3);
    globe.set_material(globe_material);
    world.add_object(globe);

    // Two solid spheres set progressively farther back, so focal blur throws
    // them out of focus by increasing amounts.
    let solid = |color: Color| {
        let mut m = Material::default();
        m.set_color(color);
        m.set_diffuse(0.7);
        m.set_specular(0.4);
        m.set_shininess(150.0);
        m
    };
    let mut mid = Primitive::sphere();
    mid.set_transform(translation(2.6, 1.0, 3.0));
    mid.set_material(solid(Color {
        r: 0.8,
        g: 0.2,
        b: 0.2,
    }));
    world.add_object(mid);
    let mut far = Primitive::sphere();
    far.set_transform(translation(5.0, 1.0, 8.0));
    far.set_material(solid(Color {
        r: 1.0,
        g: 0.8,
        b: 0.3,
    }));
    world.add_object(far);

    // A 6x6 cluster of small spheres, then subdivided into a bounding-volume
    // hierarchy with `divide` (bounding boxes & hierarchies).
    let cluster = world.add_object(Primitive::group());
    let bead = solid(Color {
        r: 0.2,
        g: 0.7,
        b: 0.6,
    });
    for row in 0..6 {
        for col in 0..6 {
            let mut s = Primitive::sphere();
            let x = -1.0 + col as Number * 0.4;
            let y = 0.18 + row as Number * 0.4;
            s.set_transform(scaling(0.18, 0.18, 0.18).then(translation(x, y, 0.0)));
            s.set_material(bead.clone());
            world.add_child(cluster, s);
        }
    }
    world.objects[cluster].set_transform(translation(-3.0, 0.0, 1.0));
    // Recursively box the cluster: groups of >=4 children split in half.
    world.divide(cluster, 4);

    world.compute_bounds();
    world
}

// `cargo run --release -- bonus` renders the combined bonus-chapters scene.
fn chapter17() {
    const W: usize = 700;
    const H: usize = 450;
    println!("chapter17: building the bonus scene (area light, textures, focal blur, divide)...");
    let world = build_bonus_world();
    println!("chapter17: {} arena objects", world.objects.len());

    let from = Point {
        x: 0.0,
        y: 2.5,
        z: -7.0,
    };
    let to = Point {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let mut camera: Camera<W, H> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        from,
        to,
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));
    // Focus on the front sphere; a positive aperture blurs everything else.
    let focal_distance = (to - from).magnitude();
    camera.set_focal_blur(0.12, focal_distance, 24);

    println!("chapter17: rendering {W}x{H} (this samples the lens + area light, so it is slow)...");
    let start = Instant::now();
    let canvas = camera.render_par(world);
    println!("chapter17: rendered in {:.2?}", start.elapsed());
    let filename = "chapter17.ppm";
    match canvas.write_ppm(filename, PpmFormat::P6) {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}

fn chapter16() {
    println!("chapter16: building the CSG widget...");
    let world = build_csg_world();

    let mut camera: Camera<800, 600> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 3.0,
            y: 3.0,
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
    println!("chapter16: rendering 800x600...");
    let canvas = camera.render_par(world);
    let filename = "chapter16.ppm";
    match canvas.write_ppm(filename, PpmFormat::P6) {
        Err(_) => println!("Something went wrong!"),
        Ok(()) => println!("Succesfully written {filename}!"),
    }
}

// The interactive fly-through. The scene list (builders + framing poses) lives
// here in the playground; the viewport machinery (window, camera ladder, dynamic
// resolution, progressive refinement, object dragging) lives in `viewport.rs`.
fn flythrough() {
    let scenes = vec![
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
        Scene {
            name: "teapot",
            build: build_teapot_world,
            pos: Point { x: 0.0, y: 4.0, z: -10.0 },
            yaw: 0.0,
            pitch: -0.18,
        },
        Scene {
            name: "csg",
            build: build_csg_world,
            pos: Point { x: 3.0, y: 3.0, z: -5.0 },
            yaw: -0.5,
            pitch: -0.3,
        },
    ];
    Viewport::new(scenes).run();
}

// The live flythrough scene uses the smooth-shaded teapot.
fn build_teapot_world() -> World {
    load_teapot(true)
}

// A CPU benchmark: render a high-res frame of a few scenes and print timings.
// Used to compare optimizations. Renders to nothing (timing only).
fn bench() {
    const W: usize = 1280;
    const H: usize = 720;
    let look = |camera: &mut Camera<W, H>, from: Point, to: Point| {
        camera.set_transform(view_transform(
            from,
            to,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ));
    };

    // Marbles with bounding boxes on, and the same scene with them off, so the
    // cost of the per-ray work (and the intersection bookkeeping) is visible.
    let marbles = build_marbles_world();
    let mut naive = marbles.clone();
    naive.use_bounds = false;
    let mut camera: Camera<W, H> = Camera::new(PI / 3.0);
    look(
        &mut camera,
        Point {
            x: 0.0,
            y: 4.0,
            z: -11.0,
        },
        Point {
            x: 0.0,
            y: 0.3,
            z: 0.0,
        },
    );

    for (label, world) in [("marbles (no bounds)", naive), ("marbles (bvh)", marbles)] {
        let start = Instant::now();
        let _ = camera.render_par(world);
        println!("bench: {label} {W}x{H}: {:.3?}", start.elapsed());
    }

    let teapot = build_teapot_world();
    let mut tcam: Camera<W, H> = Camera::new(PI / 3.0);
    look(
        &mut tcam,
        Point {
            x: 6.0,
            y: 5.0,
            z: -8.0,
        },
        Point {
            x: 0.0,
            y: 1.2,
            z: 0.0,
        },
    );
    let start = Instant::now();
    let _ = tcam.render_par(teapot);
    println!("bench: teapot (bvh) {W}x{H}: {:.3?}", start.elapsed());

    // The two costs the adaptive viewer actually pays: a coarse frame while
    // moving, and a full frame once stopped.
    println!("--- live viewer (adaptive) ---");
    let scenes: [(&str, World, Point, Point); 2] = [
        (
            "marbles",
            build_marbles_world(),
            Point { x: 0.0, y: 4.0, z: -11.0 },
            Point { x: 0.0, y: 0.3, z: 0.0 },
        ),
        (
            "teapot",
            build_teapot_world(),
            Point { x: 0.0, y: 4.0, z: -10.0 },
            Point { x: 0.0, y: 1.0, z: 0.0 },
        ),
    ];
    for (name, world, from, to) in scenes {
        let up = Vector { x: 0.0, y: 1.0, z: 0.0 };
        // Time each moving-ladder resolution so the dynamic scaler's choices are
        // visible, plus the full still frame.
        macro_rules! time_move {
            ($w:literal, $h:literal) => {{
                let mut c: Camera<$w, $h> = Camera::new(PI / 3.0);
                c.set_transform(view_transform(from, to, up));
                let start = Instant::now();
                let _ = c.render_live(&world, MOVE_DEPTH);
                let dt = start.elapsed();
                println!(
                    "bench:   {name} moving {}x{} d{MOVE_DEPTH}: {dt:.3?} (~{:.0} fps)",
                    $w,
                    $h,
                    1.0 / dt.as_secs_f64()
                );
            }};
        }
        time_move!(480, 270);
        time_move!(240, 135);
        time_move!(96, 54);
        time_move!(64, 36);
        let mut hi: Camera<DISP_W, DISP_H> = Camera::new(PI / 3.0);
        hi.set_transform(view_transform(from, to, up));
        let start = Instant::now();
        let _ = hi.render_live(&world, STILL_DEPTH);
        println!(
            "bench:   {name} still {DISP_W}x{DISP_H} d{STILL_DEPTH}: {:.3?}",
            start.elapsed()
        );
    }
}

// Load the Utah teapot (teapot.obj, ~6300 flat triangles) onto a reflective
// checkered floor under a blue ambient sky. The model is read from the working
// directory and packed into a bounding volume hierarchy so it is fast enough to
// fly around; it already sits on the y=0 plane in its own coordinates, so it
// rests on the floor without any extra transform. `smooth` chooses between
// faceted (the file's flat triangles) and Phong-smoothed shading, the two ways
// the book's "Putting It Together" suggests rendering a model.
fn load_teapot(smooth: bool) -> World {
    let mut world = World::new();
    world.lights = vec![Light::point_light(
        Point {
            x: -8.0,
            y: 12.0,
            z: -8.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    )];

    // A large ambient-only sphere acts as a soft blue sky for fill and reflections.
    let mut sky = Primitive::sphere();
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

    // A reflective checkered floor for the teapot to sit on and reflect into.
    let mut floor = Primitive::plane();
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
    floor_material.set_reflective(0.3);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Glossy blue ceramic for the teapot itself; OBJ files carry no material, so
    // the loader paints every triangle with this one.
    let mut teapot_material = Material::default();
    teapot_material.set_color(Color {
        r: 0.25,
        g: 0.45,
        b: 0.85,
    });
    teapot_material.set_ambient(0.1);
    teapot_material.set_diffuse(0.7);
    teapot_material.set_specular(0.5);
    teapot_material.set_shininess(180.0);
    teapot_material.set_reflective(0.12);

    let text = std::fs::read_to_string("teapot.obj")
        .expect("teapot.obj not found in the working directory");
    let parser = obj_parser::parse_obj(&text);
    if smooth {
        parser.to_world_bvh_smooth(&mut world, 16, teapot_material);
    } else {
        parser.to_world_bvh(&mut world, 16, teapot_material);
    }

    // Build every group's bounding box so the BVH actually culls.
    world.compute_bounds();
    world
}

// `cargo run --release -- teapot` renders the teapot scene to two stills, the
// way the book's chapter 15 "Putting It Together" suggests: teapot.ppm uses the
// file's flat triangles (faceted), and teapot_smooth.ppm interpolates synthesized
// vertex normals (smooth). Both share one camera so they line up for comparison.
fn teapot() {
    const W: usize = 600;
    const H: usize = 400;

    let mut camera: Camera<W, H> = Camera::new(PI / 3.0);
    camera.set_transform(view_transform(
        Point {
            x: 6.0,
            y: 5.0,
            z: -8.0,
        },
        Point {
            x: 0.0,
            y: 1.2,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    ));

    for (smooth, filename) in [(false, "teapot.ppm"), (true, "teapot_smooth.ppm")] {
        let label = if smooth { "smooth" } else { "faceted" };
        println!("teapot: loading teapot.obj ({label}) and building the BVH...");
        let world = load_teapot(smooth);
        println!("teapot:   {} arena objects", world.objects.len());

        println!("teapot: rendering {W}x{H} ({label})...");
        let start = Instant::now();
        let canvas = camera.render_par(world);
        println!("teapot:   rendered in {:.2?}", start.elapsed());

        match canvas.write_ppm(filename, PpmFormat::P6) {
            Err(_) => println!("Something went wrong!"),
            Ok(()) => println!("teapot: wrote {filename}"),
        }
    }
}

fn build_marbles_world() -> World {
    let mut world = World::new();
    world.lights = vec![Light::point_light(
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
    )];

    // A large sphere lit purely by ambient acts as a soft sky, giving the glass
    // and metal marbles something colorful to refract and reflect. It is a
    // top-level object, so it is always tested and is not part of the grid.
    let mut sky = Primitive::sphere();
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
    let mut floor = Primitive::plane();
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
    let grid = world.add_object(Primitive::group());
    let mut metal_index = 0;
    for row in 0..ROWS {
        let row_group = world.add_child(grid, Primitive::group());
        let z = (row as Number - (ROWS as Number - 1.0) / 2.0) * SPACING;
        for col in 0..COLS {
            let x = (col as Number - (COLS as Number - 1.0) / 2.0) * SPACING;
            let mut marble = Primitive::sphere();
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
    world.lights = vec![Light::point_light(
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
    )];

    // A reflective floor so the hexagon casts and catches a little light.
    let mut floor = Primitive::plane();
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
        let mut c = Primitive::sphere();
        c.set_transform(scaling(0.25, 0.25, 0.25).then(translation(0.0, 0.0, -1.0)));
        c.set_material(material.clone());
        c
    };
    let edge = || {
        let mut e = {
        let mut __c = Primitive::cylinder();
        __c.minimum = 0.0;
        __c.maximum = 1.0;
        __c.closed = false;
        __c
    };
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
    let mut hex = Primitive::group();
    hex.set_transform(rotation_x(-PI / 6.0).then(translation(0.0, 1.0, 0.0)));
    let hex = world.add_object(hex);

    // Six sides, each a group rotated a sixth of a turn around y.
    for n in 0..6 {
        let mut side = Primitive::group();
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
        Light::point_light(
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
        ),
        Light::point_light(
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
        ),
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
    let mut ground = Primitive::plane();
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
    let mut sky = Primitive::sphere();
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

    let mut objects: Vec<Primitive> = vec![sky, ground];

    // Main facade: a long, low block spanning the full width.
    let mut base = Primitive::cube();
    base.set_material(marble.clone());
    base.set_transform(scaling(6.0, 1.2, 2.0).then(translation(0.0, 1.2, 0.0)));
    objects.push(base);

    // The two end wings (House and Senate), raised slightly above the facade.
    for sign in [-1.0, 1.0] {
        let mut wing = Primitive::cube();
        wing.set_material(marble.clone());
        wing.set_transform(scaling(1.3, 1.4, 2.0).then(translation(sign * 4.5, 1.4, 0.0)));
        objects.push(wing);
    }

    // Central block that lifts the rotunda above the facade.
    let mut center = Primitive::cube();
    center.set_material(marble.clone());
    center.set_transform(scaling(2.0, 1.0, 2.0).then(translation(0.0, 3.4, 0.0)));
    objects.push(center);

    // The rotunda drum: a closed cylinder carrying the dome.
    let mut drum = {
        let mut __c = Primitive::cylinder();
        __c.minimum = 0.0;
        __c.maximum = 1.0;
        __c.closed = true;
        __c
    };
    drum.set_material(marble.clone());
    drum.set_transform(scaling(1.5, 1.3, 1.5).then(translation(0.0, 4.4, 0.0)));
    objects.push(drum);

    // The dome: a sphere scaled tall, sitting on the drum.
    let mut dome = Primitive::sphere();
    dome.set_material(dome_iron);
    dome.set_transform(scaling(1.5, 1.8, 1.5).then(translation(0.0, 5.7, 0.0)));
    objects.push(dome);

    // The lantern/cupola: a small closed cylinder atop the dome.
    let mut lantern = {
        let mut __c = Primitive::cylinder();
        __c.minimum = 0.0;
        __c.maximum = 1.0;
        __c.closed = true;
        __c
    };
    lantern.set_material(marble.clone());
    lantern.set_transform(scaling(0.35, 0.6, 0.35).then(translation(0.0, 7.3, 0.0)));
    objects.push(lantern);

    // The Statue of Freedom: a bronze cone tapering to a point. Polished
    // metal: dark diffuse, strong specular highlight and real reflectivity.
    let mut statue = {
        let mut __c = Primitive::cone();
        __c.minimum = -1.0;
        __c.maximum = 0.0;
        __c.closed = true;
        __c
    };
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
        let mut column = {
        let mut __c = Primitive::cylinder();
        __c.minimum = 0.0;
        __c.maximum = 1.0;
        __c.closed = true;
        __c
    };
        column.set_material(marble.clone());
        column.set_transform(scaling(0.18, 2.4, 0.18).then(translation(x, 0.0, -2.1)));
        objects.push(column);
    }

    // The pediment resting on the columns.
    let mut pediment = Primitive::cube();
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
    world.lights = vec![Light::point_light(
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
    )];
    let mut floor = Primitive::plane();
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
    let mut wall = Primitive::plane();
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
    let mut cube = Primitive::cube();
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
    let mut floor = Primitive::plane();
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
    let mut middle = Primitive::sphere();
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
    let mut right = Primitive::sphere();
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
    let mut left = Primitive::sphere();
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

    world.lights = vec![Light::point_light(
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
    )];
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
    let mut floor = Primitive::plane();
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
    let mut wall = Primitive::plane();
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

    let mut middle = Primitive::sphere();
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

    let mut right = Primitive::sphere();
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

    let mut left = Primitive::sphere();
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
    let light = Light::point_light(light_position, light_color);
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
    let mut floor = Primitive::plane();
    let mut floor_material = Material::default();
    floor_material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);

    let mut middle = Primitive::sphere();
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

    let mut right = Primitive::sphere();
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

    let mut left = Primitive::sphere();
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
    let light = Light::point_light(light_position, light_color);
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
    let mut floor = Primitive::sphere();
    floor.set_transform(scaling(10.0, 0.01, 10.0));
    let mut material = Material::default();
    material.set_color(Color {
        r: 1.0,
        g: 0.9,
        b: 0.9,
    });
    material.set_specular(0.0);
    floor.set_material(material.clone());

    let mut left_wall = Primitive::sphere();
    const LEFT_WALL_TRANSFORMATION: Matrix<4, 4> = scaling(10.0, 0.01, 10.0)
        .then(rotation_x(PI / 2.0))
        .then(rotation_y(-PI / 4.0))
        .then(translation(0.0, 0.0, 5.0));
    left_wall.set_transform(LEFT_WALL_TRANSFORMATION);
    left_wall.set_material(material);

    let mut right_wall = Primitive::sphere();
    const RIGHT_WALL_TRANSFORMATION: Matrix<4, 4> = scaling(10.0, 0.01, 10.0)
        .then(rotation_x(PI / 2.0))
        .then(rotation_y(PI / 4.0))
        .then(translation(0.0, 0.0, 5.0));
    right_wall.set_transform(RIGHT_WALL_TRANSFORMATION);

    let mut middle = Primitive::sphere();
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

    let mut right = Primitive::sphere();
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

    let mut left = Primitive::sphere();
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
    let mut sphere = Primitive::sphere();
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
    let light = Light::point_light(light_position, light_color);

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

                    let color = lightning(&sphere, light.clone(), point, eye, normal, 1.0);
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
    let mut shape = Primitive::sphere();
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
