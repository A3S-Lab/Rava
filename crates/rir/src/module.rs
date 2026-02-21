//! RIR module, function, and basic block structures (SSA form).

use serde::{Deserialize, Serialize};
use crate::{BlockId, FuncId, RirInstr, RirType, Value};

/// A compiled Java source file or class — the top-level RIR unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RirModule {
    pub name:      String,
    pub functions: Vec<RirFunction>,
}

/// A single Java method in SSA form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RirFunction {
    pub id:           FuncId,
    pub name:         String,
    pub params:       Vec<(Value, RirType)>,
    pub return_type:  RirType,
    pub basic_blocks: Vec<BasicBlock>,  // first BB is the entry block
    pub flags:        FuncFlags,
}

/// A basic block — linear sequence of instructions, single entry, single exit.
///
/// Uses MLIR-style block parameters instead of explicit phi nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id:          BlockId,
    /// Block parameters act as phi functions (MLIR style).
    pub params:      Vec<(Value, RirType)>,
    pub instrs:      Vec<RirInstr>,
}

/// Flags on a function.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FuncFlags {
    pub is_clinit:       bool,  // static initializer
    pub is_constructor:  bool,
    pub is_synchronized: bool,
}

impl RirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), functions: Vec::new() }
    }
}
