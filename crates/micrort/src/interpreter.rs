//! Bytecode interpreter — core execution engine.

use std::sync::Arc;
use rava_common::error::Result;
use rava_heap::UnifiedHeap;

/// Extension point: swap the bytecode dispatch strategy.
///
/// Default: `MatchDispatcher` (Rust `match` — safe, within 5–20% of computed-goto).
/// Alternative: a threaded/computed-goto dispatcher via unsafe Rust (Phase 5).
pub trait BytecodeDispatcher: Send + Sync {
    fn name(&self) -> &'static str;
    /// Execute a single bytecode instruction. Returns `true` if execution should continue.
    fn dispatch(&self, frame: &mut Frame, opcode: u8) -> Result<bool>;
}

/// A single call frame on the interpreter stack.
pub struct Frame {
    /// Program counter — index into the bytecode array.
    pub pc:            usize,
    /// Local variable slots (4 bytes each; long/double occupy 2 slots).
    pub locals:        Vec<u32>,
    /// Operand stack.
    pub operand_stack: Vec<rava_rir::StackValue>,
}

/// The MicroRT bytecode interpreter.
///
/// Uses an explicit frame stack (Vec) rather than recursion to avoid C stack overflow
/// on deeply nested Java call chains.
#[allow(dead_code)] // fields used in Phase 3 interpreter implementation
pub struct Interpreter {
    frames:      Vec<Frame>,
    heap:        Arc<std::sync::RwLock<UnifiedHeap>>,
    dispatcher:  Box<dyn BytecodeDispatcher>,
}

impl Interpreter {
    pub fn new(heap: Arc<std::sync::RwLock<UnifiedHeap>>, dispatcher: Box<dyn BytecodeDispatcher>) -> Self {
        Self { frames: Vec::new(), heap, dispatcher }
    }

    /// Invoke a method by its bytecode.
    pub fn invoke(&mut self, _bytecode: &[u8], _args: &[rava_rir::StackValue]) -> Result<Option<rava_rir::StackValue>> {
        // TODO(phase-3): implement main interpreter loop
        Err(rava_common::error::RavaError::Other("interpreter not yet implemented".into()))
    }
}
