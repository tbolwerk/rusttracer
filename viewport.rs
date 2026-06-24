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
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

// Window / full-quality render size.
pub const DISP_W: usize = 1920;
pub const DISP_H: usize = 1080;

// Dynamic resolution while moving (finest first). After each moving frame the
// viewport nudges toward the level that keeps the frame within FRAME_BUDGET_MS,
// so light scenes stay near full resolution and heavy ones drop to a coarse
// (blurrier) level rather than stuttering. Each entry has a matching camera.
const MOVE_LADDER: [(usize, usize); 7] = [
    (960, 540),
    (480, 270),
    (320, 180),
    (240, 135),
    (160, 90),
    (96, 54),
    (64, 36),
];
pub const MOVE_DEPTH: usize = 1; // reflection depth while moving
const FRAME_BUDGET_MS: f64 = 33.0; // ~30 fps target, while moving and per refine stripe
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
fn upscale_bilinear(src: &[u32], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize) {
    let lerp = |a: u32, b: u32, t: f64| -> u32 {
        let chan = |shift: u32| {
            let av = ((a >> shift) & 0xff) as f64;
            let bv = ((b >> shift) & 0xff) as f64;
            (av + (bv - av) * t).round() as u32
        };
        (chan(16) << 16) | (chan(8) << 8) | chan(0)
    };
    for y in 0..dh {
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
    // One camera per moving-ladder resolution (must match MOVE_LADDER exactly),
    // plus the full-resolution camera (which also serves as the picking camera).
    cam_m0: Camera<960, 540>,
    cam_m1: Camera<480, 270>,
    cam_m2: Camera<320, 180>,
    cam_m3: Camera<240, 135>,
    cam_m4: Camera<160, 90>,
    cam_m5: Camera<96, 54>,
    cam_m6: Camera<64, 36>,
    cam_full: Camera<DISP_W, DISP_H>,
    // Navigation state.
    current: usize,
    world: World,
    pos: Point,
    yaw: Number,
    pitch: Number,
    // Render state: frame cache + eviction order, and the progressive-refinement
    // bookkeeping (see the field comments in the original loop).
    cache: HashMap<FrameKey, Vec<u32>>,
    order: VecDeque<FrameKey>,
    prev_key: Option<FrameKey>,
    move_idx: usize,
    full_up: usize,
    full_down: usize,
    full_side: bool,
    full_rows: usize,
    shown_key: Option<FrameKey>,
    display: Vec<u32>,
    drag: Option<Drag>,
    // Reflection depth of the still (refined) frame, adjustable live with [ and ].
    still_depth: usize,
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
            cam_m0: Camera::new(PI / 3.0),
            cam_m1: Camera::new(PI / 3.0),
            cam_m2: Camera::new(PI / 3.0),
            cam_m3: Camera::new(PI / 3.0),
            cam_m4: Camera::new(PI / 3.0),
            cam_m5: Camera::new(PI / 3.0),
            cam_m6: Camera::new(PI / 3.0),
            cam_full: Camera::new(PI / 3.0),
            current: 0,
            world,
            pos,
            yaw,
            pitch,
            cache: HashMap::new(),
            order: VecDeque::new(),
            prev_key: None,
            move_idx: 2, // start mid-ladder (320x180); auto-adjusts to the budget
            full_up: DISP_H / 2,
            full_down: DISP_H / 2,
            full_side: false,
            full_rows: 16,
            shown_key: None,
            display: vec![0; DISP_W * DISP_H],
            drag: None,
            still_depth: STILL_DEPTH,
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
            // The full camera is used for picking and the final still frame; the
            // moving cameras get their transform set just before they render.
            self.cam_full.set_transform(view);
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
            self.full_up = DISP_H / 2;
            self.full_down = DISP_H / 2;
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
                let ray = self.cam_full.ray_for_pixel(px, py);
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

    // Moving / dragging: render at the current dynamic resolution, upscale to the
    // window, then nudge the level toward the frame-time budget.
    fn render_moving(&mut self, view: Matrix<4, 4>) {
        self.full_up = DISP_H / 2; // abort any in-progress refinement
        self.full_down = DISP_H / 2;
        self.shown_key = None;
        let start = Instant::now();
        let small = match self.move_idx {
            0 => {
                self.cam_m0.set_transform(view);
                self.cam_m0.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            1 => {
                self.cam_m1.set_transform(view);
                self.cam_m1.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            2 => {
                self.cam_m2.set_transform(view);
                self.cam_m2.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            3 => {
                self.cam_m3.set_transform(view);
                self.cam_m3.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            4 => {
                self.cam_m4.set_transform(view);
                self.cam_m4.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            5 => {
                self.cam_m5.set_transform(view);
                self.cam_m5.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
            _ => {
                self.cam_m6.set_transform(view);
                self.cam_m6.render_live(&self.world, MOVE_DEPTH).to_argb()
            }
        };
        let (mw, mh) = MOVE_LADDER[self.move_idx];
        // Guard the ladder/camera invariant: each MOVE_LADDER entry must match the
        // resolution of the camera rendered for that index, or the upscale would
        // read past `small`.
        debug_assert_eq!(small.len(), mw * mh, "MOVE_LADDER does not match cameras");
        upscale_bilinear(&small, mw, mh, &mut self.display, DISP_W, DISP_H);
        // Hysteresis: coarser if we blew the budget, finer only when well under it.
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if ms > FRAME_BUDGET_MS * 1.2 && self.move_idx < MOVE_LADDER.len() - 1 {
            self.move_idx += 1;
        } else if ms < FRAME_BUDGET_MS * 0.5 && self.move_idx > 0 {
            self.move_idx -= 1;
        }
    }

    // Held still: build the native full-resolution frame over the coarse preview,
    // one adaptive stripe per call, expanding outward from the vertical center so
    // the (usually centered) subject sharpens first.
    fn refine_still(&mut self, key: FrameKey) {
        if let Some(buffer) = self.cache.get(&key) {
            self.display.copy_from_slice(buffer);
            self.shown_key = Some(key);
            self.full_up = DISP_H / 2;
            self.full_down = DISP_H / 2;
            return;
        }
        // Pick the next band: alternate below/above center, falling to whichever
        // side still has rows left.
        let down_left = self.full_down < DISP_H;
        let render_down = if down_left && self.full_up > 0 {
            !self.full_side
        } else {
            down_left
        };
        self.full_side = !self.full_side;
        let (y0, y1) = if render_down {
            let end = (self.full_down + self.full_rows).min(DISP_H);
            let band = (self.full_down, end);
            self.full_down = end;
            band
        } else {
            let start_row = self.full_up.saturating_sub(self.full_rows);
            let band = (start_row, self.full_up);
            self.full_up = start_row;
            band
        };
        let start = Instant::now();
        self.cam_full
            .render_live_rows(&self.world, self.still_depth, y0, y1, &mut self.display);
        // Re-size the stripe so each one lands near the frame budget.
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if ms > 0.0 {
            let target = (self.full_rows as f64 * FRAME_BUDGET_MS / ms).round() as usize;
            self.full_rows = target.clamp(2, DISP_H);
        }
        if self.full_down >= DISP_H && self.full_up == 0 {
            // Frame complete: cache it and stop refining this pose.
            self.cache.insert(key, self.display.clone());
            self.order.push_back(key);
            if self.order.len() > FRAME_CACHE_CAP {
                if let Some(evicted) = self.order.pop_front() {
                    self.cache.remove(&evicted);
                }
            }
            self.shown_key = Some(key);
            self.full_up = DISP_H / 2;
            self.full_down = DISP_H / 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
