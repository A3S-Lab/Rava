//! RIR module, function, and basic block structures (SSA form).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{BlockId, FuncId, RirInstr, RirType, Value};

/// A compiled Java source file or class — the top-level RIR unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RirModule {
    pub name:        String,
    pub functions:   Vec<RirFunction>,
    /// Maps FieldId hash → field name string, so the interpreter can reverse-lookup.
    pub field_names: HashMap<u32, String>,
    /// Maps FieldId hash → field type, for AOT type propagation.
    pub field_types: HashMap<u32, RirType>,
    /// Maps ClassId hash → class name string, so the interpreter can reverse-lookup.
    pub class_names: HashMap<u32, String>,
    /// Maps MethodId hash → method name string, so the interpreter can reverse-lookup.
    pub method_names: HashMap<u32, String>,
    /// Maps class name → superclass name (for instanceof chain walking).
    pub class_hierarchy: HashMap<String, String>,
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
        Self { name: name.into(), functions: Vec::new(), field_names: HashMap::new(), field_types: HashMap::new(), class_names: HashMap::new(), method_names: HashMap::new(), class_hierarchy: HashMap::new() }
    }

    /// Merge all functions and field names from `other` into this module.
    ///
    /// FuncId/BlockId values are local to each function's basic blocks (jumps
    /// within the same function), so no remapping is needed — we just append.
    pub fn merge(&mut self, other: RirModule) {
        self.functions.extend(other.functions);
        self.field_names.extend(other.field_names);
        self.field_types.extend(other.field_types);
        self.class_names.extend(other.class_names);
        self.method_names.extend(other.method_names);
        self.class_hierarchy.extend(other.class_hierarchy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_combines_functions_and_fields() {
        let mut a = RirModule::new("a");
        a.functions.push(RirFunction {
            id: FuncId(0),
            name: "A.main".into(),
            params: vec![],
            return_type: RirType::Void,
            basic_blocks: vec![],
            flags: FuncFlags::default(),
        });
        a.field_names.insert(100, "x".into());

        let mut b = RirModule::new("b");
        b.functions.push(RirFunction {
            id: FuncId(0),
            name: "B.run".into(),
            params: vec![],
            return_type: RirType::Void,
            basic_blocks: vec![],
            flags: FuncFlags::default(),
        });
        b.field_names.insert(200, "y".into());

        a.merge(b);
        assert_eq!(a.functions.len(), 2);
        assert_eq!(a.functions[0].name, "A.main");
        assert_eq!(a.functions[1].name, "B.run");
        assert_eq!(a.field_names.len(), 2);
        assert!(a.field_names.contains_key(&100));
        assert!(a.field_names.contains_key(&200));
    }
}
