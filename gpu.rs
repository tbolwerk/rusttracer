//! GPU backend (only compiled with `--features gpu`). Uploads a `World`'s flat
//! scene buffers + camera to the GPU and dispatches the precompiled SPIR-V
//! compute shader (gpu/shader, built by build.rs via `cargo gpu`), which runs the
//! SAME `raycore::render::pixel_color` as the CPU path. Returns the framebuffer as
//! packed 0x00RRGGBB pixels.
//!
//! The SPIR-V is fed to the driver via wgpu's *passthrough* path
//! (`create_shader_module_passthrough` + the `PASSTHROUGH_SHADERS` feature), NOT
//! `ShaderSource::SpirV`. rust-gpu emits SPIR-V that wgpu's naga validator rejects
//! (it uses capabilities/patterns naga can't model), so going through naga crashes
//! the driver. Passthrough hands the module to Vulkan as-is, which is what makes
//! the GPU path actually work.
//!
//! All five bindings are std430 storage buffers (none uniform): `Cam` embeds a
//! `Matrix<4,4>` whose 4-byte array stride is illegal under std140 (uniform) but
//! fine under std430 (storage). The shared structs are plain f32/u32 `#[repr(C)]`
//! `raycore` types, so rust-gpu lays them out exactly as Rust does and we upload
//! their raw bytes directly, no padding, no bytemuck.
//!
//! Written against wgpu 29.0.3.

use pollster::block_on;
use raycore::render::{Cam, Job, WfNode, WF_MAX_LIGHTS, WF_STACK};
use raycore::tuples::Color;
use raycore::worlds::World;
use std::borrow::Cow;
use wgpu::util::DeviceExt;

// The SPIR-V produced by `cargo gpu build --shader-crate gpu/shader --output-dir
// gpu/spv` (build.rs does this automatically under --features gpu). Embedded at
// compile time so the binary is self-contained; rebuilt whenever raycore or the
// shader changes.
const SHADER_SPV: &[u8] = include_bytes!("gpu/spv/raycore_shader.spv");

// Reinterpret a slice of repr(C) Copy structs as bytes for upload.
fn as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

// Pick which GPU to render on, among adapters that support SPIR-V passthrough
// (required to feed rust-gpu's module to the driver). Selection order:
//   1. If `WGPU_ADAPTER_NAME` is set, only adapters whose name contains it (case-
//      insensitive) are considered — the explicit way to force a specific GPU,
//      e.g. `WGPU_ADAPTER_NAME=nvidia` or a model substring for an eGPU.
//   2. Among the rest, prefer a discrete GPU (an eGPU/dedicated card) over an
//      integrated one over a software/other device — so plugging in an eGPU makes
//      it the default without any env var.
// Returns None (caller falls back to CPU) if nothing supports passthrough.
fn select_adapter(instance: &wgpu::Instance) -> Option<wgpu::Adapter> {
    let want = std::env::var("WGPU_ADAPTER_NAME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase());
    let mut candidates: Vec<wgpu::Adapter> = block_on(instance.enumerate_adapters(wgpu::Backends::all()))
        .into_iter()
        .filter(|a| a.features().contains(wgpu::Features::PASSTHROUGH_SHADERS))
        .filter(|a| match &want {
            Some(name) => a.get_info().name.to_lowercase().contains(name),
            None => true,
        })
        .collect();
    if candidates.is_empty() {
        eprintln!(
            "gpu: no passthrough-capable adapter{}; using CPU",
            want.map(|n| format!(" matching \"{n}\"")).unwrap_or_default()
        );
        return None;
    }
    // Lower rank = preferred. Discrete (eGPU) first.
    let rank = |t: wgpu::DeviceType| match t {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Cpu => 3,
        wgpu::DeviceType::Other => 4,
    };
    candidates.sort_by_key(|a| rank(a.get_info().device_type));
    candidates.into_iter().next()
}

// A live wavefront GPU context: device, queue, and the five compute pipelines
// (init / trace / shadow / shade / present), all built once. Building these is
// expensive, so an interactive fly loop keeps one `WavefrontRenderer` and only
// rebuilds the per-frame scene + path-state buffers. This is the wavefront
// replacement for the old single-megakernel renderer: the kernels are small
// enough for RADV's fast ACO compiler (no LLVM fallback) and short enough not to
// trip the GPU timeout watchdog.
pub struct WavefrontRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    p_init: wgpu::ComputePipeline,
    bgl_init: wgpu::BindGroupLayout,
    p_trace: wgpu::ComputePipeline,
    bgl_trace: wgpu::BindGroupLayout,
    p_shadow: wgpu::ComputePipeline,
    bgl_shadow: wgpu::BindGroupLayout,
    p_shade: wgpu::ComputePipeline,
    bgl_shade: wgpu::BindGroupLayout,
    p_present: wgpu::ComputePipeline,
    bgl_present: wgpu::BindGroupLayout,
}

impl WavefrontRenderer {
    // Build the context, or None if there's no usable GPU adapter. Does NOT force
    // RADV_DEBUG=llvm: the small wavefront kernels compile on ACO (the megakernel
    // that needed LLVM is gone).
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = select_adapter(&instance)?;
        let info = adapter.get_info();
        eprintln!(
            "gpu: using adapter \"{}\" ({:?}, {:?})",
            info.name, info.device_type, info.backend
        );
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("raycore"),
            required_features: wgpu::Features::PASSTHROUGH_SHADERS,
            // The wavefront kernels bind up to 8 storage buffers; request the
            // adapter's real limits so we aren't capped at the conservative defaults.
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .ok()?;

        // SPIR-V passthrough: skip naga and hand the module to the driver as-is.
        let words: Vec<u32> = SHADER_SPV
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let module = unsafe {
            device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                label: Some("raycore_shader"),
                spirv: Some(Cow::Owned(words)),
                ..Default::default()
            })
        };

        let (p_init, bgl_init) = storage_pipeline(&device, &module, "wf_init_cs", 4, &[1, 2, 3]);
        let (p_trace, bgl_trace) = storage_pipeline(&device, &module, "wf_trace_cs", 8, &[5, 6, 7]);
        let (p_shadow, bgl_shadow) = storage_pipeline(&device, &module, "wf_shadow_cs", 6, &[5]);
        let (p_shade, bgl_shade) = storage_pipeline(&device, &module, "wf_shade_cs", 8, &[5, 6, 7]);
        let (p_present, bgl_present) = storage_pipeline(&device, &module, "wf_present_cs", 3, &[2]);

        Some(WavefrontRenderer {
            device,
            queue,
            p_init,
            bgl_init,
            p_trace,
            bgl_trace,
            p_shadow,
            bgl_shadow,
            p_shade,
            bgl_shade,
            p_present,
            bgl_present,
        })
    }

    // Render one frame of `world` through `cam`, returning hsize*vsize packed
    // pixels (0x00RRGGBB), row-major. Reuses the device/pipelines; only the
    // per-frame buffers are rebuilt. The world must have had `compute_bounds()` run.
    pub fn render(&self, world: &World, cam: &Cam) -> Vec<u32> {
        use core::mem::size_of;
        let device = &self.device;
        let queue = &self.queue;
        let w = cam.hsize;
        let h = cam.vsize;
        let pixels = (w as usize) * (h as usize);
        let out_bytes = (pixels * 4) as u64;

        let mut child_u32: Vec<u32> = world.child_indices.iter().map(|&i| i as u32).collect();
        if child_u32.is_empty() {
            child_u32.push(0);
        }
        let st = wgpu::BufferUsages::STORAGE;
        let init_buf = |label, data: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents: data, usage: st })
        };
        let scratch = |label, size: u64, extra: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size, usage: st | extra, mapped_at_creation: false })
        };

        let objects_b = init_buf("objects", as_bytes(&world.objects));
        let lights_b = init_buf("lights", as_bytes(&world.lights));
        let child_b = init_buf("child", as_bytes(&child_u32));
        let cam_b = init_buf("cam", as_bytes(&[*cam]));

        // Per-pixel path state.
        let jobs_b = scratch("jobs", (pixels * WF_STACK * size_of::<Job>()) as u64, wgpu::BufferUsages::empty());
        let sp_b = scratch("sp", (pixels * 4) as u64, wgpu::BufferUsages::empty());
        let nodes_b = scratch("nodes", (pixels * size_of::<WfNode>()) as u64, wgpu::BufferUsages::empty());
        let intensity_b = scratch("intensity", (pixels * WF_MAX_LIGHTS * 4) as u64, wgpu::BufferUsages::empty());
        let accum_b = scratch("accum", (pixels * size_of::<Color>()) as u64, wgpu::BufferUsages::empty());
        let out_b = scratch("out", out_bytes, wgpu::BufferUsages::COPY_SRC);
        let any_b = scratch("any_active", 4, wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        let any_rb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("any_rb"), size: 4, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"), size: out_bytes, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });

        fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, bufs: &[&wgpu::Buffer]) -> wgpu::BindGroup {
            let entries: Vec<wgpu::BindGroupEntry> = bufs
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout, entries: &entries })
        }
        let bg_init = make_bg(device, &self.bgl_init, &[&cam_b, &jobs_b, &sp_b, &accum_b]);
        let bg_trace = make_bg(device, &self.bgl_trace, &[&objects_b, &lights_b, &child_b, &cam_b, &jobs_b, &sp_b, &nodes_b, &any_b]);
        let bg_shadow = make_bg(device, &self.bgl_shadow, &[&objects_b, &lights_b, &child_b, &cam_b, &nodes_b, &intensity_b]);
        let bg_shade = make_bg(device, &self.bgl_shade, &[&objects_b, &lights_b, &cam_b, &nodes_b, &intensity_b, &jobs_b, &sp_b, &accum_b]);
        let bg_present = make_bg(device, &self.bgl_present, &[&cam_b, &accum_b, &out_b]);

        let gx = w.div_ceil(8);
        let gy = h.div_ceil(8);
        let dispatch = |encoder: &mut wgpu::CommandEncoder, pipeline: &wgpu::ComputePipeline, group: &wgpu::BindGroup| {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(gx, gy, 1);
        };

        // Seed primary rays.
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        dispatch(&mut enc, &self.p_init, &bg_init);
        queue.submit(Some(enc.finish()));

        // Bounce loop: trace, stop once no pixel has a job, else shadow + shade.
        for _ in 0..WF_MAX_ROUNDS {
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            enc.clear_buffer(&any_b, 0, None);
            dispatch(&mut enc, &self.p_trace, &bg_trace);
            enc.copy_buffer_to_buffer(&any_b, 0, &any_rb, 0, 4);
            queue.submit(Some(enc.finish()));

            let slice = any_rb.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let any = u32::from_ne_bytes(slice.get_mapped_range()[0..4].try_into().unwrap());
            any_rb.unmap();
            if any == 0 {
                break;
            }

            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            dispatch(&mut enc, &self.p_shadow, &bg_shadow);
            dispatch(&mut enc, &self.p_shade, &bg_shade);
            queue.submit(Some(enc.finish()));
        }

        // Pack the accumulator and read it back.
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        dispatch(&mut enc, &self.p_present, &bg_present);
        enc.copy_buffer_to_buffer(&out_b, 0, &readback, 0, out_bytes);
        queue.submit(Some(enc.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let data = slice.get_mapped_range();
        let out: Vec<u32> = data.chunks_exact(4).map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]])).collect();
        drop(data);
        readback.unmap();
        out
    }

    /// Like `render`, but catches a "device lost" panic and returns `None`, so a
    /// caller can fall back to the CPU. The device is dead after a loss, so the
    /// one-shot path rebuilds it next call and the fly viewer drops to CPU.
    pub fn render_caught(&self, world: &World, cam: &Cam) -> Option<Vec<u32>> {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let rendered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.render(world, cam)));
        std::panic::set_hook(prev_hook);
        rendered.ok()
    }
}

// Build a compute pipeline for `entry` with `n` sequential storage-buffer
// bindings; bindings whose index is in `read_write` are read/write, the rest
// read-only. Returns (pipeline, bind_group_layout).
fn storage_pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    entry: &str,
    n: u32,
    read_write: &[u32],
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let entries: Vec<wgpu::BindGroupLayoutEntry> = (0..n)
        .map(|i| wgpu::BindGroupLayoutEntry {
            binding: i,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: !read_write.contains(&i),
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        })
        .collect();
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(entry),
        entries: &entries,
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(entry),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: Some(&layout),
        module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    });
    (pipeline, bgl)
}

// Safety cap on wavefront bounce rounds (the host loops trace/shadow/shade until
// every pixel's job stack drains; this bounds a runaway). A Whitted tree of depth
// `max_depth` with both-reflective-and-transparent branching needs at most a few
// dozen rounds; the early-exit flag normally stops far sooner.
const WF_MAX_ROUNDS: u32 = 64;

/// One-shot wavefront render of `world` through `cam`. Builds a context, renders,
/// and returns the framebuffer. `None` if there's no usable GPU adapter or the
/// device was lost mid-render (caller falls back to CPU). The next call rebuilds.
pub fn render_gpu(world: &World, cam: &Cam) -> Option<Vec<u32>> {
    let pixels = WavefrontRenderer::new()?.render_caught(world, cam);
    if pixels.is_none() {
        eprintln!("gpu: lost the device mid-render; CPU fallback");
    }
    pixels
}
