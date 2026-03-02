//! `CraneliftBackend` — translates RIR to native object files via Cranelift.
//!
//! # Binary generation pipeline
//!
//! ```text
//! RirModule
//!   │
//!   ▼ RirFunction → CLIF IR (via FunctionBuilder)
//!   │   - Each RirInstr maps to one or more CLIF instructions
//!   │   - BasicBlock params → CLIF block params (already SSA, no phi needed)
//!   │
//!   ▼ cranelift-codegen: CLIF IR → machine code
//!   │   - Target: native host triple (or cross-compile target)
//!   │   - Optimization level: speed / size / none
//!   │
//!   ▼ cranelift-object: machine code → ELF/Mach-O/COFF object file
//!   │
//!   ▼ system linker (lld / ld / link.exe): object file → native binary
//! ```

use rava_aot::CodegenBackend;
use rava_common::error::{RavaError, Result};
use rava_rir::Module;
use std::path::Path;

use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

/// The default AOT codegen backend, powered by Cranelift.
///
/// Emits a native object file, then invokes the system linker to produce
/// the final binary.
pub struct CraneliftBackend {
    /// Target triple (defaults to the host triple).
    target: Triple,
    /// Cranelift optimization level: "speed", "speed_and_size", or "none".
    opt_level: &'static str,
}

impl CraneliftBackend {
    /// Create a backend targeting the host platform with speed optimization.
    pub fn new() -> Self {
        Self {
            target: Triple::host(),
            opt_level: "speed",
        }
    }

    /// Create a backend for a specific target triple (cross-compilation).
    pub fn for_target(target: Triple) -> Self {
        Self {
            target,
            opt_level: "speed",
        }
    }

    pub fn with_opt_level(mut self, level: &'static str) -> Self {
        self.opt_level = level;
        self
    }

    /// Build a Cranelift `ObjectModule` configured for our target.
    fn make_object_module(&self) -> Result<ObjectModule> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", self.opt_level)
            .map_err(|e| RavaError::Codegen(format!("cranelift flag error: {e}")))?;
        // Enable frame pointers for better stack unwinding / GC stack maps.
        flag_builder
            .set("preserve_frame_pointers", "true")
            .map_err(|e| RavaError::Codegen(format!("cranelift flag error: {e}")))?;
        // Enable PIC for macOS compatibility (required for linking).
        flag_builder
            .set("is_pic", "true")
            .map_err(|e| RavaError::Codegen(format!("cranelift flag error: {e}")))?;

        let flags = settings::Flags::new(flag_builder);
        let isa = isa::lookup(self.target.clone())
            .map_err(|e| RavaError::Codegen(format!("unsupported target {}: {e}", self.target)))?
            .finish(flags)
            .map_err(|e| RavaError::Codegen(format!("ISA init failed: {e}")))?;

        let obj_builder = ObjectBuilder::new(
            isa,
            "rava_output",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| RavaError::Codegen(format!("object builder error: {e}")))?;

        Ok(ObjectModule::new(obj_builder))
    }

    /// Write the finished object module to a `.o` file, then link to a binary.
    fn link(&self, obj_module: ObjectModule, output: &Path) -> Result<()> {
        // Finalize all function definitions and get the object product.
        let product = obj_module.finish();
        let obj_bytes = product
            .emit()
            .map_err(|e| RavaError::Codegen(format!("object emission failed: {e}")))?;

        // Write the object file next to the output binary.
        let obj_path = output.with_extension("o");
        std::fs::write(&obj_path, &obj_bytes).map_err(RavaError::Io)?;

        // Invoke the system linker.
        self.invoke_linker(&obj_path, output)?;

        // Clean up the intermediate object file.
        let _ = std::fs::remove_file(&obj_path);
        Ok(())
    }

    /// Invoke the system linker to produce the final native binary.
    fn invoke_linker(&self, obj_path: &Path, output: &Path) -> Result<()> {
        let rt_lib_dir = rava_rt::lib_dir();

        let status = std::process::Command::new("cc")
            .arg(obj_path)
            .arg("-o")
            .arg(output)
            .arg(format!("-L{rt_lib_dir}"))
            .arg("-lrava_rt")
            .arg("-lm")
            .status()
            .map_err(|e| RavaError::Codegen(format!("failed to invoke linker: {e}")))?;

        if !status.success() {
            return Err(RavaError::Codegen(format!(
                "linker exited with status {status}"
            )));
        }
        Ok(())
    }
}

impl Default for CraneliftBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for CraneliftBackend {
    fn name(&self) -> &'static str {
        "cranelift"
    }

    fn emit(&self, module: &Module, output: &Path) -> Result<()> {
        let mut obj_module = self.make_object_module()?;

        // Translate RIR → Cranelift IR → machine code
        crate::translator::translate_module(module, &mut obj_module)?;

        self.link(obj_module, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_name_is_cranelift() {
        assert_eq!(CraneliftBackend::new().name(), "cranelift");
    }

    #[test]
    fn make_object_module_succeeds_for_host() {
        // Verify Cranelift can be initialized for the host target.
        let backend = CraneliftBackend::new();
        assert!(backend.make_object_module().is_ok());
    }
}
