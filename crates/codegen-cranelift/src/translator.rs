//! RIR → Cranelift IR translator.
//!
//! Translates each RirFunction into a Cranelift function, mapping:
//!   - RIR BasicBlocks → CLIF Blocks
//!   - RIR Values (SSA names) → CLIF Values
//!   - RIR instructions → CLIF instructions

#![allow(dead_code)]

use std::collections::HashMap;
use cranelift_codegen::ir::{
    self, types, AbiParam, InstBuilder, MemFlags, Signature,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::entity::EntityRef;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId as ClifFuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use rava_common::error::{RavaError, Result};
use rava_rir::{BinOp, RirFunction, RirInstr, RirModule, RirType, UnaryOp};

/// Translate an entire RirModule into Cranelift IR and define functions in the ObjectModule.
pub fn translate_module(module: &RirModule, obj: &mut ObjectModule) -> Result<()> {
    let mut ctx = TranslationCtx::new(module, obj)?;
    ctx.translate()
}

struct TranslationCtx<'a> {
    rir: &'a RirModule,
    obj: &'a mut ObjectModule,
    /// Maps RIR function name → CLIF FuncId
    func_ids: HashMap<String, ClifFuncId>,
    /// Runtime function references
    rt_println_int: ClifFuncId,
    rt_println_float: ClifFuncId,
    rt_println_str: ClifFuncId,
    rt_println_bool: ClifFuncId,
    rt_println_void: ClifFuncId,
    rt_print_int: ClifFuncId,
    rt_print_str: ClifFuncId,
    rt_str_concat: ClifFuncId,
    rt_int_to_str: ClifFuncId,
    rt_float_to_str: ClifFuncId,
    /// String constant data IDs
    str_constants: HashMap<String, cranelift_module::DataId>,
}

impl<'a> TranslationCtx<'a> {
    fn new(rir: &'a RirModule, obj: &'a mut ObjectModule) -> Result<Self> {
        // Declare runtime functions
        let rt_println_int = Self::declare_rt_func(obj, "rava_println_int", &[types::I64], None)?;
        let rt_println_float = Self::declare_rt_func(obj, "rava_println_float", &[types::F64], None)?;
        let rt_println_str = Self::declare_rt_func(obj, "rava_println_str", &[types::I64], None)?;
        let rt_println_bool = Self::declare_rt_func(obj, "rava_println_bool", &[types::I64], None)?;
        let rt_println_void = Self::declare_rt_func(obj, "rava_println_void", &[], None)?;
        let rt_print_int = Self::declare_rt_func(obj, "rava_print_int", &[types::I64], None)?;
        let rt_print_str = Self::declare_rt_func(obj, "rava_print_str", &[types::I64], None)?;
        let rt_str_concat = Self::declare_rt_func(obj, "rava_str_concat", &[types::I64, types::I64], Some(types::I64))?;
        let rt_int_to_str = Self::declare_rt_func(obj, "rava_int_to_str", &[types::I64], Some(types::I64))?;
        let rt_float_to_str = Self::declare_rt_func(obj, "rava_float_to_str", &[types::F64], Some(types::I64))?;

        // Declare all RIR functions first (forward declaration)
        let mut func_ids = HashMap::new();
        for func in &rir.functions {
            let sig = build_signature(func);
            let mangled = mangle_name(&func.name);
            let id = obj.declare_function(&mangled, Linkage::Export, &sig)
                .map_err(|e| RavaError::Codegen(format!("declare {} failed: {e}", func.name)))?;
            func_ids.insert(func.name.clone(), id);
        }

        Ok(Self {
            rir, obj, func_ids,
            rt_println_int, rt_println_float, rt_println_str,
            rt_println_bool, rt_println_void,
            rt_print_int, rt_print_str,
            rt_str_concat, rt_int_to_str, rt_float_to_str,
            str_constants: HashMap::new(),
        })
    }

    fn declare_rt_func(
        obj: &mut ObjectModule, name: &str,
        params: &[ir::Type], ret: Option<ir::Type>,
    ) -> Result<ClifFuncId> {
        let mut sig = Signature::new(CallConv::SystemV);
        for &p in params { sig.params.push(AbiParam::new(p)); }
        if let Some(r) = ret { sig.returns.push(AbiParam::new(r)); }
        obj.declare_function(name, Linkage::Import, &sig)
            .map_err(|e| RavaError::Codegen(format!("declare {name} failed: {e}")))
    }

    fn translate(&mut self) -> Result<()> {
        for func in &self.rir.functions {
            self.translate_function(func)?;
        }
        Ok(())
    }

    fn translate_function(&mut self, func: &RirFunction) -> Result<()> {
        let sig = build_signature(func);
        let clif_func_id = self.func_ids[&func.name];

        let mut clif_func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, clif_func_id.as_u32()),
            sig.clone(),
        );

        let mut fb_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut clif_func, &mut fb_ctx);

        // Create CLIF blocks for each RIR basic block
        let mut block_map: HashMap<u32, ir::Block> = HashMap::new();
        for bb in &func.basic_blocks {
            let clif_block = builder.create_block();
            block_map.insert(bb.id.0, clif_block);
        }

        // Variable counter for SSA values
        let mut var_map: HashMap<String, Variable> = HashMap::new();
        let mut var_counter = 0u32;

        // Helper to get or create a variable
        let get_var = |name: &str, var_map: &mut HashMap<String, Variable>, var_counter: &mut u32| -> Variable {
            if let Some(&v) = var_map.get(name) {
                return v;
            }
            let v = Variable::new(*var_counter as usize);
            *var_counter += 1;
            var_map.insert(name.to_string(), v);
            v
        };

        // Declare all variables as I64 (we use I64 as the universal type for simplicity)
        // We'll collect all value names first, deduplicating
        let mut all_names: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for p in &func.params {
            if seen.insert(p.0 .0.clone()) {
                all_names.push(p.0 .0.clone());
            }
        }
        for bb in &func.basic_blocks {
            for instr in &bb.instrs {
                let mut new_names = Vec::new();
                collect_def_names(instr, &mut new_names);
                for n in new_names {
                    if seen.insert(n.clone()) {
                        all_names.push(n);
                    }
                }
            }
        }
        for name in &all_names {
            let v = get_var(name, &mut var_map, &mut var_counter);
            builder.declare_var(v, types::I64);
        }

        // Entry block: set up parameters
        let entry_block = block_map[&func.basic_blocks[0].id.0];
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        for (i, (param_name, _)) in func.params.iter().enumerate() {
            let param_val = builder.block_params(entry_block)[i];
            let var = var_map[&param_name.0];
            builder.def_var(var, param_val);
        }

        // Translate each basic block
        for (bb_idx, bb) in func.basic_blocks.iter().enumerate() {
            let clif_block = block_map[&bb.id.0];

            if bb_idx > 0 {
                builder.switch_to_block(clif_block);
            }

            for instr in &bb.instrs {
                self.translate_instr(
                    instr, &mut builder, &block_map, &mut var_map, &mut var_counter,
                )?;
            }

            // If block doesn't end with a terminator, add a return
            if !block_ends_with_terminator(&bb.instrs) {
                builder.ins().return_(&[]);
            }
        }

        // Seal all blocks after all instructions have been emitted
        builder.seal_all_blocks();

        builder.finalize();

        // Define the function in the object module
        let mut ctx = cranelift_codegen::Context::for_function(clif_func);
        self.obj.define_function(clif_func_id, &mut ctx)
            .map_err(|e| RavaError::Codegen(format!("define {} failed: {e}", func.name)))?;

        Ok(())
    }

    fn translate_instr(
        &mut self,
        instr: &RirInstr,
        builder: &mut FunctionBuilder,
        block_map: &HashMap<u32, ir::Block>,
        var_map: &mut HashMap<String, Variable>,
        var_counter: &mut u32,
    ) -> Result<()> {
        match instr {
            RirInstr::ConstInt { ret, value } => {
                let v = builder.ins().iconst(types::I64, *value);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::ConstFloat { ret, value } => {
                let v = builder.ins().f64const(*value);
                // Store as bits in I64 for uniform variable type
                let bits = builder.ins().bitcast(types::I64, MemFlags::new(), v);
                self.def_val(builder, var_map, var_counter, &ret.0, bits);
            }
            RirInstr::ConstStr { ret, value } => {
                // Handle __copy__ markers
                if let Some(src) = value.strip_prefix("__copy__") {
                    let src_val = self.use_val(builder, var_map, var_counter, src);
                    self.def_val(builder, var_map, var_counter, &ret.0, src_val);
                } else {
                    let ptr = self.get_string_ptr(builder, value)?;
                    self.def_val(builder, var_map, var_counter, &ret.0, ptr);
                }
            }
            RirInstr::ConstNull { ret } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::BinOp { op, lhs, rhs, ret } => {
                let l = self.use_val(builder, var_map, var_counter, &lhs.0);
                let r = self.use_val(builder, var_map, var_counter, &rhs.0);
                let result = self.translate_binop(builder, op, l, r);
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::UnaryOp { op, operand, ret } => {
                let v = self.use_val(builder, var_map, var_counter, &operand.0);
                let result = match op {
                    UnaryOp::Neg => builder.ins().ineg(v),
                    UnaryOp::Not => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let cmp = builder.ins().icmp(IntCC::Equal, v, zero);
                        builder.ins().uextend(types::I64, cmp)
                    }
                };
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::Return(val) => {
                match val {
                    Some(v) => {
                        let rv = self.use_val(builder, var_map, var_counter, &v.0);
                        builder.ins().return_(&[rv]);
                    }
                    None => { builder.ins().return_(&[]); }
                }
            }
            RirInstr::Jump(target) => {
                let blk = block_map[&target.0];
                builder.ins().jump(blk, &[]);
            }
            RirInstr::Branch { cond, then_bb, else_bb } => {
                let cv = self.use_val(builder, var_map, var_counter, &cond.0);
                let then_blk = block_map[&then_bb.0];
                let else_blk = block_map[&else_bb.0];
                builder.ins().brif(cv, then_blk, &[], else_blk, &[]);
            }
            RirInstr::Call { func: func_id, args, ret } => {
                let arg_vals: Vec<ir::Value> = args.iter()
                    .map(|a| self.use_val(builder, var_map, var_counter, &a.0))
                    .collect();
                self.translate_call(builder, func_id.0, &arg_vals, ret.as_ref(), var_map, var_counter)?;
            }
            RirInstr::Convert { val, to, ret, .. } => {
                let v = self.use_val(builder, var_map, var_counter, &val.0);
                // For Phase 1, most conversions are identity (everything is I64)
                let result = match to {
                    RirType::F32 | RirType::F64 => v, // keep as bits
                    _ => v,
                };
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            // Instructions that are no-ops or stubs in AOT Phase 1
            RirInstr::New { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::GetField { ret, .. } | RirInstr::GetStatic { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::SetField { .. } | RirInstr::SetStatic { .. } => {}
            RirInstr::NewArray { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::ArrayLoad { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::ArrayStore { .. } | RirInstr::ArrayLen { .. } => {
                // ArrayLen needs a ret
            }
            RirInstr::Instanceof { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::Checkcast { .. } => {}
            RirInstr::Throw(_) => {
                builder.ins().trap(ir::TrapCode::unwrap_user(0));
            }
            RirInstr::Unreachable => {
                builder.ins().trap(ir::TrapCode::unwrap_user(1));
            }
            RirInstr::MonitorEnter(_) | RirInstr::MonitorExit(_) => {}
            RirInstr::CallVirtual { ret, .. } | RirInstr::CallInterface { ret, .. } => {
                if let Some(r) = ret {
                    let v = builder.ins().iconst(types::I64, 0);
                    self.def_val(builder, var_map, var_counter, &r.0, v);
                }
            }
            RirInstr::MicroRtReflect { ret, .. } |
            RirInstr::MicroRtProxy { ret, .. } |
            RirInstr::MicroRtClassLoad { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
        }
        Ok(())
    }

    fn translate_binop(&self, builder: &mut FunctionBuilder, op: &BinOp, l: ir::Value, r: ir::Value) -> ir::Value {
        match op {
            BinOp::Add => builder.ins().iadd(l, r),
            BinOp::Sub => builder.ins().isub(l, r),
            BinOp::Mul => builder.ins().imul(l, r),
            BinOp::Div => builder.ins().sdiv(l, r),
            BinOp::Rem => builder.ins().srem(l, r),
            BinOp::And | BinOp::BitAnd => builder.ins().band(l, r),
            BinOp::Or | BinOp::BitOr => builder.ins().bor(l, r),
            BinOp::Xor => builder.ins().bxor(l, r),
            BinOp::Shl => builder.ins().ishl(l, r),
            BinOp::Shr => builder.ins().sshr(l, r),
            BinOp::UShr => builder.ins().ushr(l, r),
            BinOp::Eq => {
                let cmp = builder.ins().icmp(IntCC::Equal, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
            BinOp::Ne => {
                let cmp = builder.ins().icmp(IntCC::NotEqual, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
            BinOp::Lt => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThan, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
            BinOp::Le => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
            BinOp::Gt => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
            BinOp::Ge => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                builder.ins().uextend(types::I64, cmp)
            }
        }
    }

    fn translate_call(
        &mut self,
        builder: &mut FunctionBuilder,
        func_id: u32,
        args: &[ir::Value],
        ret: Option<&rava_rir::Value>,
        var_map: &mut HashMap<String, Variable>,
        var_counter: &mut u32,
    ) -> Result<()> {
        use rava_frontend::lowerer::encode_builtin;

        // System.out.println dispatch
        if func_id == encode_builtin("System.out.println") {
            let func_ref = self.obj.declare_func_in_func(self.rt_println_int, builder.func);
            if args.is_empty() {
                let void_ref = self.obj.declare_func_in_func(self.rt_println_void, builder.func);
                builder.ins().call(void_ref, &[]);
            } else {
                builder.ins().call(func_ref, &[args[0]]);
            }
            if let Some(r) = ret {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &r.0, v);
            }
            return Ok(());
        }
        if func_id == encode_builtin("System.out.print") {
            if !args.is_empty() {
                let func_ref = self.obj.declare_func_in_func(self.rt_print_int, builder.func);
                builder.ins().call(func_ref, &[args[0]]);
            }
            if let Some(r) = ret {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &r.0, v);
            }
            return Ok(());
        }

        // User-defined function call
        for (name, &clif_id) in &self.func_ids {
            let name_match = encode_builtin(name) == func_id
                || name.rsplit('.').next()
                    .map(|s| encode_builtin(s) == func_id)
                    .unwrap_or(false);
            if name_match {
                let func_ref = self.obj.declare_func_in_func(clif_id, builder.func);
                let sig = builder.func.dfg.ext_funcs[func_ref].signature;
                let expected_params = builder.func.dfg.signatures[sig].params.len();
                let call_args = if args.len() > expected_params {
                    &args[..expected_params]
                } else {
                    args
                };
                let inst = builder.ins().call(func_ref, call_args);
                if let Some(r) = ret {
                    let results = builder.inst_results(inst);
                    if !results.is_empty() {
                        self.def_val(builder, var_map, var_counter, &r.0, results[0]);
                    } else {
                        let v = builder.ins().iconst(types::I64, 0);
                        self.def_val(builder, var_map, var_counter, &r.0, v);
                    }
                }
                return Ok(());
            }
        }

        // Unknown call — return 0
        if let Some(r) = ret {
            let v = builder.ins().iconst(types::I64, 0);
            self.def_val(builder, var_map, var_counter, &r.0, v);
        }
        Ok(())
    }

    fn def_val(
        &self,
        builder: &mut FunctionBuilder,
        var_map: &mut HashMap<String, Variable>,
        var_counter: &mut u32,
        name: &str,
        val: ir::Value,
    ) {
        let var = if let Some(&v) = var_map.get(name) {
            v
        } else {
            let v = Variable::new(*var_counter as usize);
            *var_counter += 1;
            var_map.insert(name.to_string(), v);
            builder.declare_var(v, types::I64);
            v
        };
        builder.def_var(var, val);
    }

    fn use_val(
        &self,
        builder: &mut FunctionBuilder,
        var_map: &mut HashMap<String, Variable>,
        var_counter: &mut u32,
        name: &str,
    ) -> ir::Value {
        let var = if let Some(&v) = var_map.get(name) {
            v
        } else {
            let v = Variable::new(*var_counter as usize);
            *var_counter += 1;
            var_map.insert(name.to_string(), v);
            builder.declare_var(v, types::I64);
            // Initialize to 0
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(v, zero);
            v
        };
        builder.use_var(var)
    }

    fn get_string_ptr(&mut self, builder: &mut FunctionBuilder, s: &str) -> Result<ir::Value> {
        // For now, string constants are stored as their hash (pointer to data section)
        // A real implementation would store the string in a data section
        let hash = rava_frontend::lowerer::encode_builtin(s) as i64;
        Ok(builder.ins().iconst(types::I64, hash))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_signature(func: &RirFunction) -> Signature {
    let mut sig = Signature::new(CallConv::SystemV);
    for (_, ty) in &func.params {
        sig.params.push(AbiParam::new(rir_type_to_clif(ty)));
    }
    match &func.return_type {
        RirType::Void => {}
        ty => { sig.returns.push(AbiParam::new(rir_type_to_clif(ty))); }
    }
    sig
}

fn rir_type_to_clif(ty: &RirType) -> ir::Type {
    match ty {
        RirType::I8 | RirType::I16 | RirType::I32 | RirType::Bool => types::I64,
        RirType::I64 => types::I64,
        RirType::F32 => types::I64, // stored as bits
        RirType::F64 => types::I64, // stored as bits
        RirType::Ref(_) | RirType::Array(_) | RirType::RawPtr => types::I64,
        RirType::Void => types::I64, // shouldn't happen for params
    }
}

fn block_ends_with_terminator(instrs: &[RirInstr]) -> bool {
    matches!(instrs.last(),
        Some(RirInstr::Return(_) | RirInstr::Jump(_) | RirInstr::Branch { .. } |
             RirInstr::Unreachable | RirInstr::Throw(_)))
}

/// Collect all value names defined by an instruction.
fn collect_def_names(instr: &RirInstr, names: &mut Vec<String>) {
    match instr {
        RirInstr::ConstInt { ret, .. } |
        RirInstr::ConstFloat { ret, .. } |
        RirInstr::ConstStr { ret, .. } |
        RirInstr::ConstNull { ret } => { names.push(ret.0.clone()); }
        RirInstr::BinOp { ret, .. } |
        RirInstr::UnaryOp { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::Call { ret: Some(ret), .. } => { names.push(ret.0.clone()); }
        RirInstr::New { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::GetField { ret, .. } |
        RirInstr::GetStatic { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::NewArray { ret, .. } |
        RirInstr::ArrayLoad { ret, .. } |
        RirInstr::ArrayLen { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::Instanceof { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::Convert { ret, .. } => { names.push(ret.0.clone()); }
        RirInstr::CallVirtual { ret: Some(ret), .. } |
        RirInstr::CallInterface { ret: Some(ret), .. } => { names.push(ret.0.clone()); }
        RirInstr::MicroRtReflect { ret, .. } |
        RirInstr::MicroRtProxy { ret, .. } |
        RirInstr::MicroRtClassLoad { ret, .. } => { names.push(ret.0.clone()); }
        _ => {}
    }
}

/// Mangle a Java-style name (e.g. `Main.main`) to a C-compatible symbol (`Main_main`).
fn mangle_name(name: &str) -> String {
    name.replace('.', "_").replace('<', "_").replace('>', "_")
}
