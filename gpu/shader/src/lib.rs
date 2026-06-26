//! rust-gpu compute shader: one invocation per pixel, calling the shared
//! `raycore::render::pixel_color`. The whole renderer (intersection, CSG/group
//! traversal, Phong, shadows, reflection, refraction, patterns) runs here exactly
//! as on the CPU, because `raycore` (no default features) is no_std, heap-free
//! and recursion-free.
//!
//! NOTE (GPU bring-up, to verify on your eGPU):
//!  * Buffer layouts come from `raycore`'s `#[repr(C)]` structs. Storage buffers
//!    are std430-ish; watch alignment of `Matrix<4,4>`, `Color`/`Point` (12-byte,
//!    may need padding to 16), and the `bool` fields (`closed`, `has_bounds`) —
//!    you may want to widen those to `u32` if the validator complains.
//!  * `Scene::child_indices` is `&[usize]`. rust-gpu lowers `usize` to 32-bit, so
//!    the HOST must upload that buffer as `u32` (see gpu.rs). Object ids inside
//!    `Primitive` (child_start/count, left/right, parent) are already `u32`.
//!  * Pin `spirv-std` and the toolchain to matching versions (gpu/README.md).
#![cfg_attr(target_arch = "spirv", no_std)]

use raycore::lights::Light;
use raycore::render::{pixel_color, Cam};
use raycore::shapes::Primitive;
use raycore::worlds::Scene;
use spirv_std::glam::UVec3;
use spirv_std::spirv;

// Pack a linear Color (channels in roughly 0..1) into 0x00RRGGBB, clamping.
fn pack_color(c: raycore::tuples::Color) -> u32 {
    let to8 = |v: f32| -> u32 {
        let v = if v < 0.0 {
            0.0
        } else if v > 1.0 {
            1.0
        } else {
            v
        };
        (v * 255.0 + 0.5) as u32
    };
    (to8(c.r) << 16) | (to8(c.g) << 8) | to8(c.b)
}

#[spirv(compute(threads(8, 8)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] objects: &[Primitive],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] lights: &[Light],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] child_indices: &[usize],
    #[spirv(uniform, descriptor_set = 0, binding = 3)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] out: &mut [u32],
) {
    let x = id.x;
    let y = id.y;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    // Same flat scene the CPU borrows from `World`, here borrowed from buffers.
    let scene = Scene {
        objects,
        lights,
        child_indices,
        use_bounds: true,
    };
    let color = pixel_color(&scene, cam, x, y);
    out[(y * cam.hsize + x) as usize] = pack_color(color);
}
