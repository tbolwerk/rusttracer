//! The shared per-pixel entry point used by BOTH backends. The CPU host calls
//! `pixel_color` from its rayon render loop; the rust-gpu shader (gpu/shader)
//! calls the exact same function per invocation. It is `no_std`, heap-free and
//! recursion-free, so it compiles to SPIR-V unchanged.

use crate::matrices::Matrix;
use crate::rays::Ray;
use crate::tuples::*;
use crate::worlds::Scene;

// A pinhole camera flattened for upload to the GPU (repr(C), no Option). The host
// builds it from its `Camera` (pixel_size/half_width/half_height come straight
// from `Camera::new`; `inverse_transform` is the camera's world<-view matrix, or
// identity when the camera is at the origin looking down -z). Focal blur is a
// host-only feature and is intentionally omitted here.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Cam {
    pub inverse_transform: Matrix<4, 4>,
    pub pixel_size: Number,
    pub half_width: Number,
    pub half_height: Number,
    pub hsize: u32,
    pub vsize: u32,
    // Reflection/refraction bounce budget (the host's MAX_REFLECTION_DEPTH). Kept
    // on the camera so the GPU honors the same depth the CPU uses.
    pub max_depth: u32,
    // Row offset added to the compute shader's `global_invocation_id.y`. The host
    // dispatches a heavy frame in horizontal tiles (one dispatch per band of rows)
    // so no single dispatch runs long enough to trip the GPU's timeout watchdog;
    // each tile sets this to its first row. Always 0 on the CPU.
    pub row_offset: u32,
}

impl Cam {
    // The primary ray through pixel (px, py). Mirrors the host
    // `Camera::ray_for_pixel` exactly (pinhole, no lens jitter).
    pub fn ray_for_pixel(&self, px: u32, py: u32) -> Ray {
        let xoffset = (px as Number + 0.5) * self.pixel_size;
        let yoffset = (py as Number + 0.5) * self.pixel_size;
        let world_x = self.half_width - xoffset;
        let world_y = self.half_height - yoffset;
        let pixel = self.inverse_transform
            * Point {
                x: world_x,
                y: world_y,
                z: -1.0,
            };
        let origin = self.inverse_transform
            * Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };
        let direction = (pixel - origin).normalize();
        Ray { origin, direction }
    }
}

// Max lights a wavefront frame shades (per-light shadow intensities are stored in
// a fixed-width buffer). Scenes with more lights than this still render on the CPU.
pub const WF_MAX_LIGHTS: usize = 4;

// Per-pixel job-stack depth for the wavefront driver (the global-memory analogue
// of the CPU `color_at`'s `MAX_SHADE_STACK`). Bounds the branching reflect/refract
// tree held in flight per pixel; sized to match the CPU stack.
pub const WF_STACK: usize = 16;

/// A queued shade job — the book's recursive `color_at` call made explicit so the
/// wavefront GPU driver can hold the branching reflect/refract tree as a stack in
/// global memory instead of the in-thread `ShadeJob` stack. Same three fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Job {
    pub ray: Ray,
    pub weight: Color,
    pub remaining: u32,
}

/// Per-pixel working state for ONE wavefront bounce: the surface hit's
/// `Computations` (flattened, `inside` dropped, `bool`-free for storage buffers)
/// plus the popped job's weight/remaining and an `active` flag. The trace kernel
/// fills it (via the book's `prepare_computations`), the shadow kernel reads
/// `over_point`, and the shade kernel reads the rest. Mirrors `Computations` so the
/// shading math stays the single book source.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WfNode {
    pub point: Point,
    pub eyev: Vector,
    pub normalv: Vector,
    pub reflectv: Vector,
    pub over_point: Point,
    pub under_point: Point,
    pub n1: Number,
    pub n2: Number,
    pub weight: Color,
    pub object_id: u32,
    pub remaining: u32,
    pub active: u32, // 1 if this pixel has a hit to shade this round
    pub _pad: u32,
}

impl WfNode {
    // Build the per-pixel node from the book's `Computations` plus the job's weight
    // and bounce budget. The single place the two representations meet.
    pub fn from_comps(c: &crate::intersections::Computations, weight: Color, remaining: u32) -> Self {
        WfNode {
            point: c.point,
            eyev: c.eyev,
            normalv: c.normalv,
            reflectv: c.reflectv,
            over_point: c.over_point,
            under_point: c.under_point,
            n1: c.n1,
            n2: c.n2,
            weight,
            object_id: c.object_id as u32,
            remaining,
            active: 1,
            _pad: 0,
        }
    }

    // Schlick reflectance for this hit. Identical math to `Computations::schlick`,
    // restated on the flat node so the shade kernel needs only the node.
    pub fn schlick(&self) -> Number {
        let mut cos = self.eyev.dot(self.normalv);
        if self.n1 > self.n2 {
            let n = self.n1 / self.n2;
            let sin2_t = n.powi(2) * (1.0 - cos.powi(2));
            if sin2_t > 1.0 {
                return 1.0;
            }
            cos = (1.0 - sin2_t).sqrt();
        }
        let r0 = ((self.n1 - self.n2) / (self.n1 + self.n2)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cos).powi(5)
    }

    // Cosine of the angle between the eye and the surface normal at this hit.
    pub fn cos_i(&self) -> Number {
        self.eyev.dot(self.normalv)
    }

    // Whether this hit is total internal reflection (no refracted ray). Split from
    // `refract_dir` because rust-gpu 0.9 can't lower an `Option` across the shader.
    pub fn tir(&self) -> bool {
        let cos_i = self.cos_i();
        let n_ratio = self.n1 / self.n2;
        n_ratio * n_ratio * (1.0 - cos_i * cos_i) > 1.0
    }

    // The refracted ray direction at this hit. Valid only when `!tir()`; the caller
    // must guard on `tir()` first (the math `sqrt`s a negative under TIR). Same as
    // the refraction branch of the book's `color_at`; kept in raycore so the shade
    // kernel needs no float intrinsics of its own.
    pub fn refract_dir(&self) -> Vector {
        let cos_i = self.cos_i();
        let n_ratio = self.n1 / self.n2;
        let sin2_t = n_ratio * n_ratio * (1.0 - cos_i * cos_i);
        let cos_t = (1.0 - sin2_t).sqrt();
        self.normalv * (n_ratio * cos_i - cos_t) - self.eyev * n_ratio
    }
}

// Color of pixel (px, py): generate the primary ray and trace it through the
// scene. This is the single function the GPU shader wraps.
pub fn pixel_color(scene: &Scene, cam: &Cam, px: u32, py: u32) -> Color {
    let ray = cam.ray_for_pixel(px, py);
    scene.color_at(&ray, cam.max_depth as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::World;

    // pixel_color through a default camera must agree with tracing the same ray
    // via World::color_at (i.e. the GPU entry matches the CPU renderer).
    #[test]
    fn pixel_color_matches_color_at() {
        let w = World::default();
        // A default camera: identity view transform, square 11x11, fov pi/2 -> the
        // classic book test camera. pixel_size/half_* computed as Camera::new does.
        let hsize = 11u32;
        let vsize = 11u32;
        let half_view = (core::f32::consts::FRAC_PI_2 / 2.0).tan();
        let half_width = half_view;
        let half_height = half_view;
        let pixel_size = (half_width * 2.0) / hsize as Number;
        let cam = Cam {
            inverse_transform: Matrix::identity(),
            pixel_size,
            half_width,
            half_height,
            hsize,
            vsize,
            max_depth: 5,
            row_offset: 0,
        };
        let scene = w.scene();
        for &(x, y) in &[(5u32, 5u32), (0, 0), (10, 10), (3, 7)] {
            let ray = cam.ray_for_pixel(x, y);
            let expected = scene.color_at(&ray, 5);
            let got = pixel_color(&scene, &cam, x, y);
            assert_eq!(got, expected);
        }
    }
}
