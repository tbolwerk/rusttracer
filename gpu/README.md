# GPU backend (rust-gpu + wgpu, wavefront)

The renderer core (`raycore`) is `no_std`, heap-free, recursion-free and free of
data-carrying enums, so the *same* Rust code that runs on the CPU compiles to
SPIR-V compute shaders via [rust-gpu]. One source of truth, two backends.

```
raycore (no_std core: math + Scene + intersect/shade + the wf_* kernel bodies)
   |                                   |
   | default-features (std)            | default-features = false, features = ["gpu"]
   v                                   v
rusttracer (CPU host, this crate)   gpu/shader (SPIR-V wavefront kernels)
                                        |  built by build.rs -> gpu/spv/raycore_shader.spv
   rusttracer --features gpu  --------> gpu.rs (wgpu) drives the kernels per bounce
```

## Running

One command (re)builds the SPIR-V shader, then renders.

```sh
cargo run --release --features gpu          # render every chapter on the GPU
cargo run --release                          # render every chapter on the CPU
cargo run --release --features gpu -- fly    # interactive viewer, GPU per frame
cargo run --release            -- fly        # interactive viewer, CPU per frame
```

The backend is chosen purely by the `gpu` feature; without it the wgpu/pollster
dependency tree and the rust-gpu toolchain aren't needed at all.

`build.rs` runs `cargo gpu build` automatically under `--features gpu`, so the
shader always matches the current `raycore`. It re-runs only when `gpu/shader/` or
`raycore/src/` changes.

### Selecting a GPU (e.g. an eGPU)

Among adapters that support SPIR-V passthrough, the host prefers a **discrete GPU**
(eGPU / dedicated card) over an integrated one — so plugging in an eGPU makes it
the default. To force a specific card, set `WGPU_ADAPTER_NAME` to a case-insensitive
substring of its name (e.g. `WGPU_ADAPTER_NAME=6600` or `=nvidia`). The chosen
adapter is printed at startup.

### Prerequisites (GPU build only)

- [`cargo gpu`](https://github.com/Rust-GPU/cargo-gpu): `cargo install --git
  https://github.com/Rust-GPU/cargo-gpu cargo-gpu`. It pins the exact rust-gpu
  nightly, so `gpu/shader/rust-toolchain.toml` + `gpu/shader/Cargo.toml`
  (`spirv-std`) just need to agree with it.
- A Vulkan driver whose adapter supports `PASSTHROUGH_SHADERS`. If none is found
  (or the build lacks the `gpu` feature), rendering uses the CPU.

## Why wavefront (not a single megakernel)

A single kernel that traces a whole pixel's path inlines the entire renderer into
one ~24,000-instruction function. That's the classic **megakernel** problem
([Laine et al., "Megakernels Considered Harmful"][mk]; [PBRT-v4 §15][pbrt];
[Blender Cycles][cycles]): huge code, high register pressure, long-running threads.
Concretely on RADV it made the **ACO compiler segfault** (forcing the slow LLVM
backend) and tripped the **GPU timeout watchdog** on heavy scenes (teapot, the
280-marble field), which then fell back to the CPU.

The fix, as in production GPU path tracers, is to split the renderer into small
kernels connected by **per-pixel global buffers**, and loop over bounces on the
host. The kernels (`gpu/shader/src/lib.rs`):

1. **`wf_init`** — seed each pixel's job stack with the primary ray.
2. **`wf_trace`** — pop a pixel's top job, trace its ray (`intersect_world`), and
   fill the per-pixel `WfNode` via the book's `prepare_computations`.
3. **`wf_shadow`** — per active pixel, each light's shadow intensity
   (`intensity_at`).
4. **`wf_shade`** — Phong shade (`lightning`), accumulate, and push the
   reflect/refract child jobs (the exact branching the CPU `color_at` recurses).
5. **`wf_present`** — pack the accumulator into the framebuffer.

`gpu.rs` (`WavefrontRenderer`) builds these five pipelines once and, per frame,
loops `trace → shadow → shade` until every pixel's job stack drains (an early-exit
flag stops as soon as no pixel has work). The per-pixel branching reflect/refract
tree the book recurses through is held as a **job stack in global memory**
(`Job` / `WfNode` / `WF_STACK` in `raycore::render`), so results match the CPU
exactly. The shading/intersection logic is the same raycore functions — only the
driver differs from the CPU's recursive `color_at`.

Result: each kernel is small enough for **ACO** (no LLVM fallback, no
`RADV_DEBUG=llvm` needed) and short enough that the watchdog never fires — the
whole chapter sweep, including the marble field and teapot, renders on the GPU.

## SPIR-V passthrough

The module is handed to Vulkan as-is via `create_shader_module_passthrough` (+ the
`PASSTHROUGH_SHADERS` device feature), NOT `ShaderSource::SpirV`: rust-gpu emits
SPIR-V that wgpu's naga validator rejects, so the naga path crashes the driver.

## Layout rules (host `#[repr(C)]` must equal the shader's std430 view)

Scene/state structs are uploaded as raw bytes, so their host size must match
rust-gpu's layout, or data past the first element is read at the wrong stride.

- **No `usize` in uploaded structs.** rust-gpu lowers `usize` to 32-bit. `Light`'s
  `usteps`/`vsteps`/`samples`, the ids in `Primitive`, and `Job`/`WfNode` use
  `u32`; `Scene::child_indices` (`&[usize]`) is uploaded as `u32`. (Getting this
  wrong made every light read at the wrong offset.)
- **No `bool`** (`u32` 0/1) and **no `Option`** on the shader path (rust-gpu 0.9
  can't lower `Option<tuple>` / `Option<struct>`); `WfNode` exposes `tir()` +
  `refract_dir()` instead of an `Option`.
- `Color`/`Point`/`Vector` are plain 3×`f32` structs; `Matrix<4,4>` is `[[f32;4];4]`.
- All bindings are std430 storage buffers (no uniforms — `Matrix`'s 4-byte array
  stride is illegal under std140).

## Fixed scratch sizes

The trace uses fixed stacks/buffers sized for SPIR-V codegen (`MAX_TRAVERSAL_STACK`
= `MAX_TREE_DEPTH` = 32, `MAX_SHADE_STACK` = 16 in `worlds.rs`; `WF_STACK` = 16 in
`render.rs`). The CPU uses the same caps, so both backends agree.

[rust-gpu]: https://github.com/Rust-GPU/rust-gpu
[mk]: https://research.nvidia.com/sites/default/files/pubs/2013-07_Megakernels-Considered-Harmful/laine2013hpg_paper.pdf
[pbrt]: https://pbr-book.org/4ed/Wavefront_Rendering_on_GPUs/Mapping_Path_Tracing_to_the_GPU
[cycles]: https://wiki.blender.org/wiki/Source/Render/Cycles/KernelScheduling
