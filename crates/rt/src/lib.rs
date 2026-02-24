//! rava-rt: Rust crate that bundles the C runtime library.
//!
//! The build.rs compiles src/lib.c into a static library that gets linked
//! into rava-compiled binaries via `rava build`.
//!
//! This crate also exposes the path to the compiled static library so the
//! CLI can pass it to the system linker.

/// Returns the directory containing the compiled `librava_rt.a`.
pub fn lib_dir() -> &'static str {
    env!("OUT_DIR")
}
