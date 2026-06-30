// The interactive viewport: a real-time window onto a `World`, the counterpart
// to the offline `Camera::render_par` used for the book's chapter images. You fly
// a camera through a scene, and because ray tracing a full frame with reflections
// is too slow to do every frame on a CPU, the viewport renders at a dynamically
// chosen low resolution while the camera moves and then refines to the native
// resolution, center-outward, once it stops — staying responsive throughout.
// Objects can be picked and transformed with the mouse and keyboard.
//
// `Viewport` owns the window, a ladder of cameras (one per moving resolution plus
// the full-resolution one), the current scene/pose, and the frame cache. Drop in
// a list of `Scene`s and call `run`.

use crate::camera::Camera;
use crate::matrices::Matrix;
use crate::rays::Ray;
use crate::shapes::HasTransform;
use crate::transformations::{rotation_y, scaling, translation, view_transform, PI};
use crate::tuples::*;
use crate::worlds::World;

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use rayon::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

// Window / full-quality render size.
pub const DISP_W: usize = 1920;
pub const DISP_H: usize = 1080;

// Dynamic resolution, expressed as a "block size": one ray is traced per
// block x block square and the result fills the block. A larger block is coarser
// and faster. MAX_BLOCK is the coarsest (used for the first refinement pass and as
// the slowest-scene fallback while moving); blocks are always powers of two so the
// refinement's sample grids nest, letting each pixel be traced exactly once.
const MAX_BLOCK: usize = 32;
// GPU still refinement: full-res tile edge (pixels). The full-res phase is rendered
// as these tiles in center-out order, a budgeted batch per frame.
#[cfg(feature = "gpu")]
const GPU_TILE: usize = 128;
// The whole-frame resolution ladder refines down to this scale (1/4) before handing
// off to the full-res center-out tiles; finer whole-frame levels would be too slow
// to render in one go on heavy scenes.
#[cfg(feature = "gpu")]
const GPU_LADDER_FLOOR: usize = 4;
pub const MOVE_DEPTH: usize = 1; // reflection depth while moving
const FRAME_BUDGET_MS: f64 = 33.0; // ~30 fps target, while moving and per refine chunk
pub const STILL_DEPTH: usize = 4; // full-frame reflection depth once stopped

// Pose-keyed frame cache: a full-resolution frame is keyed by scene + quantized
// pose, so revisiting a viewpoint is instant. POSE_STEP / ANGLE_STEP are the
// quantization grids; finer means fewer false cache hits but less reuse.
const POSE_STEP: Number = 0.1;
const ANGLE_STEP: Number = 0.01;
const FRAME_CACHE_CAP: usize = 12; // bounded; at 1920x1080 that's ~12 * 8.3 MB ≈ 100 MB

const MOVE: Number = 0.35; // world units per frame while a move key is held
const LOOK: Number = 0.04; // radians per frame while a look key is held

// A selectable scene: a name, a builder, and a camera pose to start from.
pub struct Scene {
    pub name: &'static str,
    pub build: fn() -> World,
    pub pos: Point,
    pub yaw: Number,
    pub pitch: Number,
}

// A cache key: scene index plus the camera pose snapped to the grids above.
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

// Forward (look) direction from yaw (around +y) and pitch. yaw = pitch = 0 looks
// toward +z, where scenes sit from their default start pose.
fn forward(yaw: Number, pitch: Number) -> Vector {
    Vector {
        x: pitch.cos() * yaw.sin(),
        y: pitch.sin(),
        z: pitch.cos() * yaw.cos(),
    }
}

// Bilinear upscale of a coarse ARGB frame into the full-size display buffer.
// Blending four source texels per channel turns the blocky nearest-neighbor
// preview into a smooth one for the cost of a little blur, which reads far better
// while moving than hard pixel edges.
pub(crate) fn upscale_bilinear(src: &[u32], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize) {
    let lerp = |a: u32, b: u32, t: f32| -> u32 {
        let chan = |shift: u32| {
            let av = ((a >> shift) & 0xff) as f32;
            let bv = ((b >> shift) & 0xff) as f32;
            (av + (bv - av) * t).round() as u32
        };
        (chan(16) << 16) | (chan(8) << 8) | chan(0)
    };
    // Parallel over destination rows: at 1920x1080 this was ~65 ms single-threaded
    // every moving frame, which alone capped the fly rate; rayon makes it a few ms.
    dst.par_chunks_mut(dw).enumerate().for_each(|(y, row)| {
        let fy = (y as f32 + 0.5) * sh as f32 / dh as f32 - 0.5;
        let y0 = fy.floor().max(0.0) as usize;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = (fy - y0 as f32).clamp(0.0, 1.0);
        for x in 0..dw {
            let fx = (x as f32 + 0.5) * sw as f32 / dw as f32 - 0.5;
            let x0 = fx.floor().max(0.0) as usize;
            let x1 = (x0 + 1).min(sw - 1);
            let tx = (fx - x0 as f32).clamp(0.0, 1.0);
            let top = lerp(src[y0 * sw + x0], src[y0 * sw + x1], tx);
            let bot = lerp(src[y1 * sw + x0], src[y1 * sw + x1], tx);
            row[x] = lerp(top, bot, ty);
        }
    });
}

// Paint the block x block square whose top-left is (x, y) with one color, clipped
// to the buffer. Cheap memory writes; the ray tracing is what's parallelized.
fn fill_block(dst: &mut [u32], x: usize, y: usize, block: usize, w: usize, h: usize, color: u32) {
    let x1 = (x + block).min(w);
    let y1 = (y + block).min(h);
    for yy in y..y1 {
        let row = yy * w;
        for xx in x..x1 {
            dst[row + xx] = color;
        }
    }
}

// The sample points to trace for one refinement level, ordered center-outward.
//
// A pixel is a "rep" at this `block` if it lies on the block grid (x % block == 0,
// y % block == 0). It is *new* at this level if it was not already a rep at the
// previous (coarser) level `prev` — so across the levels MAX_BLOCK, MAX_BLOCK/2,
// ... 1 every pixel is traced exactly once (interlaced / mip refinement). `prev`
// is None for the coarsest level, where every rep is new.
//
// Reps are emitted in square (Chebyshev) rings growing from the screen center, so
// the image resolves center-outward in both axes. No sort: the ring walk is O(reps).
fn center_out_reps(block: usize, prev: Option<usize>, w: usize, h: usize) -> Vec<(u32, u32)> {
    let is_new = |x: usize, y: usize| match prev {
        None => true,
        Some(p) => !(x % p == 0 && y % p == 0),
    };
    // The rep nearest the screen center, snapped to the block grid.
    let cx = (w / 2 / block * block) as i64;
    let cy = (h / 2 / block * block) as i64;
    let step = block as i64;
    let (w, h) = (w as i64, h as i64);
    let mut out = Vec::new();
    let mut emit = |x: i64, y: i64, out: &mut Vec<(u32, u32)>| {
        if x >= 0 && x < w && y >= 0 && y < h && is_new(x as usize, y as usize) {
            out.push((x as u32, y as u32));
        }
    };
    let max_r = (w.max(h) / step) + 1;
    for r in 0..=max_r {
        if r == 0 {
            emit(cx, cy, &mut out);
            continue;
        }
        let (lo, hi) = (-r, r);
        // Top and bottom edges of the ring (full width).
        for i in lo..=hi {
            emit(cx + i * step, cy + lo * step, &mut out);
            emit(cx + i * step, cy + hi * step, &mut out);
        }
        // Left and right edges (excluding the corners already done).
        for j in (lo + 1)..hi {
            emit(cx + lo * step, cy + j * step, &mut out);
            emit(cx + hi * step, cy + j * step, &mut out);
        }
    }
    out
}

// The full-res tiles covering a w x h frame, ordered center-out (nearest tile
// center to the screen center first), so GPU still refinement sharpens from the
// middle outward like the CPU's center-out ring refinement.
#[cfg(feature = "gpu")]
fn center_out_tiles(w: usize, h: usize, tile: usize) -> Vec<(u32, u32, u32, u32)> {
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    let mut tiles = Vec::new();
    let mut y = 0;
    while y < h {
        let th = tile.min(h - y);
        let mut x = 0;
        while x < w {
            let tw = tile.min(w - x);
            tiles.push((x as u32, y as u32, tw as u32, th as u32));
            x += tile;
        }
        y += tile;
    }
    let dist2 = |&(x, y, tw, th): &(u32, u32, u32, u32)| {
        let dx = (x as f64 + tw as f64 / 2.0) - cx;
        let dy = (y as f64 + th as f64 / 2.0) - cy;
        dx * dx + dy * dy
    };
    tiles.sort_by(|a, b| dist2(a).partial_cmp(&dist2(b)).unwrap());
    tiles
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

// An in-progress object drag, with every manipulation accumulated relative to the
// transform (`base`) and world grab point (`grab`) captured at pick time:
//   mouse  -> horizontal slide (`dx`/`dz`)
//   Q / E  -> lift up / down (`lift`)
//   W / S  -> scale larger / smaller (`scale`)
//   A / D  -> rotate about Y (`angle`)
// Scale and rotation pivot on the grab point, so the object transforms in place.
struct Drag {
    id: usize,
    base: Matrix<4, 4>,
    grab: Point,
    dx: Number,
    dz: Number,
    lift: Number,
    scale: Number,
    angle: Number,
}

fn title(name: &str, depth: usize) -> String {
    format!("rusttracer [{name}] reflect:{depth} - 1-6 scene, N, WASD/QE move, arrows look, drag, [ ] quality, Esc")
}

// Bounds for the live-adjustable still reflection depth.
const MIN_STILL_DEPTH: usize = 1;
const MAX_STILL_DEPTH: usize = 12;

pub struct Viewport {
    scenes: Vec<Scene>,
    window: Window,
    // A single full-resolution camera, sampled at whatever block size is needed.
    // Also the mouse-picking camera.
    cam: Camera<DISP_W, DISP_H>,
    // Navigation state.
    current: usize,
    world: World,
    pos: Point,
    yaw: Number,
    pitch: Number,
    // Render state.
    cache: HashMap<FrameKey, Vec<u32>>,
    order: VecDeque<FrameKey>,
    prev_key: Option<FrameKey>,
    // Dynamic moving resolution: trace one ray per `move_block` square, budget-tuned.
    move_block: usize,
    // Interlaced still refinement: `refine_block` is the current level, `refine_list`
    // its center-out sample points, `refine_idx` the next to trace, `refine_chunk`
    // the budget-tuned number traced per frame.
    refine_block: usize,
    refine_list: Vec<(u32, u32)>,
    refine_idx: usize,
    refine_chunk: usize,
    shown_key: Option<FrameKey>,
    display: Vec<u32>,
    drag: Option<Drag>,
    // Reflection depth of the still (refined) frame, adjustable live with [ and ].
    still_depth: usize,
    // GPU backend (only with --features gpu). When present, frames are traced on
    // the GPU: coarse while moving, then a resolution ladder that sharpens to full
    // res once still. `None` (no adapter) transparently keeps the CPU path.
    #[cfg(feature = "gpu")]
    gpu: Option<crate::gpu::WavefrontRenderer>,
    // GPU still-refinement state. Refinement runs in two phases that visibly improve
    // quality while held still, like the CPU: first a whole-frame resolution ladder
    // (`gpu_scale` halves each frame, so the image sharpens in steps), then full-res
    // center-out tiles for the final detail (`gpu_tiles` ordered center-out, a cursor
    // into it, and a per-frame batch size auto-tuned to the budget).
    #[cfg(feature = "gpu")]
    gpu_refine_key: Option<FrameKey>,
    // Set when the scene's geometry changed (scene switch / object drag) so the GPU
    // re-uploads it; cleared after the upload, so a static scene isn't re-sent each
    // fly frame.
    #[cfg(feature = "gpu")]
    gpu_scene_dirty: bool,
    #[cfg(feature = "gpu")]
    gpu_scale: usize,
    #[cfg(feature = "gpu")]
    gpu_tiles: Vec<(u32, u32, u32, u32)>,
    #[cfg(feature = "gpu")]
    gpu_tile_cursor: usize,
    #[cfg(feature = "gpu")]
    gpu_tiles_per_frame: usize,
}

impl Viewport {
    // Open the window on the first scene. Panics only if the window can't be
    // created (no display) or `scenes` is empty.
    pub fn new(scenes: Vec<Scene>) -> Self {
        assert!(!scenes.is_empty(), "Viewport needs at least one scene");
        let window = Window::new(
            &title(scenes[0].name, STILL_DEPTH),
            DISP_W,
            DISP_H,
            WindowOptions {
                scale: Scale::X1,
                ..WindowOptions::default()
            },
        )
        .expect("failed to open window");
        let world = (scenes[0].build)();
        let (pos, yaw, pitch) = (scenes[0].pos, scenes[0].yaw, scenes[0].pitch);
        Self {
            scenes,
            window,
            cam: Camera::new(PI / 3.0),
            current: 0,
            world,
            pos,
            yaw,
            pitch,
            cache: HashMap::new(),
            order: VecDeque::new(),
            prev_key: None,
            move_block: 8, // auto-adjusts to the frame budget
            refine_block: usize::MAX, // sentinel: refinement not started
            refine_list: Vec::new(),
            refine_idx: 0,
            refine_chunk: 4096,
            shown_key: None,
            display: vec![0; DISP_W * DISP_H],
            drag: None,
            still_depth: STILL_DEPTH,
            #[cfg(feature = "gpu")]
            gpu: crate::gpu::WavefrontRenderer::new(),
            #[cfg(feature = "gpu")]
            gpu_refine_key: None,
            #[cfg(feature = "gpu")]
            gpu_scene_dirty: true,
            #[cfg(feature = "gpu")]
            gpu_scale: 1,
            #[cfg(feature = "gpu")]
            gpu_tiles: Vec::new(),
            #[cfg(feature = "gpu")]
            gpu_tile_cursor: 0,
            #[cfg(feature = "gpu")]
            gpu_tiles_per_frame: 8,
        }
    }

    // Reset the still refinement to "not started" (next still frame begins at the
    // coarsest level). Called whenever the view or scene changes.
    fn reset_refine(&mut self) {
        self.refine_block = usize::MAX;
        self.refine_list.clear();
        self.refine_idx = 0;
    }

    fn cache_frame(&mut self, key: FrameKey) {
        self.cache.insert(key, self.display.clone());
        self.order.push_back(key);
        if self.order.len() > FRAME_CACHE_CAP {
            if let Some(evicted) = self.order.pop_front() {
                self.cache.remove(&evicted);
            }
        }
    }

    // The render/input loop: render a frame, blit it, read input, repeat, until
    // the window closes or Esc is pressed.
    pub fn run(&mut self) {
        while self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            self.poll_scene_switch();
            self.poll_quality();
            self.poll_navigation();
            let view = self.view();
            self.cam.set_transform(view);
            let scene_changed = self.poll_drag();
            if !self.present(view, scene_changed) {
                break; // window closed
            }
        }
    }

    fn view(&self) -> Matrix<4, 4> {
        let fwd = forward(self.yaw, self.pitch);
        view_transform(
            self.pos,
            self.pos + fwd,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        )
    }

    // Digits 1-6 pick a scene, N cycles to the next. A switch rebuilds the world
    // and resets to that scene's framing pose.
    fn poll_scene_switch(&mut self) {
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
            if i < self.scenes.len() && self.window.is_key_pressed(*key, KeyRepeat::No) {
                next = Some(i);
            }
        }
        if self.window.is_key_pressed(Key::N, KeyRepeat::No) {
            next = Some((self.current + 1) % self.scenes.len());
        }
        if let Some(i) = next {
            self.current = i;
            self.world = (self.scenes[i].build)();
            self.pos = self.scenes[i].pos;
            self.yaw = self.scenes[i].yaw;
            self.pitch = self.scenes[i].pitch;
            #[cfg(feature = "gpu")]
            {
                self.gpu_scene_dirty = true; // new geometry: re-upload to the GPU
            }
            self.window
                .set_title(&title(self.scenes[i].name, self.still_depth));
        }
    }

    // `[` / `]` lower / raise the still-frame reflection depth live. A new depth
    // makes every cached frame stale, so we drop the cache and re-refine the
    // current view from the center at the new quality.
    fn poll_quality(&mut self) {
        let mut changed = false;
        if self.window.is_key_pressed(Key::RightBracket, KeyRepeat::No) {
            self.still_depth = (self.still_depth + 1).min(MAX_STILL_DEPTH);
            changed = true;
        }
        if self.window.is_key_pressed(Key::LeftBracket, KeyRepeat::No) {
            self.still_depth = self.still_depth.saturating_sub(1).max(MIN_STILL_DEPTH);
            changed = true;
        }
        if changed {
            self.cache.clear();
            self.order.clear();
            self.shown_key = None;
            self.reset_refine();
            let name = self.scenes[self.current].name;
            self.window.set_title(&title(name, self.still_depth));
        }
    }

    // WASD/QE fly the camera, arrows look around. While an object is being dragged
    // WASD/QE are consumed by the drag (see `poll_drag`) instead.
    fn poll_navigation(&mut self) {
        let fwd = forward(self.yaw, self.pitch);
        let right = Vector {
            x: self.yaw.cos(),
            y: 0.0,
            z: -self.yaw.sin(),
        };
        if self.drag.is_none() {
            if self.window.is_key_down(Key::W) {
                self.pos = self.pos + fwd * MOVE;
            }
            if self.window.is_key_down(Key::S) {
                self.pos = self.pos + fwd * -MOVE;
            }
            if self.window.is_key_down(Key::A) {
                self.pos = self.pos + right * -MOVE;
            }
            if self.window.is_key_down(Key::D) {
                self.pos = self.pos + right * MOVE;
            }
            if self.window.is_key_down(Key::Q) {
                self.pos.y += MOVE;
            }
            if self.window.is_key_down(Key::E) {
                self.pos.y -= MOVE;
            }
        }
        if self.window.is_key_down(Key::Left) {
            self.yaw -= LOOK;
        }
        if self.window.is_key_down(Key::Right) {
            self.yaw += LOOK;
        }
        if self.window.is_key_down(Key::Up) {
            self.pitch += LOOK;
        }
        if self.window.is_key_down(Key::Down) {
            self.pitch -= LOOK;
        }
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    // Left-click picks the top-level object under the cursor; dragging then moves
    // it (mouse = ground slide, Q/E = lift, W/S = scale, A/D = rotate). Returns
    // whether the scene was mutated this frame.
    fn poll_drag(&mut self) -> bool {
        let mut scene_changed = false;
        let cursor = self.window.get_mouse_pos(MouseMode::Discard);
        if self.window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = cursor {
                let px = (mx as usize).min(DISP_W - 1);
                let py = (my as usize).min(DISP_H - 1);
                let ray = self.cam.ray_for_pixel(px, py);
                // Begin a drag: pick the top-level object under the cursor.
                if self.drag.is_none() {
                    if let Some(hit) = self.world.intersect_world(&ray).hit() {
                        let root = self.world.root_of(hit.object_id);
                        if self.world.is_pickable(root) {
                            self.drag = Some(Drag {
                                id: root,
                                base: self.world.objects[root].get_transform(),
                                grab: ray.position(hit.t),
                                dx: 0.0,
                                dz: 0.0,
                                lift: 0.0,
                                scale: 1.0,
                                angle: 0.0,
                            });
                        }
                    }
                }
                // Continue: only touch the scene when something actually changed,
                // so holding the button still lets the frame refine.
                if let Some(d) = &mut self.drag {
                    const SCALE_RATE: Number = 1.02; // per frame while held
                    const ROTATE_RATE: Number = 0.03; // radians per frame while held
                    let mut changed = false;
                    if self.window.is_key_down(Key::Q) {
                        d.lift += MOVE;
                        changed = true;
                    }
                    if self.window.is_key_down(Key::E) {
                        d.lift -= MOVE;
                        changed = true;
                    }
                    if self.window.is_key_down(Key::W) {
                        d.scale *= SCALE_RATE;
                        changed = true;
                    }
                    if self.window.is_key_down(Key::S) {
                        d.scale = (d.scale / SCALE_RATE).max(0.05);
                        changed = true;
                    }
                    if self.window.is_key_down(Key::A) {
                        d.angle -= ROTATE_RATE; // left
                        changed = true;
                    }
                    if self.window.is_key_down(Key::D) {
                        d.angle += ROTATE_RATE; // right
                        changed = true;
                    }
                    if let Some(p) = ray_ground_hit(&ray, d.grab.y) {
                        let nx = p.x - d.grab.x;
                        let nz = p.z - d.grab.z;
                        if nx != d.dx || nz != d.dz {
                            d.dx = nx;
                            d.dz = nz;
                            changed = true;
                        }
                    }
                    if changed {
                        // Scale and rotate about the grab point, then slide by the
                        // drag delta, composed onto the grab-time transform.
                        let g = d.grab;
                        let m = d
                            .base
                            .then(translation(-g.x, -g.y, -g.z))
                            .then(scaling(d.scale, d.scale, d.scale))
                            .then(rotation_y(d.angle))
                            .then(translation(g.x, g.y, g.z))
                            .then(translation(d.dx, d.lift, d.dz));
                        self.world.objects[d.id].set_transform(m);
                        scene_changed = true;
                    }
                }
            }
        } else {
            self.drag = None;
        }
        scene_changed
    }

    // Render this frame and blit it. Returns false if the window closed.
    fn present(&mut self, view: Matrix<4, 4>, scene_changed: bool) -> bool {
        let key = frame_key(self.current, self.pos, self.yaw, self.pitch);
        // "Active" = the view or the scene changed, so render coarsely and (if the
        // scene changed) drop the now-stale caches.
        let active = self.prev_key != Some(key) || scene_changed;
        self.prev_key = Some(key);
        if scene_changed {
            self.cache.clear();
            self.order.clear();
            self.world.compute_bounds();
        }

        // GPU path. Like the CPU viewport, trade resolution for responsiveness, with
        // a progressive ladder (the GPU traces every pixel, so a full heavy frame per
        // loop is too slow to move through):
        //   * moving -> one coarse 1/`move_block`-res frame at shallow depth, upscaled;
        //     `move_block` auto-tunes to the frame budget like `render_moving`.
        //   * cached -> a previously finished full-res frame for this pose: instant.
        //   * still  -> sharpen over successive frames (1/scale halving to full res),
        //     each frame responsive, then cache the full-res result for revisits.
        // On device loss, drop the renderer and fall through to the CPU path.
        #[cfg(feature = "gpu")]
        if self.gpu.is_some() {
            // Upload the scene to the GPU only when it changed; otherwise reuse what's
            // already there (a static scene isn't re-sent every fly frame).
            let dirty = scene_changed || self.gpu_scene_dirty;
            self.gpu_scene_dirty = false;
            if active {
                self.gpu_refine_key = None; // stopping will restart the ladder
                let scale = self.move_block.max(1) as u32;
                let cam = self.cam.to_cam_scaled(MOVE_DEPTH as u32, scale);
                let (sw, sh) = (cam.hsize as usize, cam.vsize as usize);
                let start = Instant::now();
                match self.gpu.as_ref().unwrap().render_caught(&self.world, &cam, dirty) {
                    Some(small) => {
                        upscale_bilinear(&small, sw, sh, &mut self.display, DISP_W, DISP_H);
                        self.shown_key = None;
                        let ms = start.elapsed().as_secs_f64() * 1000.0;
                        if ms > FRAME_BUDGET_MS * 1.2 {
                            self.move_block = (self.move_block * 2).min(MAX_BLOCK);
                        } else if ms < FRAME_BUDGET_MS * 0.5 {
                            self.move_block = (self.move_block / 2).max(1);
                        }
                        return self.window.update_with_buffer(&self.display, DISP_W, DISP_H).is_ok();
                    }
                    None => {
                        eprintln!("gpu: device lost; switching the viewer to CPU rendering");
                        self.gpu = None;
                    }
                }
            } else if let Some(buf) = self.cache.get(&key) {
                self.display.copy_from_slice(buf);
                self.shown_key = Some(key);
                self.gpu_refine_key = None;
                return self.window.update_with_buffer(&self.display, DISP_W, DISP_H).is_ok();
            } else if self.shown_key != Some(key) {
                // Still: refine in visible quality steps, like the CPU. Phase 1 is a
                // whole-frame resolution ladder (gpu_scale halves each frame), so the
                // image sharpens in steps; phase 2 (scale below the floor) renders
                // full-res center-out tiles over the last coarse frame, then caches.
                if self.gpu_refine_key != Some(key) {
                    self.gpu_refine_key = Some(key);
                    self.gpu_scale = self.move_block.max(GPU_LADDER_FLOOR);
                    self.gpu_tiles.clear();
                    self.gpu_tile_cursor = 0;
                }
                if self.gpu_scale >= GPU_LADDER_FLOOR {
                    // Whole-frame ladder step at the current scale.
                    let cam = self.cam.to_cam_scaled(self.still_depth as u32, self.gpu_scale as u32);
                    let (sw, sh) = (cam.hsize as usize, cam.vsize as usize);
                    match self.gpu.as_ref().unwrap().render_caught(&self.world, &cam, dirty) {
                        Some(frame) => {
                            upscale_bilinear(&frame, sw, sh, &mut self.display, DISP_W, DISP_H);
                            self.gpu_scale /= 2; // sharper next frame
                            return self.window.update_with_buffer(&self.display, DISP_W, DISP_H).is_ok();
                        }
                        None => {
                            eprintln!("gpu: device lost; switching the viewer to CPU rendering");
                            self.gpu = None;
                        }
                    }
                } else {
                    // Full-res center-out tile phase, building on the ladder's frame.
                    if self.gpu_tiles.is_empty() {
                        self.gpu_tiles = center_out_tiles(DISP_W, DISP_H, GPU_TILE);
                        self.gpu_tile_cursor = 0;
                    }
                    let start = self.gpu_tile_cursor;
                    let end = (start + self.gpu_tiles_per_frame.max(1)).min(self.gpu_tiles.len());
                    let batch: Vec<(u32, u32, u32, u32)> = self.gpu_tiles[start..end].to_vec();
                    let cam = self.cam.to_cam(self.still_depth as u32);
                    // Seed the framebuffer with the ladder's coarse frame on the first batch.
                    let seed = if start == 0 { Some(self.display.clone()) } else { None };
                    let t0 = Instant::now();
                    let rendered = self.gpu.as_ref().unwrap().render_tiles_caught(
                        &self.world,
                        &cam,
                        &batch,
                        seed.as_deref(),
                        dirty,
                    );
                    match rendered {
                        Some(full) => {
                            self.display.copy_from_slice(&full);
                            self.gpu_tile_cursor = end;
                            let ms = t0.elapsed().as_secs_f64() * 1000.0;
                            if ms > FRAME_BUDGET_MS * 1.2 {
                                self.gpu_tiles_per_frame = (self.gpu_tiles_per_frame / 2).max(1);
                            } else if ms < FRAME_BUDGET_MS * 0.5 {
                                self.gpu_tiles_per_frame = (self.gpu_tiles_per_frame * 2).min(self.gpu_tiles.len().max(1));
                            }
                            if self.gpu_tile_cursor >= self.gpu_tiles.len() {
                                self.cache_frame(key); // full res done: cache for revisit
                                self.shown_key = Some(key);
                                self.gpu_refine_key = None;
                            }
                            return self.window.update_with_buffer(&self.display, DISP_W, DISP_H).is_ok();
                        }
                        None => {
                            eprintln!("gpu: device lost; switching the viewer to CPU rendering");
                            self.gpu = None;
                        }
                    }
                }
            } else {
                // Fully sharpened and shown: reuse the display.
                return self.window.update_with_buffer(&self.display, DISP_W, DISP_H).is_ok();
            }
        }

        if active {
            self.render_moving(view);
        } else if self.shown_key != Some(key) {
            self.refine_still(key);
        }
        // Otherwise still and already showing the full frame: reuse `display`.
        self.window
            .update_with_buffer(&self.display, DISP_W, DISP_H)
            .is_ok()
    }

    // Moving / dragging: trace one ray per `move_block` square (a sparse,
    // point-sampled image), bilinear-upscale it to the window, then nudge the
    // block size toward the frame-time budget. Larger block = coarser + faster.
    fn render_moving(&mut self, _view: Matrix<4, 4>) {
        self.reset_refine(); // abort any in-progress refinement
        self.shown_key = None;
        let b = self.move_block;
        let sw = (DISP_W / b).max(1);
        let sh = (DISP_H / b).max(1);
        let start = Instant::now();
        // Sample the center pixel of each block, in parallel over the small image.
        let small: Vec<u32> = (0..sw * sh)
            .into_par_iter()
            .map(|k| {
                let px = ((k % sw) * b + b / 2).min(DISP_W - 1);
                let py = ((k / sw) * b + b / 2).min(DISP_H - 1);
                self.cam.pixel_argb(&self.world, px, py, MOVE_DEPTH)
            })
            .collect();
        upscale_bilinear(&small, sw, sh, &mut self.display, DISP_W, DISP_H);
        // Hysteresis on the (power-of-two) block size: coarser if over budget,
        // finer only when comfortably under it, so it settles per scene.
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if ms > FRAME_BUDGET_MS * 1.2 {
            self.move_block = (self.move_block * 2).min(MAX_BLOCK);
        } else if ms < FRAME_BUDGET_MS * 0.5 {
            self.move_block = (self.move_block / 2).max(1);
        }
    }

    // Held still: refine the frame interlaced, coarse-to-fine. Each level traces a
    // growing set of sample points (each pixel exactly once across all levels) in
    // center-out rings, filling each sample's block — so detail grows outward from
    // the center while reusing every ray already traced. A budgeted chunk per call
    // keeps input responsive; the work is parallel across the chunk.
    fn refine_still(&mut self, key: FrameKey) {
        if let Some(buffer) = self.cache.get(&key) {
            self.display.copy_from_slice(buffer);
            self.shown_key = Some(key);
            self.reset_refine();
            return;
        }
        // Advance to the next level when the current one is exhausted.
        if self.refine_idx >= self.refine_list.len() {
            let next = if self.refine_block > MAX_BLOCK {
                MAX_BLOCK // first level
            } else {
                self.refine_block / 2
            };
            if next == 0 {
                // Finest level done: the frame is complete.
                self.cache_frame(key);
                self.shown_key = Some(key);
                self.reset_refine();
                return;
            }
            let prev = if next == MAX_BLOCK { None } else { Some(next * 2) };
            self.refine_block = next;
            self.refine_list = center_out_reps(next, prev, DISP_W, DISP_H);
            self.refine_idx = 0;
        }

        // Trace a budgeted chunk of this level's sample points in parallel, then
        // fill their blocks.
        let b = self.refine_block;
        let end = (self.refine_idx + self.refine_chunk).min(self.refine_list.len());
        let slice = &self.refine_list[self.refine_idx..end];
        let start = Instant::now();
        let colors: Vec<u32> = slice
            .par_iter()
            .map(|&(x, y)| self.cam.pixel_argb(&self.world, x as usize, y as usize, self.still_depth))
            .collect();
        for (k, &(x, y)) in slice.iter().enumerate() {
            fill_block(&mut self.display, x as usize, y as usize, b, DISP_W, DISP_H, colors[k]);
        }
        self.refine_idx = end;
        // Re-size the chunk so each lands near the frame budget.
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if ms > 0.0 {
            let target = (self.refine_chunk as f64 * FRAME_BUDGET_MS / ms).round() as usize;
            self.refine_chunk = target.clamp(64, DISP_W * DISP_H);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn interlaced_levels_cover_every_pixel_exactly_once() {
        let (w, h) = (8usize, 8usize);
        // Levels 4, 2, 1 with their previous (coarser) grids.
        let levels = [(4usize, None), (2, Some(4)), (1, Some(2))];
        let mut all = Vec::new();
        for (b, prev) in levels {
            all.extend(center_out_reps(b, prev, w, h));
        }
        assert_eq!(all.len(), w * h, "every pixel traced once, no waste");
        let set: HashSet<_> = all.iter().copied().collect();
        assert_eq!(set.len(), w * h, "no pixel traced twice");
        for x in 0..w {
            for y in 0..h {
                assert!(set.contains(&(x as u32, y as u32)), "pixel ({x},{y}) covered");
            }
        }
        // The coarsest level starts at the center.
        assert_eq!(center_out_reps(4, None, w, h)[0], (4, 4));
    }

    #[test]
    fn ray_ground_hit_finds_the_plane_crossing() {
        let down = Ray {
            origin: Point {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        };
        let p = ray_ground_hit(&down, 1.0).expect("should cross y=1");
        assert_almost_eq!(p.y, 1.0);
        assert_almost_eq!(p.x, 0.0);
        assert_almost_eq!(p.z, 0.0);
        let flat = Ray {
            origin: Point {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            },
            direction: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        };
        assert!(ray_ground_hit(&flat, 1.0).is_none());
        let up = Ray {
            origin: Point {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
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
        let src = vec![0x00_ff_00_00u32, 0x00_00_00_00u32]; // left red, right black
        let mut dst = vec![0u32; 8]; // 8x1
        upscale_bilinear(&src, 2, 1, &mut dst, 8, 1);
        let red = |p: u32| (p >> 16) & 0xff;
        assert!(red(dst[0]) > 200, "left edge stays red");
        assert!(red(dst[7]) < 60, "right edge stays dark");
        assert!(red(dst[0]) >= red(dst[7]), "red falls off left to right");
    }
}
