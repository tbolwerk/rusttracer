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

// Wavefront path tracing. A single megakernel that inlines the WHOLE renderer per
// pixel compiles to one ~24k-instruction function, which RADV's ACO compiler
// refuses (segfaults), forcing the slow LLVM backend and tripping the GPU watchdog
// on heavy scenes. The fix mirrors production GPU path tracers (Cycles / PBRT
// wavefront, "Megakernels Considered Harmful"): split the renderer into small
// kernels connected by per-pixel global buffers, and loop over bounces on the
// host. Each kernel is small enough for ACO and short enough to dodge the
// watchdog. The shading/intersection logic is the SAME raycore functions
// (`intersect_world`, `prepare_computations`, `intensity_at`, `lightning`) — only
// the orchestration differs from the CPU's recursive `color_at`.
//
// Per-pixel state lives in global buffers: a `Job` stack (the branching
// reflect/refract tree the book recurses through) + `sp`, the current bounce's
// `WfNode`, per-light shadow `intensity`, and the `accum`ulated color. The host
// loops: trace → shadow → shade, until every pixel's stack is empty.

use raycore::lights::Light;
use raycore::render::Cam;
use raycore::shapes::{HasMaterial, Primitive};
use raycore::tuples::Color;
use raycore::worlds::Scene;
use spirv_std::glam::UVec3;
use spirv_std::spirv;

// Pack a linear Color (channels in roughly 0..1) into 0x00RRGGBB, clamping.
fn pack_color(c: Color) -> u32 {
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

use raycore::materials::lightning;
use raycore::render::{Job, WfNode, WF_MAX_LIGHTS, WF_STACK};
use raycore::rays::Ray;

const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

// Seed each pixel's path: one job (the primary ray, full weight, full depth) and a
// zeroed accumulator.
#[spirv(compute(threads(8, 8)))]
pub fn wf_init_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] jobs: &mut [Job],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] sp: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] accum: &mut [Color],
) {
    let x = id.x;
    let y = id.y + cam.row_offset;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    let idx = (y * cam.hsize + x) as usize;
    accum[idx] = BLACK;
    jobs[idx * WF_STACK] = Job {
        ray: cam.ray_for_pixel(x, y),
        weight: Color { r: 1.0, g: 1.0, b: 1.0 },
        remaining: cam.max_depth,
    };
    sp[idx] = 1;
}

// Pop this pixel's top job, trace its ray, and (on a hit) fill the per-pixel
// `WfNode` via the book's `prepare_computations`. `active=0` marks no work / miss.
#[spirv(compute(threads(8, 8)))]
pub fn wf_trace_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] objects: &[Primitive],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] lights: &[Light],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] child_indices: &[usize],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] jobs: &[Job],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] sp: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 6)] nodes: &mut [WfNode],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 7)] any_active: &mut [u32],
) {
    let x = id.x;
    let y = id.y + cam.row_offset;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    let idx = (y * cam.hsize + x) as usize;
    let s = sp[idx];
    if s == 0 {
        nodes[idx].active = 0;
        return;
    }
    any_active[0] = 1;
    let s2 = s - 1;
    sp[idx] = s2;
    let job = jobs[idx * WF_STACK + s2 as usize];
    let scene = Scene { objects, lights, child_indices, use_bounds: true };
    let xs = scene.intersect_world(&job.ray);
    let hi = xs.hit_index();
    if hi == xs.len {
        nodes[idx].active = 0;
        return;
    }
    let comps = xs.xs[hi].prepare_computations(&job.ray, &scene, &xs);
    nodes[idx] = WfNode::from_comps(&comps, job.weight, job.remaining);
}

// Per active pixel, compute each light's shadow intensity at the hit (point light:
// 0/1; area light: lit fraction) via the book's `intensity_at`.
#[spirv(compute(threads(8, 8)))]
pub fn wf_shadow_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] objects: &[Primitive],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] lights: &[Light],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] child_indices: &[usize],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] nodes: &[WfNode],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] intensity: &mut [f32],
) {
    let x = id.x;
    let y = id.y + cam.row_offset;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    let idx = (y * cam.hsize + x) as usize;
    if nodes[idx].active == 0 {
        return;
    }
    let scene = Scene { objects, lights, child_indices, use_bounds: true };
    let over = nodes[idx].over_point;
    let mut li = 0usize;
    while li < lights.len() && li < WF_MAX_LIGHTS {
        intensity[idx * WF_MAX_LIGHTS + li] = scene.intensity_at(over, &lights[li]);
        li += 1;
    }
}

// Per active pixel: Phong-shade the hit (book `lightning`, per light, using the
// precomputed shadow intensities), accumulate weight*surface, then push the
// reflect/refract child jobs — the exact branching the CPU `color_at` does.
#[spirv(compute(threads(8, 8)))]
pub fn wf_shade_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] objects: &[Primitive],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] lights: &[Light],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] nodes: &[WfNode],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] intensity: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] jobs: &mut [Job],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 6)] sp: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 7)] accum: &mut [Color],
) {
    let x = id.x;
    let y = id.y + cam.row_offset;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    let idx = (y * cam.hsize + x) as usize;
    let node = nodes[idx];
    if node.active == 0 {
        return;
    }
    let object = &objects[node.object_id as usize];
    let mut surface = BLACK;
    let mut li = 0usize;
    while li < lights.len() && li < WF_MAX_LIGHTS {
        surface = surface
            + lightning(
                object,
                lights[li],
                node.point,
                node.eyev,
                node.normalv,
                intensity[idx * WF_MAX_LIGHTS + li],
            );
        li += 1;
    }
    accum[idx] = accum[idx] + surface * node.weight;

    if node.remaining == 0 {
        return;
    }
    let material = object.get_material();
    let reflective = material.reflective;
    let transparency = material.transparency;
    if reflective == 0.0 && transparency == 0.0 {
        return;
    }
    let both = reflective > 0.0 && transparency > 0.0;
    let reflectance = if both { node.schlick() } else { 1.0 };
    let tir = node.tir();

    if reflective > 0.0 {
        let s = sp[idx];
        if (s as usize) < WF_STACK {
            let w = if both { reflective * reflectance } else { reflective };
            jobs[idx * WF_STACK + s as usize] = Job {
                ray: Ray { origin: node.over_point, direction: node.reflectv },
                weight: node.weight * w,
                remaining: node.remaining - 1,
            };
            sp[idx] = s + 1;
        }
    }
    if transparency > 0.0 && !tir {
        let s = sp[idx];
        if (s as usize) < WF_STACK {
            let direction = node.refract_dir();
            let w = if both { transparency * (1.0 - reflectance) } else { transparency };
            jobs[idx * WF_STACK + s as usize] = Job {
                ray: Ray { origin: node.under_point, direction },
                weight: node.weight * w,
                remaining: node.remaining - 1,
            };
            sp[idx] = s + 1;
        }
    }
}

// Pack the accumulated linear color into the 0x00RRGGBB framebuffer.
#[spirv(compute(threads(8, 8)))]
pub fn wf_present_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] cam: &Cam,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] accum: &[Color],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] out: &mut [u32],
) {
    let x = id.x;
    let y = id.y + cam.row_offset;
    if x >= cam.hsize || y >= cam.vsize {
        return;
    }
    let idx = (y * cam.hsize + x) as usize;
    out[idx] = pack_color(accum[idx]);
}
