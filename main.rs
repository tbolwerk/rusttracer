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
mod csg;
use csg::*;
mod cylinders;
mod obj_parser;
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
mod texture_maps;
use texture_maps::*;
mod triangles;
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
    world.lights = vec![Light::Point(PointLight::new(
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
    ))];

    // Ambient sky sphere for soft fill and reflections.
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

    // Reflective checkered floor.
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
    let hero = world.add_object(Shape::csg(CsgOperation::Difference));

    // left = rounded cube = Intersection(cube, slightly larger sphere)
    let rounded = world.add_object(Shape::csg(CsgOperation::Intersection));
    let mut cube = Shape::cube();
    cube.set_material(shell.clone());
    let cube_id = world.add_object(cube);
    let mut sphere = Shape::sphere();
    sphere.set_transform(scaling(1.3, 1.3, 1.3));
    sphere.set_material(shell.clone());
    let sphere_id = world.add_object(sphere);
    world.set_csg_children(rounded, cube_id, sphere_id);

    // right = drill = Union of three finite cylinders, one per axis. Finite (not
    // infinite) so the CSG bounding box stays finite and cullable.
    let drill = |transform: Matrix<4, 4>| {
        let mut c = Shape::Cylinder(Cylinder::new(-1.5, 1.5, false));
        c.set_transform(scaling(0.5, 1.0, 0.5).then(transform));
        c.set_material(gold.clone());
        c
    };
    let bore = world.add_object(Shape::csg(CsgOperation::Union));
    let bore_xy = world.add_object(Shape::csg(CsgOperation::Union));
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
    let mut floor = Shape::plane();
    let mut floor_material = Material::default();
    floor_material.set_pattern(Pattern::texture_map(
        UvPattern::checkers(
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
        UvMapping::Planar,
    ));
    floor_material.set_specular(0.0);
    floor.set_material(floor_material);
    world.add_object(floor);

    // Front sphere: a spherical-mapped checker (texture mapping). It sits at the
    // focal plane, so it stays sharp while the others blur.
    let mut globe = Shape::sphere();
    globe.set_transform(translation(0.0, 1.0, 0.0));
    let mut globe_material = Material::default();
    globe_material.set_pattern(Pattern::texture_map(
        UvPattern::checkers(
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
        UvMapping::Spherical,
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
    let mut mid = Shape::sphere();
    mid.set_transform(translation(2.6, 1.0, 3.0));
    mid.set_material(solid(Color {
        r: 0.8,
        g: 0.2,
        b: 0.2,
    }));
    world.add_object(mid);
    let mut far = Shape::sphere();
    far.set_transform(translation(5.0, 1.0, 8.0));
    far.set_material(solid(Color {
        r: 1.0,
        g: 0.8,
        b: 0.3,
    }));
    world.add_object(far);

    // A 6x6 cluster of small spheres, then subdivided into a bounding-volume
    // hierarchy with `divide` (bounding boxes & hierarchies).
    let cluster = world.add_object(Shape::group());
    let bead = solid(Color {
        r: 0.2,
        g: 0.7,
        b: 0.6,
    });
    for row in 0..6 {
        for col in 0..6 {
            let mut s = Shape::sphere();
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

// Adaptive resolution for the interactive viewer. Ray tracing a high-resolution
// frame with reflections is far too slow to do every frame on a CPU, so the
// viewer renders coarsely *while the camera is moving* (few pixels, shallow
// reflections, upscaled to fill the window) and only renders the full
// DISP_W x DISP_H frame once the camera stops. The full frames are pose-cached,
// so revisiting a viewpoint is instant.
const DISP_W: usize = 960; // window / full-quality render width
const DISP_H: usize = 540; // window / full-quality render height

// Dynamic resolution while moving. The viewer renders at one of these sizes
// (finest first), bilinear-upscaled to the window, and after each moving frame
// nudges toward the level that keeps the frame within FRAME_BUDGET_MS. A light
// scene settles near full resolution; a heavy scene like the teapot drops to a
// coarse level so motion stays smooth (just blurrier), then sharpens when still.
// Each entry has a matching Camera in `flythrough`.
const MOVE_LADDER: [(usize, usize); 6] =
    [(480, 270), (320, 180), (240, 135), (160, 90), (96, 54), (64, 36)];
const MOVE_DEPTH: usize = 1; // reflection depth while moving
const FRAME_BUDGET_MS: f64 = 33.0; // ~30 fps target, both while moving and per refine stripe

// Once stopped, the full-resolution frame is built in horizontal stripes, a few
// rows per loop iteration, so the image scan-refines over the coarse preview
// while input stays responsive between stripes. The stripe height auto-tunes to
// the frame budget. STILL_DEPTH is the full-frame reflection depth.
const STILL_DEPTH: usize = 4;

// Frame cache tuning for the live viewer. A full-resolution frame is keyed by the
// camera pose (position + yaw + pitch) and the scene it belongs to, so flying
// back through a viewpoint already seen returns the cached frame instead of
// re-tracing every pixel. The pose is quantized onto a grid to make revisits
// hit: POSE_STEP is the position grid (world units) and ANGLE_STEP the look
// grid (radians). Finer grids reduce the visual "snap" of a reused frame but
// reuse less often.
const POSE_STEP: Number = 0.1; // ~10 cm position buckets
const ANGLE_STEP: Number = 0.01; // ~0.6 degree look buckets
const FRAME_CACHE_CAP: usize = 24; // bounded: ~24 * 960*540 * 4 bytes ≈ 50 MB

// Bilinear upscale of a coarse ARGB frame into the full-size display buffer.
// Blending four source texels per channel turns the blocky nearest-neighbor
// preview into a smooth one for the cost of a little blur, which reads far better
// while moving than hard pixel edges. Cheap enough to run every frame.
fn upscale_bilinear(src: &[u32], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize) {
    // Per-channel lerp on packed 0x00RRGGBB values.
    let lerp = |a: u32, b: u32, t: f64| -> u32 {
        let chan = |shift: u32| {
            let av = ((a >> shift) & 0xff) as f64;
            let bv = ((b >> shift) & 0xff) as f64;
            (av + (bv - av) * t).round() as u32
        };
        (chan(16) << 16) | (chan(8) << 8) | chan(0)
    };
    for y in 0..dh {
        // Map the dst pixel center back into source space, then clamp to texels.
        let fy = (y as f64 + 0.5) * sh as f64 / dh as f64 - 0.5;
        let y0 = fy.floor().max(0.0) as usize;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = (fy - y0 as f64).clamp(0.0, 1.0);
        for x in 0..dw {
            let fx = (x as f64 + 0.5) * sw as f64 / dw as f64 - 0.5;
            let x0 = fx.floor().max(0.0) as usize;
            let x1 = (x0 + 1).min(sw - 1);
            let tx = (fx - x0 as f64).clamp(0.0, 1.0);
            let top = lerp(src[y0 * sw + x0], src[y0 * sw + x1], tx);
            let bot = lerp(src[y1 * sw + x0], src[y1 * sw + x1], tx);
            dst[y * dw + x] = lerp(top, bot, ty);
        }
    }
}

// Where a ray crosses the horizontal plane y = `plane_y`, going forward. Used to
// drag a picked object across a horizontal plane at the height it was grabbed.
fn ray_ground_hit(ray: &Ray, plane_y: Number) -> Option<Point> {
    if ray.direction.y.abs() < EPSILON {
        return None;
    }
    let t = (plane_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return None;
    }
    Some(ray.position(t))
}

// A cache key for a rendered live frame: the scene index plus the camera pose
// snapped to the grids above. Quantizing to integers makes the key hashable and
// lets nearby revisits share a cached frame.
type FrameKey = (usize, i64, i64, i64, i64, i64);
fn frame_key(scene: usize, pos: Point, yaw: Number, pitch: Number) -> FrameKey {
    let snap = |v: Number, step: Number| (v / step).round() as i64;
    (
        scene,
        snap(pos.x, POSE_STEP),
        snap(pos.y, POSE_STEP),
        snap(pos.z, POSE_STEP),
        snap(yaw, ANGLE_STEP),
        snap(pitch, ANGLE_STEP),
    )
}

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
    use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
    use std::collections::{HashMap, VecDeque};

    const MOVE: Number = 0.35; // world units per frame while a move key is held
    const LOOK: Number = 0.04; // radians per frame while a look key is held

    // An in-progress object drag: the object being moved, its transform at the
    // moment it was grabbed, and the world point under the cursor at grab time.
    // Dragging translates the object along the horizontal plane through `grab`.
    struct Drag {
        id: usize,
        base: Matrix<4, 4>,
        grab: Point,
    }

    // Each scene starts from a pose that frames it; fly freely from there.
    let scenes = [
        Scene {
            name: "marbles",
            build: build_marbles_world,
            pos: Point {
                x: 0.0,
                y: 4.0,
                z: -11.0,
            },
            yaw: 0.0,
            pitch: -0.25,
        },
        Scene {
            name: "capitol",
            build: build_capitol_world,
            pos: Point {
                x: 0.0,
                y: 5.0,
                z: -18.0,
            },
            yaw: 0.0,
            pitch: -0.08,
        },
        Scene {
            name: "hexagon",
            build: build_hexagon_world,
            pos: Point {
                x: 0.0,
                y: 2.5,
                z: -5.0,
            },
            yaw: 0.0,
            pitch: -0.25,
        },
        Scene {
            name: "glass",
            build: build_glass_world,
            pos: Point {
                x: 0.0,
                y: 1.5,
                z: -5.5,
            },
            yaw: 0.0,
            pitch: -0.08,
        },
        Scene {
            name: "teapot",
            build: build_teapot_world,
            pos: Point {
                x: 0.0,
                y: 4.0,
                z: -10.0,
            },
            yaw: 0.0,
            pitch: -0.18,
        },
        Scene {
            name: "csg",
            build: build_csg_world,
            pos: Point {
                x: 3.0,
                y: 3.0,
                z: -5.0,
            },
            yaw: -0.5,
            pitch: -0.3,
        },
    ];

    // One camera per moving-ladder resolution, plus the full-resolution camera
    // (which also serves as the mouse-picking camera). cam_m0 (480x270) doubles as
    // the still mid-refinement frame. All share the same pose each frame.
    let mut cam_m0: Camera<480, 270> = Camera::new(PI / 3.0);
    let mut cam_m1: Camera<320, 180> = Camera::new(PI / 3.0);
    let mut cam_m2: Camera<240, 135> = Camera::new(PI / 3.0);
    let mut cam_m3: Camera<160, 90> = Camera::new(PI / 3.0);
    let mut cam_m4: Camera<96, 54> = Camera::new(PI / 3.0);
    let mut cam_m5: Camera<64, 36> = Camera::new(PI / 3.0);
    let mut cam_full: Camera<DISP_W, DISP_H> = Camera::new(PI / 3.0);
    let title = |name: &str| {
        format!("rusttracer [{name}] - 1-6 scene, N next, WASD/RF move, arrows look, drag to move, Esc quit")
    };
    let mut window = Window::new(
        &title(scenes[0].name),
        DISP_W,
        DISP_H,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");

    let mut current = 0usize;
    let mut world = (scenes[current].build)();
    let mut pos = scenes[current].pos;
    let mut yaw = scenes[current].yaw;
    let mut pitch = scenes[current].pitch;

    // Pose-keyed cache of full-resolution frames, plus an insertion-order queue so
    // the cache can evict its oldest frame once it reaches FRAME_CACHE_CAP.
    let mut cache: HashMap<FrameKey, Vec<u32>> = HashMap::new();
    let mut order: VecDeque<FrameKey> = VecDeque::new();
    // `prev_key` detects camera motion; `level` is the current refinement step
    // (0..=MAX_LEVEL), climbing while still; `shown_key` marks which pose's full
    // frame is in `display`.
    let mut prev_key: Option<FrameKey> = None;
    // `move_idx` is the current dynamic-resolution level while moving. `full_y` is
    // the next row of the in-progress full-resolution stripe refinement, and
    // `full_rows` the auto-tuned stripe height.
    let mut move_idx: usize = 2; // start mid-ladder (240x135)
    let mut full_y: usize = 0;
    let mut full_rows: usize = 16;
    let mut shown_key: Option<FrameKey> = None;
    let mut display: Vec<u32> = vec![0; DISP_W * DISP_H];
    let mut drag: Option<Drag> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Scene switching: digits 1-4 pick a scene, N cycles to the next. Each
        // switch rebuilds the world and resets the camera to that scene's pose.
        let digit_keys = [
            Key::Key1,
            Key::Key2,
            Key::Key3,
            Key::Key4,
            Key::Key5,
            Key::Key6,
        ];
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
        if window.is_key_down(Key::Q) {
            pos.y += MOVE; // Q: rise
        }
        if window.is_key_down(Key::E) {
            pos.y -= MOVE; // E: descend
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
        let view = view_transform(
            pos,
            pos + fwd,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        );
        // The full-resolution camera is used for both picking and the final still
        // frame; the per-render moving cameras get their transform set just before
        // they render. One inverse here is negligible.
        cam_full.set_transform(view);

        // Mouse: left-click picks the object under the cursor and drags it across
        // the horizontal plane it was grabbed on. Moving an object mutates the
        // scene, so it invalidates the frame cache and the cached bounding boxes.
        let mut scene_changed = false;
        let cursor = window.get_mouse_pos(MouseMode::Discard);
        if window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = cursor {
                let px = (mx as usize).min(DISP_W - 1);
                let py = (my as usize).min(DISP_H - 1);
                let ray = cam_full.ray_for_pixel(px, py);
                match &drag {
                    None => {
                        // Begin a drag: pick the top-level object under the cursor.
                        if let Some(hit) = world.intersect_world(&ray).hit() {
                            let root = world.root_of(hit.object_id);
                            if world.is_pickable(root) {
                                drag = Some(Drag {
                                    id: root,
                                    base: world.objects[root].get_transform(),
                                    grab: ray.position(hit.t),
                                });
                            }
                        }
                    }
                    Some(d) => {
                        // Continue: slide the object so the grab point tracks the
                        // cursor across the horizontal plane at the grab height.
                        if let Some(p) = ray_ground_hit(&ray, d.grab.y) {
                            let delta = translation(p.x - d.grab.x, 0.0, p.z - d.grab.z);
                            world.objects[d.id].set_transform(d.base.then(delta));
                            scene_changed = true;
                        }
                    }
                }
            }
        } else {
            drag = None;
        }

        let key = frame_key(current, pos, yaw, pitch);
        // "Active" = the view or the scene changed this frame, so render coarsely
        // and (if the scene changed) drop the now-stale caches.
        let active = prev_key != Some(key) || scene_changed;
        prev_key = Some(key);
        if scene_changed {
            cache.clear();
            order.clear();
            world.compute_bounds();
        }

        if active {
            // Moving / dragging: render at the current dynamic resolution, then
            // adjust the level toward the frame-time budget for next frame.
            full_y = 0; // abort any in-progress full refinement
            shown_key = None;
            let start = Instant::now();
            let small = match move_idx {
                0 => {
                    cam_m0.set_transform(view);
                    cam_m0.render_live(&world, MOVE_DEPTH).to_argb()
                }
                1 => {
                    cam_m1.set_transform(view);
                    cam_m1.render_live(&world, MOVE_DEPTH).to_argb()
                }
                2 => {
                    cam_m2.set_transform(view);
                    cam_m2.render_live(&world, MOVE_DEPTH).to_argb()
                }
                3 => {
                    cam_m3.set_transform(view);
                    cam_m3.render_live(&world, MOVE_DEPTH).to_argb()
                }
                4 => {
                    cam_m4.set_transform(view);
                    cam_m4.render_live(&world, MOVE_DEPTH).to_argb()
                }
                _ => {
                    cam_m5.set_transform(view);
                    cam_m5.render_live(&world, MOVE_DEPTH).to_argb()
                }
            };
            let (mw, mh) = MOVE_LADDER[move_idx];
            upscale_bilinear(&small, mw, mh, &mut display, DISP_W, DISP_H);
            // Hysteresis: drop to a coarser level if we blew the budget, climb to a
            // finer one only when comfortably under it, so it settles per scene.
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if ms > FRAME_BUDGET_MS * 1.2 && move_idx < MOVE_LADDER.len() - 1 {
                move_idx += 1;
            } else if ms < FRAME_BUDGET_MS * 0.5 && move_idx > 0 {
                move_idx -= 1;
            }
        } else if shown_key != Some(key) {
            // Held still: build the native full-resolution frame over the coarse
            // preview, one adaptive stripe per iteration so input stays live.
            if let Some(buffer) = cache.get(&key) {
                display.copy_from_slice(buffer);
                shown_key = Some(key);
                full_y = 0;
            } else {
                let y1 = (full_y + full_rows).min(DISP_H);
                let start = Instant::now();
                cam_full.render_live_rows(&world, STILL_DEPTH, full_y, y1, &mut display);
                // Re-size the stripe so each one lands near the frame budget.
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                if ms > 0.0 {
                    let target = (full_rows as f64 * FRAME_BUDGET_MS / ms).round() as usize;
                    full_rows = target.clamp(2, DISP_H);
                }
                full_y = y1;
                if full_y >= DISP_H {
                    // Frame complete: cache it and stop refining this pose.
                    cache.insert(key, display.clone());
                    order.push_back(key);
                    if order.len() > FRAME_CACHE_CAP {
                        if let Some(evicted) = order.pop_front() {
                            cache.remove(&evicted);
                        }
                    }
                    shown_key = Some(key);
                    full_y = 0;
                }
            }
        }
        // Otherwise still and already showing the full frame: reuse `display`.
        if window.update_with_buffer(&display, DISP_W, DISP_H).is_err() {
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
    world.lights = vec![Light::Point(PointLight::new(
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
    ))];

    // A large ambient-only sphere acts as a soft blue sky for fill and reflections.
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

    // A reflective checkered floor for the teapot to sit on and reflect into.
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

#[cfg(test)]
mod viewer_tests {
    use super::*;

    #[test]
    fn ray_ground_hit_finds_the_plane_crossing() {
        let down = Ray {
            origin: Point { x: 0.0, y: 5.0, z: 0.0 },
            direction: Vector { x: 0.0, y: -1.0, z: 0.0 },
        };
        let p = ray_ground_hit(&down, 1.0).expect("should cross y=1");
        assert_almost_eq!(p.y, 1.0);
        assert_almost_eq!(p.x, 0.0);
        assert_almost_eq!(p.z, 0.0);
        // Parallel to the plane: no crossing.
        let flat = Ray {
            origin: Point { x: 0.0, y: 5.0, z: 0.0 },
            direction: Vector { x: 1.0, y: 0.0, z: 0.0 },
        };
        assert!(ray_ground_hit(&flat, 1.0).is_none());
        // Pointing away (plane is behind the ray): no forward crossing.
        let up = Ray {
            origin: Point { x: 0.0, y: 5.0, z: 0.0 },
            direction: Vector { x: 0.0, y: 1.0, z: 0.0 },
        };
        assert!(ray_ground_hit(&up, 1.0).is_none());
    }

    #[test]
    fn bilinear_upscale_preserves_a_flat_color() {
        let src = vec![0x00_80_40_20u32; 4]; // 2x2, uniform
        let mut dst = vec![0u32; 16]; // 4x4
        upscale_bilinear(&src, 2, 2, &mut dst, 4, 4);
        for p in dst {
            assert_eq!(p, 0x00_80_40_20);
        }
    }

    #[test]
    fn bilinear_upscale_blends_between_texels() {
        // 2x1 source: left fully red, right black.
        let src = vec![0x00_ff_00_00u32, 0x00_00_00_00u32];
        let mut dst = vec![0u32; 8]; // 8x1
        upscale_bilinear(&src, 2, 1, &mut dst, 8, 1);
        let red = |p: u32| (p >> 16) & 0xff;
        assert!(red(dst[0]) > 200, "left edge stays red");
        assert!(red(dst[7]) < 60, "right edge stays dark");
        assert!(red(dst[0]) >= red(dst[7]), "red falls off left to right");
    }
}
