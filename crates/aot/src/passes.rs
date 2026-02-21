//! The 7 named optimization passes from §25.1.
//!
//! Each pass implements [`OptPass`] and is registered in [`AotCompiler::default_passes`].

use rava_common::error::Result;
use rava_rir::Module;
use crate::OptPass;

/// Pass 1 — Escape analysis: decide stack vs heap allocation for each `New` instruction.
pub struct EscapeAnalysisPass;
impl OptPass for EscapeAnalysisPass {
    fn name(&self) -> &'static str { "escape-analysis" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-1): implement escape analysis
        Ok(())
    }
}

/// Pass 2 — Inlining: inline methods smaller than 32 bytecodes.
pub struct InliningPass;
impl OptPass for InliningPass {
    fn name(&self) -> &'static str { "inlining" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-1): implement inlining
        Ok(())
    }
}

/// Pass 3 — Dead code elimination: remove unreachable basic blocks and dead values.
pub struct DeadCodeElimPass;
impl OptPass for DeadCodeElimPass {
    fn name(&self) -> &'static str { "dead-code-elim" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-1): implement DCE
        Ok(())
    }
}

/// Pass 4 — Constant folding: evaluate constant expressions at compile time.
pub struct ConstFoldingPass;
impl OptPass for ConstFoldingPass {
    fn name(&self) -> &'static str { "const-folding" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-1): implement constant folding
        Ok(())
    }
}

/// Pass 5 — Metadata table generation: embed reflection metadata in the binary (Phase 2).
pub struct MetadataTableGenPass;
impl OptPass for MetadataTableGenPass {
    fn name(&self) -> &'static str { "metadata-table-gen" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-2): generate ClassMetadata table for all classes
        Ok(())
    }
}

/// Pass 6 — Proxy pre-generation: AOT-compile proxy classes for known interface combos (Phase 4).
pub struct ProxyPregenPass;
impl OptPass for ProxyPregenPass {
    fn name(&self) -> &'static str { "proxy-pregen" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-4): pre-generate proxy classes
        Ok(())
    }
}

/// Pass 7 — MicroRT bridge: generate bridging stubs for MicroRtReflect/Proxy/ClassLoad instructions.
pub struct MicroRtBridgePass;
impl OptPass for MicroRtBridgePass {
    fn name(&self) -> &'static str { "micrort-bridge" }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-3): generate MicroRT interop stubs
        Ok(())
    }
}
