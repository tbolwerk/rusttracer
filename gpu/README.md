# GPU backend (rust-gpu + wgpu)

The renderer core (`raycore`) is `no_std`, heap-free, recursion-free and free of
data-carrying enums, so the *same* Rust code that runs on the CPU compiles to a
SPIR-V compute shader via [rust-gpu]. One source of truth, two backends.

```
raycore (no_std core: math + Scene + trace + render::pixel_color)
   |                                   |
   | default-features (std)            | default-features = false
   v                                   v
rusttracer (CPU host, this crate)   gpu/shader (SPIR-V compute shader)
                                        |  builds to ->  gpu/spv/raycore_shader.spv
   rusttracer --features gpu  --------> gpu.rs (wgpu) uploads buffers + dispatches
```

## What is done (CPU-verified)

- `raycore` compiles `no_std`: `cargo build -p raycore --no-default-features --features cpu-math`.
  (The math backend is a feature: `cpu-math` = num-traits/libm for CPU/no_std;
  `gpu` = spirv-std intrinsics for the shader. The shader crate uses `gpu`.)
- `gpu/shader/src/lib.rs` — the compute entry calling `raycore::render::pixel_color`.
- `gpu.rs` — wgpu host that uploads the scene and dispatches (behind `--features gpu`).
- All data types are `#[repr(C)]` with `Option` fields flattened to sentinels.

## What you must do on your machine (NOT verifiable in CI/sandbox)

### 1. Build the shader to SPIR-V

Easiest is [`cargo gpu`](https://github.com/Rust-GPU/cargo-gpu):

```sh
cargo install cargo-gpu
cd gpu/shader
cargo gpu build --output-dir ../spv        # produces ../spv/raycore_shader.spv
```

Or drive `spirv-builder` from a build script. Either way:

- **Pin versions that match.** `gpu/shader/Cargo.toml` (`spirv-std`) and
  `gpu/shader/rust-toolchain.toml` (nightly channel + `rust-src`/`rustc-dev`/
  `llvm-tools`) must agree with each other and with your `cargo gpu`. The values
  checked in are placeholders.

### 2. Run the GPU path

```sh
cargo run --release --features gpu -- <your gpu entry>
```

`gpu.rs` reads `gpu/spv/raycore_shader.spv` at runtime. `wgpu` is pinned to 0.20
in the root `Cargo.toml`; if you use another version, adjust the `gpu.rs` API
calls (wgpu changes between releases).

## Known issues to expect during bring-up (in priority order)

1. **Buffer alignment (std430).** Layouts come from `raycore`'s `#[repr(C)]`
   structs. The validator may reject 12-byte `Color`/`Point`/`Vector` or the
   `bool` fields (`Primitive.closed`, `Primitive.has_bounds`). Fixes: pad those
   vec3s to 16 bytes and widen the `bool`s to `u32` (0/1). The math reads them the
   same way; only the layout changes.
2. **`usize` width.** `Scene::child_indices` is `&[usize]`; rust-gpu lowers
   `usize` to 32-bit, so `gpu.rs` already uploads that buffer as `u32`. Object ids
   inside `Primitive` (`child_start`/`child_count`/`left`/`right`/`parent`) are
   already `u32`.
3. **Fixed scratch sizes.** The trace uses fixed stacks/buffers
   (`MAX_TRAVERSAL_STACK`, `MAX_TREE_DEPTH`, `MAX_SHADE_STACK` = 64 in worlds.rs;
   `MAX_XS` = 256 in intersections.rs). Large registers per invocation can hurt
   GPU occupancy or overflow; tune for your scenes/hardware.
4. **Transcendental math.** `no_std` math comes from `num_traits::Float` (libm).
   rust-gpu may prefer SPIR-V `GLSL.std.450` intrinsics; if `sqrt`/`sin`/etc. fail
   to lower or are slow, route them through `spirv_std` intrinsics behind
   `cfg(target_arch = "spirv")`.
5. **Host wiring.** `gpu.rs` exposes `render_gpu(&World, &Cam) -> Vec<u32>`. Build
   the `Cam` from your `Camera` (pixel_size / half_width / half_height / inverse
   view transform) and feed the packed pixels to your window or PPM writer.

[rust-gpu]: https://github.com/Rust-GPU/rust-gpu
