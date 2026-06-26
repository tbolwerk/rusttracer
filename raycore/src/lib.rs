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
