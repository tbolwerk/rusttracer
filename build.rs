//! Build script. With `--features gpu`, (re)compiles the rust-gpu compute shader
//! (gpu/shader) to SPIR-V at gpu/spv/raycore_shader.spv BEFORE the host crate is
//! compiled, so `gpu.rs`'s `include_bytes!("gpu/spv/raycore_shader.spv")` always
//! picks up a shader matching the current `raycore`. This is what lets a single
//! `cargo run --release --features gpu` rebuild the shader and render in one step.
//!
//! Without the `gpu` feature this is a no-op (the pure-CPU build needs no shader
//! and must not require the rust-gpu toolchain / cargo-gpu).

use std::path::Path;
use std::process::Command;

fn main() {
    // Cargo sets CARGO_FEATURE_<NAME> for every enabled feature. Only build the
    // shader when the `gpu` feature is on.
    if std::env::var_os("CARGO_FEATURE_GPU").is_none() {
        return;
    }

    // Rebuild the SPIR-V whenever the shader crate or the shared core changes:
    // the GPU buffer layout comes from raycore, so a stale .spv renders garbage.
    println!("cargo:rerun-if-changed=gpu/shader/src");
    println!("cargo:rerun-if-changed=gpu/shader/Cargo.toml");
    println!("cargo:rerun-if-changed=raycore/src");

    // `cargo gpu` fetches/pins the rust-gpu toolchain and emits the .spv. It does
    // its own up-to-date check, so this is fast when nothing changed.
    let status = Command::new("cargo")
        .args([
            "gpu",
            "build",
            "--shader-crate",
            "gpu/shader",
            "--output-dir",
            "gpu/spv",
            "--force-overwrite-lockfiles-v4-to-v3",
        ])
        .status();

    let spv = Path::new("gpu/spv/raycore_shader.spv");
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            // The build command ran but failed. If we have no usable shader at all,
            // there's no point continuing — fail loudly with the cause.
            if !spv.exists() {
                panic!(
                    "`cargo gpu build` failed (exit {s}) and gpu/spv/raycore_shader.spv \
                     does not exist. Install cargo-gpu and see gpu/README.md."
                );
            }
            println!(
                "cargo:warning=`cargo gpu build` failed (exit {s}); using the existing \
                 gpu/spv/raycore_shader.spv (may be stale)."
            );
        }
        Err(e) => {
            if !spv.exists() {
                panic!(
                    "could not run `cargo gpu` ({e}) and gpu/spv/raycore_shader.spv does \
                     not exist. Install it with `cargo install --git \
                     https://github.com/Rust-GPU/cargo-gpu cargo-gpu` (see gpu/README.md)."
                );
            }
            println!(
                "cargo:warning=could not run `cargo gpu` ({e}); using the existing \
                 gpu/spv/raycore_shader.spv (may be stale)."
            );
        }
    }
}
