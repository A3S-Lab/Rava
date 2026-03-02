//! The 7 named optimization passes from §25.1.
//!
//! Each pass implements [`OptPass`] and is registered in [`AotCompiler::default_passes`].

use crate::OptPass;
use rava_common::error::Result;
use rava_rir::instr::{BinOp, RirInstr, UnaryOp};
use rava_rir::Module;
use std::collections::{HashMap, HashSet};

/// Pass 1 — Escape analysis: decide stack vs heap allocation for each `New` instruction.
pub struct EscapeAnalysisPass;
impl OptPass for EscapeAnalysisPass {
    fn name(&self) -> &'static str {
        "escape-analysis"
    }
    fn run(&self, module: &mut Module) -> Result<()> {
        // Escape analysis determines if an object allocation can be done on the stack
        // instead of the heap. An object "escapes" if:
        // 1. It's returned from the function
        // 2. It's stored in a field or static variable
        // 3. It's passed to another function that might store it
        // 4. It's stored in an array that escapes

        for func in &module.functions {
            let mut escaping_values = HashSet::new();

            // Scan all instructions to find escaping allocations
            for bb in &func.basic_blocks {
                for instr in &bb.instrs {
                    match instr {
                        // Objects returned from function escape
                        RirInstr::Return(Some(val)) => {
                            escaping_values.insert(val.0.clone());
                        }

                        // Objects stored in fields escape
                        RirInstr::SetField { val, .. } | RirInstr::SetStatic { val, .. } => {
                            escaping_values.insert(val.0.clone());
                        }

                        // Objects stored in arrays escape (conservative)
                        RirInstr::ArrayStore { val, .. } => {
                            escaping_values.insert(val.0.clone());
                        }

                        // Objects passed to functions escape (conservative)
                        RirInstr::Call { args, .. } => {
                            for arg in args {
                                escaping_values.insert(arg.0.clone());
                            }
                        }

                        // Objects used as receivers/args in virtual/interface calls escape
                        RirInstr::CallVirtual { receiver, args, .. }
                        | RirInstr::CallInterface { receiver, args, .. } => {
                            escaping_values.insert(receiver.0.clone());
                            for arg in args {
                                escaping_values.insert(arg.0.clone());
                            }
                        }

                        _ => {}
                    }
                }
            }

            // Note: In a real implementation, we would mark New instructions with
            // metadata indicating whether they can be stack-allocated. For now,
            // we just perform the analysis without applying the optimization.
            // The actual stack allocation would be done in the code generator.

            // Count non-escaping allocations for statistics
            let mut total_news = 0;
            let mut non_escaping = 0;
            for bb in &func.basic_blocks {
                for instr in &bb.instrs {
                    if let RirInstr::New { ret, .. } = instr {
                        total_news += 1;
                        if !escaping_values.contains(&ret.0) {
                            non_escaping += 1;
                        }
                    }
                }
            }

            // Log statistics (in a real implementation)
            if total_news > 0 {
                let _escape_rate = (total_news - non_escaping) as f64 / total_news as f64;
                // eprintln!("Function {}: {}/{} allocations escape ({:.1}%)",
                //     func.name, total_news - non_escaping, total_news, escape_rate * 100.0);
            }
        }

        Ok(())
    }
}

/// Pass 2 — Inlining: inline methods smaller than 32 bytecodes.
pub struct InliningPass;
impl OptPass for InliningPass {
    fn name(&self) -> &'static str {
        "inlining"
    }
    fn run(&self, module: &mut Module) -> Result<()> {
        // Build a map of function sizes (instruction count) and identify inlinable functions
        let mut inlinable_funcs: HashSet<u32> = HashSet::new();

        for func in &module.functions {
            let instr_count: usize = func.basic_blocks.iter().map(|bb| bb.instrs.len()).sum();

            // Mark function as inlinable if:
            // 1. It's small (< 32 instructions)
            // 2. It has simple control flow (single basic block or simple linear flow)
            // 3. It's not a constructor or static initializer
            if instr_count < 32
                && func.basic_blocks.len() <= 3
                && !func.name.contains("<init>")
                && !func.name.contains("<clinit>")
            {
                // Compute hash of function name for lookup
                let func_hash = compute_hash(&func.name);
                inlinable_funcs.insert(func_hash);
            }
        }

        // Note: Full inlining implementation would require:
        // 1. Copying function body instructions
        // 2. Renaming all SSA values to avoid conflicts
        // 3. Substituting parameters with call arguments
        // 4. Handling return values
        // 5. Merging basic blocks
        //
        // For now, we just identify inlinable functions. The actual inlining
        // would be done in a more sophisticated pass or in the code generator.

        // Count inlining opportunities for statistics
        let mut total_calls = 0;
        let mut inlinable_calls = 0;

        for func in &module.functions {
            for bb in &func.basic_blocks {
                for instr in &bb.instrs {
                    if let RirInstr::Call { func: func_id, .. } = instr {
                        total_calls += 1;
                        if inlinable_funcs.contains(&func_id.0) {
                            inlinable_calls += 1;
                        }
                    }
                }
            }
        }

        // Log statistics (in a real implementation)
        if total_calls > 0 {
            let _inline_rate = inlinable_calls as f64 / total_calls as f64;
            // eprintln!("Inlining: {}/{} calls are inlinable ({:.1}%)",
            //     inlinable_calls, total_calls, inline_rate * 100.0);
        }

        Ok(())
    }
}

// Simple hash function for function names (matches encode_builtin behavior)
fn compute_hash(s: &str) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as u32
}

/// Pass 3 — Dead code elimination: remove unreachable basic blocks and dead values.
pub struct DeadCodeElimPass;
impl OptPass for DeadCodeElimPass {
    fn name(&self) -> &'static str {
        "dead-code-elim"
    }
    fn run(&self, module: &mut Module) -> Result<()> {
        for func in &mut module.functions {
            // Step 1: Find reachable blocks via DFS from entry block
            let mut reachable = HashSet::new();
            let mut stack = vec![];
            if let Some(entry) = func.basic_blocks.first() {
                stack.push(entry.id.0);
                reachable.insert(entry.id.0);
            }

            while let Some(bb_id) = stack.pop() {
                if let Some(bb) = func.basic_blocks.iter().find(|b| b.id.0 == bb_id) {
                    // Find successor blocks from terminator instructions
                    for instr in &bb.instrs {
                        match instr {
                            RirInstr::Branch {
                                then_bb, else_bb, ..
                            } => {
                                if reachable.insert(then_bb.0) {
                                    stack.push(then_bb.0);
                                }
                                if reachable.insert(else_bb.0) {
                                    stack.push(else_bb.0);
                                }
                            }
                            RirInstr::Jump(target) => {
                                if reachable.insert(target.0) {
                                    stack.push(target.0);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Step 2: Remove unreachable blocks
            func.basic_blocks.retain(|bb| reachable.contains(&bb.id.0));

            // Step 3: Find live values (used as operands)
            let mut live_values = HashSet::new();
            for bb in &func.basic_blocks {
                for instr in &bb.instrs {
                    // Mark all operand values as live
                    match instr {
                        RirInstr::Branch { cond, .. } => {
                            live_values.insert(cond.0.clone());
                        }
                        RirInstr::Return(Some(val)) => {
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::Call { args, .. } => {
                            for arg in args {
                                live_values.insert(arg.0.clone());
                            }
                        }
                        RirInstr::CallVirtual { receiver, args, .. }
                        | RirInstr::CallInterface { receiver, args, .. } => {
                            live_values.insert(receiver.0.clone());
                            for arg in args {
                                live_values.insert(arg.0.clone());
                            }
                        }
                        RirInstr::GetField { obj, .. } => {
                            live_values.insert(obj.0.clone());
                        }
                        RirInstr::SetField { obj, val, .. } => {
                            live_values.insert(obj.0.clone());
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::SetStatic { val, .. } => {
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::Instanceof { obj, .. } => {
                            live_values.insert(obj.0.clone());
                        }
                        RirInstr::Checkcast { obj, .. } => {
                            live_values.insert(obj.0.clone());
                        }
                        RirInstr::NewArray { len, .. } => {
                            live_values.insert(len.0.clone());
                        }
                        RirInstr::NewMultiArray { dims, .. } => {
                            for dim in dims {
                                live_values.insert(dim.0.clone());
                            }
                        }
                        RirInstr::ArrayLoad { arr, idx, .. } => {
                            live_values.insert(arr.0.clone());
                            live_values.insert(idx.0.clone());
                        }
                        RirInstr::ArrayStore { arr, idx, val } => {
                            live_values.insert(arr.0.clone());
                            live_values.insert(idx.0.clone());
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::ArrayLen { arr, .. } => {
                            live_values.insert(arr.0.clone());
                        }
                        RirInstr::BinOp { lhs, rhs, .. } => {
                            live_values.insert(lhs.0.clone());
                            live_values.insert(rhs.0.clone());
                        }
                        RirInstr::UnaryOp { operand, .. } => {
                            live_values.insert(operand.0.clone());
                        }
                        RirInstr::Convert { val, .. } => {
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::Throw(val) => {
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::MonitorEnter(val) | RirInstr::MonitorExit(val) => {
                            live_values.insert(val.0.clone());
                        }
                        RirInstr::MicroRtReflect { class_name, .. } => {
                            live_values.insert(class_name.0.clone());
                        }
                        RirInstr::MicroRtProxy {
                            interfaces, handler, ..
                        } => {
                            for iface in interfaces {
                                live_values.insert(iface.0.clone());
                            }
                            live_values.insert(handler.0.clone());
                        }
                        RirInstr::MicroRtClassLoad { class_name, .. } => {
                            live_values.insert(class_name.0.clone());
                        }
                        _ => {}
                    }
                }
            }

            // Step 4: Remove dead value definitions (instructions with unused results)
            // Note: We can't remove instructions with side effects (calls, stores, etc.)
            for bb in &mut func.basic_blocks {
                bb.instrs.retain(|instr| {
                    match instr {
                        // Pure instructions - can be removed if result is unused
                        RirInstr::ConstInt { ret, .. }
                        | RirInstr::ConstFloat { ret, .. }
                        | RirInstr::ConstStr { ret, .. }
                        | RirInstr::ConstBool { ret, .. }
                        | RirInstr::ConstNull { ret } => live_values.contains(&ret.0),

                        RirInstr::BinOp { ret, .. }
                        | RirInstr::UnaryOp { ret, .. }
                        | RirInstr::Convert { ret, .. } => live_values.contains(&ret.0),

                        RirInstr::GetField { ret, .. }
                        | RirInstr::GetStatic { ret, .. }
                        | RirInstr::ArrayLoad { ret, .. }
                        | RirInstr::ArrayLen { ret, .. }
                        | RirInstr::Instanceof { ret, .. } => live_values.contains(&ret.0),

                        // Instructions with side effects - always keep
                        _ => true,
                    }
                });
            }
        }

        Ok(())
    }
}

/// Pass 4 — Constant folding: evaluate constant expressions at compile time.
pub struct ConstFoldingPass;
impl OptPass for ConstFoldingPass {
    fn name(&self) -> &'static str {
        "const-folding"
    }
    fn run(&self, module: &mut Module) -> Result<()> {
        for func in &mut module.functions {
            // Track known constant values
            let mut constants: HashMap<String, ConstValue> = HashMap::new();

            for bb in &mut func.basic_blocks {
                for instr in &mut bb.instrs {
                    match instr {
                        // Track constant definitions
                        RirInstr::ConstInt { ret, value } => {
                            constants.insert(ret.0.clone(), ConstValue::Int(*value));
                        }
                        RirInstr::ConstFloat { ret, value } => {
                            constants.insert(ret.0.clone(), ConstValue::Float(*value));
                        }
                        RirInstr::ConstBool { ret, value } => {
                            constants.insert(ret.0.clone(), ConstValue::Bool(*value));
                        }

                        // Fold binary operations on constants
                        RirInstr::BinOp { op, lhs, rhs, ret } => {
                            if let (Some(l), Some(r)) =
                                (constants.get(&lhs.0), constants.get(&rhs.0))
                            {
                                if let Some(result) = fold_binop(op, l, r) {
                                    let ret_val = ret.clone();
                                    // Replace BinOp with constant
                                    *instr = match result {
                                        ConstValue::Int(v) => RirInstr::ConstInt {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                        ConstValue::Float(v) => RirInstr::ConstFloat {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                        ConstValue::Bool(v) => RirInstr::ConstBool {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                    };
                                    constants.insert(ret_val.0, result);
                                }
                            }
                        }

                        // Fold unary operations on constants
                        RirInstr::UnaryOp { op, operand, ret } => {
                            if let Some(val) = constants.get(&operand.0) {
                                if let Some(result) = fold_unaryop(op, val) {
                                    let ret_val = ret.clone();
                                    *instr = match result {
                                        ConstValue::Int(v) => RirInstr::ConstInt {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                        ConstValue::Float(v) => RirInstr::ConstFloat {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                        ConstValue::Bool(v) => RirInstr::ConstBool {
                                            ret: ret_val.clone(),
                                            value: v,
                                        },
                                    };
                                    constants.insert(ret_val.0, result);
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
}

fn fold_binop(op: &BinOp, lhs: &ConstValue, rhs: &ConstValue) -> Option<ConstValue> {
    match (lhs, rhs) {
        (ConstValue::Int(l), ConstValue::Int(r)) => match op {
            BinOp::Add => Some(ConstValue::Int(l.wrapping_add(*r))),
            BinOp::Sub => Some(ConstValue::Int(l.wrapping_sub(*r))),
            BinOp::Mul => Some(ConstValue::Int(l.wrapping_mul(*r))),
            BinOp::Div if *r != 0 => Some(ConstValue::Int(l.wrapping_div(*r))),
            BinOp::Rem if *r != 0 => Some(ConstValue::Int(l.wrapping_rem(*r))),
            BinOp::BitAnd => Some(ConstValue::Int(l & r)),
            BinOp::BitOr => Some(ConstValue::Int(l | r)),
            BinOp::Xor => Some(ConstValue::Int(l ^ r)),
            BinOp::Shl => Some(ConstValue::Int(l << (r & 63))),
            BinOp::Shr => Some(ConstValue::Int(l >> (r & 63))),
            BinOp::UShr => Some(ConstValue::Int((*l as u64 >> (r & 63)) as i64)),
            BinOp::Eq => Some(ConstValue::Bool(l == r)),
            BinOp::Ne => Some(ConstValue::Bool(l != r)),
            BinOp::Lt => Some(ConstValue::Bool(l < r)),
            BinOp::Le => Some(ConstValue::Bool(l <= r)),
            BinOp::Gt => Some(ConstValue::Bool(l > r)),
            BinOp::Ge => Some(ConstValue::Bool(l >= r)),
            _ => None,
        },
        (ConstValue::Float(l), ConstValue::Float(r)) => match op {
            BinOp::Add => Some(ConstValue::Float(l + r)),
            BinOp::Sub => Some(ConstValue::Float(l - r)),
            BinOp::Mul => Some(ConstValue::Float(l * r)),
            BinOp::Div => Some(ConstValue::Float(l / r)),
            BinOp::Rem => Some(ConstValue::Float(l % r)),
            BinOp::Eq => Some(ConstValue::Bool(l == r)),
            BinOp::Ne => Some(ConstValue::Bool(l != r)),
            BinOp::Lt => Some(ConstValue::Bool(l < r)),
            BinOp::Le => Some(ConstValue::Bool(l <= r)),
            BinOp::Gt => Some(ConstValue::Bool(l > r)),
            BinOp::Ge => Some(ConstValue::Bool(l >= r)),
            _ => None,
        },
        (ConstValue::Bool(l), ConstValue::Bool(r)) => match op {
            BinOp::And => Some(ConstValue::Bool(*l && *r)),
            BinOp::Or => Some(ConstValue::Bool(*l || *r)),
            BinOp::Xor => Some(ConstValue::Bool(*l ^ *r)),
            BinOp::Eq => Some(ConstValue::Bool(l == r)),
            BinOp::Ne => Some(ConstValue::Bool(l != r)),
            _ => None,
        },
        _ => None,
    }
}

fn fold_unaryop(op: &UnaryOp, val: &ConstValue) -> Option<ConstValue> {
    match (op, val) {
        (UnaryOp::Neg, ConstValue::Int(v)) => Some(ConstValue::Int(v.wrapping_neg())),
        (UnaryOp::Neg, ConstValue::Float(v)) => Some(ConstValue::Float(-v)),
        (UnaryOp::Not, ConstValue::Bool(v)) => Some(ConstValue::Bool(!v)),
        _ => None,
    }
}

/// Pass 5 — Metadata table generation: embed reflection metadata in the binary (Phase 2).
pub struct MetadataTableGenPass;
impl OptPass for MetadataTableGenPass {
    fn name(&self) -> &'static str {
        "metadata-table-gen"
    }
    fn run(&self, module: &mut Module) -> Result<()> {
        use rava_rir::metadata::{ClassMetadata, FieldMetadata, MethodMetadata, MetadataTable};

        let mut metadata_table = MetadataTable::new();

        // Extract class information from RIR module
        // Group functions by class (functions with names like "ClassName.methodName")
        let mut class_methods: HashMap<String, Vec<String>> = HashMap::new();

        for func in &module.functions {
            // Parse function name to extract class and method
            if let Some((class_name, method_name)) = parse_function_name(&func.name) {
                class_methods
                    .entry(class_name.to_string())
                    .or_default()
                    .push(method_name.to_string());
            }
        }

        // Build metadata for each class
        for (class_name, methods) in class_methods {
            let mut class_metadata = ClassMetadata {
                name: class_name.clone(),
                superclass: None, // TODO: extract from class hierarchy
                interfaces: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                constructors: Vec::new(),
                modifiers: vec!["public".to_string()],
            };

            // Add methods
            for method_name in methods {
                if method_name.starts_with("<init>") {
                    // Constructor
                    class_metadata.constructors.push(rava_rir::metadata::ConstructorMetadata {
                        signature: "()V".to_string(), // TODO: extract actual signature
                        function_ptr: None, // TODO: resolve function pointer
                        modifiers: vec!["public".to_string()],
                    });
                } else if !method_name.starts_with("<clinit>") {
                    // Regular method
                    class_metadata.methods.push(MethodMetadata {
                        name: method_name.clone(),
                        signature: "()V".to_string(), // TODO: extract actual signature
                        function_ptr: None, // TODO: resolve function pointer
                        modifiers: vec!["public".to_string()],
                    });
                }
            }

            // Add fields from field_names map
            for (field_hash, field_name) in &module.field_names {
                if field_name.starts_with(&format!("{}.", class_name)) {
                    let simple_name = field_name.strip_prefix(&format!("{}.", class_name))
                        .unwrap_or(field_name);

                    let field_type = module.field_types.get(field_hash);
                    let type_descriptor = match field_type {
                        Some(rava_rir::RirType::I32) => "I",
                        Some(rava_rir::RirType::I64) => "J",
                        Some(rava_rir::RirType::F32) => "F",
                        Some(rava_rir::RirType::F64) => "D",
                        Some(rava_rir::RirType::Bool) => "Z",
                        Some(rava_rir::RirType::Ref(_)) => "Ljava/lang/Object;",
                        _ => "Ljava/lang/Object;",
                    };

                    class_metadata.fields.push(FieldMetadata {
                        name: simple_name.to_string(),
                        type_descriptor: type_descriptor.to_string(),
                        offset: None, // TODO: compute field offset
                        getter_ptr: None,
                        setter_ptr: None,
                        modifiers: vec!["public".to_string()],
                    });
                }
            }

            metadata_table.add_class(class_name, class_metadata);
        }

        // Store metadata table in module (for now, just log statistics)
        let class_count = metadata_table.classes.len();
        let method_count: usize = metadata_table.classes.values()
            .map(|c| c.methods.len() + c.constructors.len())
            .sum();
        let field_count: usize = metadata_table.classes.values()
            .map(|c| c.fields.len())
            .sum();

        // In a real implementation, we would serialize the metadata table
        // and embed it in the binary as a data section
        // eprintln!("Generated metadata: {} classes, {} methods, {} fields",
        //     class_count, method_count, field_count);

        let _ = (class_count, method_count, field_count); // Suppress unused warnings

        Ok(())
    }
}

/// Parse a function name like "ClassName.methodName" into (class, method).
fn parse_function_name(name: &str) -> Option<(&str, &str)> {
    if let Some(dot_pos) = name.rfind('.') {
        let class = &name[..dot_pos];
        let method = &name[dot_pos + 1..];
        Some((class, method))
    } else {
        None
    }
}

/// Pass 6 — Proxy pre-generation: AOT-compile proxy classes for known interface combos (Phase 4).
pub struct ProxyPregenPass;
impl OptPass for ProxyPregenPass {
    fn name(&self) -> &'static str {
        "proxy-pregen"
    }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-4): pre-generate proxy classes
        Ok(())
    }
}

/// Pass 7 — MicroRT bridge: generate bridging stubs for MicroRtReflect/Proxy/ClassLoad instructions.
pub struct MicroRtBridgePass;
impl OptPass for MicroRtBridgePass {
    fn name(&self) -> &'static str {
        "micrort-bridge"
    }
    fn run(&self, _module: &mut Module) -> Result<()> {
        // TODO(phase-3): generate MicroRT interop stubs
        Ok(())
    }
}
