//! AOT compiler — runs the 7-pass optimization chain then calls the codegen backend.

use crate::passes::*;
use crate::{CodegenBackend, OptPass};
use rava_common::error::{RavaError, Result};
use rava_rir::Module;
use std::path::Path;

/// Compiles an optimized RIR module to a native binary.
///
/// The default pass chain matches §25.1:
///   EscapeAnalysis → Inlining → DCE → ConstFolding →
///   MetadataTableGen → ProxyPregen → MicroRtBridge → CodegenBackend
pub struct AotCompiler {
    passes: Vec<Box<dyn OptPass>>,
    backend: Box<dyn CodegenBackend>,
}

impl AotCompiler {
    pub fn new(backend: Box<dyn CodegenBackend>) -> Self {
        Self {
            passes: Vec::new(),
            backend,
        }
    }

    /// Build a compiler with the full default pass chain (§25.1).
    pub fn with_default_passes(backend: Box<dyn CodegenBackend>) -> Self {
        let mut c = Self::new(backend);
        c.add_pass(Box::new(EscapeAnalysisPass));
        c.add_pass(Box::new(InliningPass));
        c.add_pass(Box::new(DeadCodeElimPass));
        c.add_pass(Box::new(ConstFoldingPass));
        c.add_pass(Box::new(MetadataTableGenPass));
        c.add_pass(Box::new(ProxyPregenPass));
        c.add_pass(Box::new(MicroRtBridgePass));
        c
    }

    /// Register an additional optimization pass (appended after the default chain).
    pub fn add_pass(&mut self, pass: Box<dyn OptPass>) {
        self.passes.push(pass);
    }

    /// Run all passes then emit native code to `output`.
    pub fn compile(&self, module: &mut Module, output: &Path) -> Result<()> {
        for pass in &self.passes {
            pass.run(module)
                .map_err(|e| RavaError::Codegen(format!("pass '{}' failed: {e}", pass.name())))?;
        }
        self.backend.emit(module, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rava_rir::Module;

    struct StubBackend;
    impl CodegenBackend for StubBackend {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn emit(&self, _m: &Module, _o: &Path) -> Result<()> {
            Err(RavaError::Codegen("stub".into()))
        }
    }

    #[test]
    fn default_passes_are_registered() {
        let compiler = AotCompiler::with_default_passes(Box::new(StubBackend));
        assert_eq!(compiler.passes.len(), 7);
    }

    #[test]
    fn passes_run_in_order() {
        let compiler = AotCompiler::with_default_passes(Box::new(StubBackend));
        let names: Vec<_> = compiler.passes.iter().map(|p| p.name()).collect();
        assert_eq!(
            names,
            [
                "escape-analysis",
                "inlining",
                "dead-code-elim",
                "const-folding",
                "metadata-table-gen",
                "proxy-pregen",
                "micrort-bridge",
            ]
        );
    }
}
