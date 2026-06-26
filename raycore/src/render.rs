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
