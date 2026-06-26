//! `raycore` is the shared ray-tracing core: the f32 vector/point/color/matrix
//! math plus the renderer (primitives, materials, patterns, lights, groups, CSG,
//! intersections and shading). It is the single source of truth used by both the
//! CPU host binary and (Stage 3) the rust-gpu SPIR-V shader.
//!
//! The crate is `no_std` unless the `std` feature is on (default). With `std`
//! off, only the heap-free core compiles (Scene + trace path + math + the flat
//! data types); the Vec-based `World` and scene building are `std`-only. This is
//! what lets rust-gpu compile the renderer to SPIR-V.
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
// `cfg(target_arch = "spirv")` is unknown to non-spirv toolchains' check-cfg, so
// they'd warn on it; rust-gpu sets it when compiling the shader.
#![allow(unexpected_cfgs)]
// rust-gpu pins an older nightly (1.71) where `const fn` with `&mut self` and
// float arithmetic in `const fn` are still unstable. They are stable on the
// host's modern nightly, so only enable the gates for the GPU build.
#![cfg_attr(feature = "gpu", feature(const_mut_refs))]
#![cfg_attr(feature = "gpu", feature(const_fn_floating_point_arithmetic))]

pub mod tuples;
pub mod matrices;
pub mod transformations;
pub mod rays;
pub mod materials;
pub mod patterns;
pub mod texture_maps;
pub mod lights;
pub mod bounds;
pub mod intersections;
pub mod shapes;
pub mod spheres;
pub mod planes;
pub mod cubes;
pub mod cylinders;
pub mod cones;
pub mod triangles;
pub mod groups;
pub mod csg;
pub mod worlds;
pub mod render;
