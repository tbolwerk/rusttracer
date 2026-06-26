//! GPU backend (only compiled with `--features gpu`). Uploads a `World`'s flat
//! scene buffers + camera to the GPU and dispatches the precompiled SPIR-V
//! compute shader (gpu/shader, built separately — see gpu/README.md), which runs
//! the SAME `raycore::render::pixel_color` as the CPU path. Returns the framebuffer
//! as packed 0x00RRGGBB pixels.
//!
//! UNVERIFIED IN THE SANDBOX: this needs a real adapter and the compiled
//! `gpu/spv/raycore_shader.spv`. The wgpu API below targets wgpu 0.20; adjust to
//! your version. The struct byte layouts must match the shader's `#[repr(C)]`
//! views — if you see garbage, suspect std430 padding (pad Color/Point to 16
//! bytes, widen `bool` fields to `u32`) before anything else.

use pollster::FutureExt as _;
use raycore::render::Cam;
use raycore::worlds::World;
use wgpu::util::DeviceExt;

// Reinterpret a slice of repr(C) values as raw bytes for upload.
fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(slice.as_ptr() as *const u8, core::mem::size_of_val(slice))
    }
}

/// Render `world` through `cam` on the GPU, returning `hsize*vsize` packed pixels
/// (0x00RRGGBB), row-major. Falls back to a panic with a clear message if no GPU
/// adapter or shader is available; callers should fall back to the CPU path.
pub fn render_gpu(world: &World, cam: &Cam) -> Vec<u32> {
    let hsize = cam.hsize as usize;
    let vsize = cam.vsize as usize;
    let pixels = hsize * vsize;

    // --- wgpu setup -------------------------------------------------------
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .block_on()
        .expect("no GPU adapter (run without --features gpu for the CPU path)");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .block_on()
        .expect("failed to create GPU device");

    // --- load the precompiled SPIR-V shader -------------------------------
    // Build it first: see gpu/README.md (`cargo gpu build` / spirv-builder).
    let spv = std::fs::read("gpu/spv/raycore_shader.spv")
        .expect("gpu/spv/raycore_shader.spv missing — build the shader (gpu/README.md)");
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("raycore_shader"),
        source: wgpu::ShaderSource::SpirV(wgpu::util::make_spirv_raw(&spv)),
    });

    // --- scene buffers ----------------------------------------------------
    // child_indices is Vec<usize> on the host; the shader's `usize` is 32-bit, so
    // upload it as u32.
    let child_indices_u32: Vec<u32> = world.child_indices.iter().map(|&i| i as u32).collect();

    let storage = |label, data: &[u8]| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage: wgpu::BufferUsages::STORAGE,
        })
    };
    let objects_buf = storage("objects", as_bytes(&world.objects));
    let lights_buf = storage("lights", as_bytes(&world.lights));
    let child_buf = storage("child_indices", as_bytes(&child_indices_u32));
    let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cam"),
        contents: as_bytes(&[*cam]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let out_size = (pixels * core::mem::size_of::<u32>()) as u64;
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("out"),
        size: out_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: out_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // --- pipeline + bind group -------------------------------------------
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("raycore"),
        layout: None,
        module: &module,
        entry_point: "main_cs",
        compilation_options: Default::default(),
        cache: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: objects_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: lights_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: child_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: cam_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: out_buf.as_entire_binding() },
        ],
    });

    // --- dispatch ---------------------------------------------------------
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass =
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // 8x8 workgroup (matches threads(8,8) in the shader).
        let gx = ((hsize as u32) + 7) / 8;
        let gy = ((vsize as u32) + 7) / 8;
        pass.dispatch_workgroups(gx, gy, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging, 0, out_size);
    queue.submit(Some(encoder.finish()));

    // --- read back --------------------------------------------------------
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();
    let out: Vec<u32> = bytemuck_cast_u32(&data);
    drop(data);
    staging.unmap();
    out
}

// Minimal &[u8] -> Vec<u32> (the output is exactly pixels*4 bytes, 4-aligned).
fn bytemuck_cast_u32(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
