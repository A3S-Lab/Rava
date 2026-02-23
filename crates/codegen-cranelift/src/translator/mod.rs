//! RIR → Cranelift IR translator.
//!
//! Translates each RirFunction into a Cranelift function, mapping:
//!   - RIR BasicBlocks → CLIF Blocks
//!   - RIR Values (SSA names) → CLIF Values
//!   - RIR instructions → CLIF instructions

#![allow(dead_code)]

mod helpers;

use std::collections::HashMap;
use cranelift_codegen::ir::{
    self, types, AbiParam, InstBuilder, MemFlags, Signature,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::condcodes::FloatCC;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::entity::EntityRef;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId as ClifFuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use rava_common::error::{RavaError, Result};
use rava_rir::{BinOp, RirFunction, RirInstr, RirModule, RirType, UnaryOp};

use helpers::{build_signature, block_ends_with_terminator, collect_def_names, mangle_name};

/// Tracked value type for AOT dispatch (println, concat, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValType {
    Int,
    Float,
    Bool,
    Str,
    Obj,
}

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
    // Object runtime
    rt_obj_alloc: ClifFuncId,
    rt_obj_get_field: ClifFuncId,
    rt_obj_set_field: ClifFuncId,
    // Array runtime
    rt_arr_alloc: ClifFuncId,
    rt_arr_load: ClifFuncId,
    rt_arr_store: ClifFuncId,
    rt_arr_len: ClifFuncId,
    // Class tag runtime (instanceof)
    rt_obj_set_tag: ClifFuncId,
    rt_obj_get_tag: ClifFuncId,
    /// String constant data IDs
    str_constants: HashMap<String, cranelift_module::DataId>,
    /// Maps FieldId hash → sequential slot index (per class, but flattened for Phase 1)
    field_slots: HashMap<u32, u32>,
    /// Maps ClassId hash → unique class tag integer
    class_tags: HashMap<u32, i64>,
}

impl<'a> TranslationCtx<'a> {
    fn new(rir: &'a RirModule, obj: &'a mut ObjectModule) -> Result<Self> {
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
        let rt_obj_alloc = Self::declare_rt_func(obj, "rava_obj_alloc", &[types::I64], Some(types::I64))?;
        let rt_obj_get_field = Self::declare_rt_func(obj, "rava_obj_get_field", &[types::I64, types::I64], Some(types::I64))?;
        let rt_obj_set_field = Self::declare_rt_func(obj, "rava_obj_set_field", &[types::I64, types::I64, types::I64], None)?;
        let rt_arr_alloc = Self::declare_rt_func(obj, "rava_arr_alloc", &[types::I64], Some(types::I64))?;
        let rt_arr_load = Self::declare_rt_func(obj, "rava_arr_load", &[types::I64, types::I64], Some(types::I64))?;
        let rt_arr_store = Self::declare_rt_func(obj, "rava_arr_store", &[types::I64, types::I64, types::I64], None)?;
        let rt_arr_len = Self::declare_rt_func(obj, "rava_arr_len", &[types::I64], Some(types::I64))?;
        let rt_obj_set_tag = Self::declare_rt_func(obj, "rava_obj_set_tag", &[types::I64, types::I64], None)?;
        let rt_obj_get_tag = Self::declare_rt_func(obj, "rava_obj_get_tag", &[types::I64], Some(types::I64))?;

        let mut field_slots = HashMap::new();
        let mut slot = 0u32;
        for &hash in rir.field_names.keys() {
            field_slots.insert(hash, slot);
            slot += 1;
        }

        let mut class_tags = HashMap::new();
        let mut tag = 1i64;
        for func in &rir.functions {
            if let Some(class_name) = func.name.split('.').next() {
                let hash = rava_frontend::lowerer::encode_builtin(class_name);
                if !class_tags.contains_key(&hash) {
                    class_tags.insert(hash, tag);
                    tag += 1;
                }
            }
        }

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
            rt_obj_alloc, rt_obj_get_field, rt_obj_set_field,
            rt_arr_alloc, rt_arr_load, rt_arr_store, rt_arr_len,
            rt_obj_set_tag, rt_obj_get_tag,
            str_constants: HashMap::new(),
            field_slots,
            class_tags,
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
        self.emit_entry_trampoline()?;
        Ok(())
    }

    fn emit_entry_trampoline(&mut self) -> Result<()> {
        let main_func = self.rir.functions.iter().find(|f| {
            f.name.ends_with(".main") && !f.flags.is_constructor && !f.flags.is_clinit
        });
        let main_name = match main_func {
            Some(f) => f.name.clone(),
            None => return Ok(()),
        };
        let main_clif_id = match self.func_ids.get(&main_name) {
            Some(&id) => id,
            None => return Ok(()),
        };

        let mut entry_sig = Signature::new(CallConv::SystemV);
        entry_sig.params.push(AbiParam::new(types::I64));
        let entry_id = self.obj.declare_function("rava_entry", Linkage::Export, &entry_sig)
            .map_err(|e| RavaError::Codegen(format!("declare rava_entry failed: {e}")))?;

        let mut clif_func = ir::Function::with_name_signature(
            ir::UserFuncName::user(0, entry_id.as_u32()),
            entry_sig,
        );

        let mut fb_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut clif_func, &mut fb_ctx);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);

        let args_val = builder.block_params(block)[0];
        let main_ref = self.obj.declare_func_in_func(main_clif_id, builder.func);
        let main_sig = builder.func.dfg.ext_funcs[main_ref].signature;
        let expected = builder.func.dfg.signatures[main_sig].params.len();
        if expected > 0 {
            builder.ins().call(main_ref, &[args_val]);
        } else {
            builder.ins().call(main_ref, &[]);
        }
        builder.ins().return_(&[]);
        builder.finalize();

        let mut ctx = cranelift_codegen::Context::for_function(clif_func);
        self.obj.define_function(entry_id, &mut ctx)
            .map_err(|e| RavaError::Codegen(format!("define rava_entry failed: {e}")))?;

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

        let mut block_map: HashMap<u32, ir::Block> = HashMap::new();
        for bb in &func.basic_blocks {
            let clif_block = builder.create_block();
            block_map.insert(bb.id.0, clif_block);
        }

        let mut var_map: HashMap<String, Variable> = HashMap::new();
        let mut var_counter = 0u32;
        let mut val_types: HashMap<String, ValType> = HashMap::new();

        let get_var = |name: &str, var_map: &mut HashMap<String, Variable>, var_counter: &mut u32| -> Variable {
            if let Some(&v) = var_map.get(name) {
                return v;
            }
            let v = Variable::new(*var_counter as usize);
            *var_counter += 1;
            var_map.insert(name.to_string(), v);
            v
        };

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

        let entry_block = block_map[&func.basic_blocks[0].id.0];
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        for (i, (param_name, _)) in func.params.iter().enumerate() {
            let param_val = builder.block_params(entry_block)[i];
            let var = var_map[&param_name.0];
            builder.def_var(var, param_val);
        }

        for (bb_idx, bb) in func.basic_blocks.iter().enumerate() {
            let clif_block = block_map[&bb.id.0];
            if bb_idx > 0 {
                builder.switch_to_block(clif_block);
            }
            for instr in &bb.instrs {
                self.translate_instr(
                    instr, &mut builder, &block_map, &mut var_map, &mut var_counter,
                    &mut val_types,
                )?;
            }
            if !block_ends_with_terminator(&bb.instrs) {
                builder.ins().return_(&[]);
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

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
        val_types: &mut HashMap<String, ValType>,
    ) -> Result<()> {
        match instr {
            RirInstr::ConstInt { ret, value } => {
                let v = builder.ins().iconst(types::I64, *value);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
                val_types.insert(ret.0.clone(), ValType::Int);
            }
            RirInstr::ConstFloat { ret, value } => {
                let v = builder.ins().f64const(*value);
                let bits = builder.ins().bitcast(types::I64, MemFlags::new(), v);
                self.def_val(builder, var_map, var_counter, &ret.0, bits);
                val_types.insert(ret.0.clone(), ValType::Float);
            }
            RirInstr::ConstStr { ret, value } => {
                if let Some(src) = value.strip_prefix("__copy__") {
                    let src_val = self.use_val(builder, var_map, var_counter, src);
                    self.def_val(builder, var_map, var_counter, &ret.0, src_val);
                    if let Some(&ty) = val_types.get(src) {
                        val_types.insert(ret.0.clone(), ty);
                    }
                } else {
                    let ptr = self.get_string_ptr(builder, value)?;
                    self.def_val(builder, var_map, var_counter, &ret.0, ptr);
                    val_types.insert(ret.0.clone(), ValType::Str);
                }
            }
            RirInstr::ConstBool { ret, value } => {
                let v = builder.ins().iconst(types::I64, if *value { 1 } else { 0 });
                self.def_val(builder, var_map, var_counter, &ret.0, v);
                val_types.insert(ret.0.clone(), ValType::Bool);
            }
            RirInstr::ConstNull { ret } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::BinOp { op, lhs, rhs, ret } => {
                let l_is_str = val_types.get(&lhs.0) == Some(&ValType::Str);
                let r_is_str = val_types.get(&rhs.0) == Some(&ValType::Str);
                if *op == BinOp::Add && (l_is_str || r_is_str) {
                    let l = self.use_val(builder, var_map, var_counter, &lhs.0);
                    let r = self.use_val(builder, var_map, var_counter, &rhs.0);
                    let l_str = if l_is_str {
                        l
                    } else {
                        let func_ref = self.obj.declare_func_in_func(self.rt_int_to_str, builder.func);
                        let inst = builder.ins().call(func_ref, &[l]);
                        builder.inst_results(inst)[0]
                    };
                    let r_str = if r_is_str {
                        r
                    } else {
                        let func_ref = self.obj.declare_func_in_func(self.rt_int_to_str, builder.func);
                        let inst = builder.ins().call(func_ref, &[r]);
                        builder.inst_results(inst)[0]
                    };
                    let concat_ref = self.obj.declare_func_in_func(self.rt_str_concat, builder.func);
                    let inst = builder.ins().call(concat_ref, &[l_str, r_str]);
                    let result = builder.inst_results(inst)[0];
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                    val_types.insert(ret.0.clone(), ValType::Str);
                } else if matches!(op, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge) {
                    let l = self.use_val(builder, var_map, var_counter, &lhs.0);
                    let r = self.use_val(builder, var_map, var_counter, &rhs.0);
                    let is_float = matches!(val_types.get(&lhs.0), Some(ValType::Float))
                        || matches!(val_types.get(&rhs.0), Some(ValType::Float));
                    let result = self.translate_binop(builder, op, l, r, is_float);
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                    val_types.insert(ret.0.clone(), ValType::Bool);
                } else {
                    let l = self.use_val(builder, var_map, var_counter, &lhs.0);
                    let r = self.use_val(builder, var_map, var_counter, &rhs.0);
                    let is_float = matches!(val_types.get(&lhs.0), Some(ValType::Float))
                        || matches!(val_types.get(&rhs.0), Some(ValType::Float));
                    let result = self.translate_binop(builder, op, l, r, is_float);
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                    if is_float {
                        val_types.insert(ret.0.clone(), ValType::Float);
                    }
                }
            }
            RirInstr::UnaryOp { op, operand, ret } => {
                let v = self.use_val(builder, var_map, var_counter, &operand.0);
                let is_float = matches!(val_types.get(&operand.0), Some(ValType::Float));
                let result = match op {
                    UnaryOp::Neg => {
                        if is_float {
                            let fv = builder.ins().bitcast(types::F64, MemFlags::new(), v);
                            let neg = builder.ins().fneg(fv);
                            builder.ins().bitcast(types::I64, MemFlags::new(), neg)
                        } else {
                            builder.ins().ineg(v)
                        }
                    }
                    UnaryOp::Not => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let cmp = builder.ins().icmp(IntCC::Equal, v, zero);
                        builder.ins().uextend(types::I64, cmp)
                    }
                };
                self.def_val(builder, var_map, var_counter, &ret.0, result);
                if is_float && matches!(op, UnaryOp::Neg) {
                    val_types.insert(ret.0.clone(), ValType::Float);
                }
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
                let first_arg_type = args.first()
                    .and_then(|a| val_types.get(&a.0).copied());
                self.translate_call(builder, func_id.0, &arg_vals, ret.as_ref(),
                    var_map, var_counter, first_arg_type, val_types)?;
            }
            RirInstr::Convert { val, to, ret, .. } => {
                let v = self.use_val(builder, var_map, var_counter, &val.0);
                let result = match to {
                    RirType::F32 | RirType::F64 => v,
                    _ => v,
                };
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::New { class, ret } => {
                let num_fields = self.field_slots.len() as i64;
                let nf = builder.ins().iconst(types::I64, num_fields);
                let func_ref = self.obj.declare_func_in_func(self.rt_obj_alloc, builder.func);
                let inst = builder.ins().call(func_ref, &[nf]);
                let result = builder.inst_results(inst)[0];
                let tag_val = self.class_tags.get(&class.0).copied().unwrap_or(0);
                let tag = builder.ins().iconst(types::I64, tag_val);
                let set_tag_ref = self.obj.declare_func_in_func(self.rt_obj_set_tag, builder.func);
                builder.ins().call(set_tag_ref, &[result, tag]);
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::GetField { obj: obj_val, field, ret } => {
                use rava_frontend::lowerer::encode_builtin;
                let ptr = self.use_val(builder, var_map, var_counter, &obj_val.0);
                let length_hash = encode_builtin("length");
                if field.0 == length_hash {
                    let func_ref = self.obj.declare_func_in_func(self.rt_arr_len, builder.func);
                    let inst = builder.ins().call(func_ref, &[ptr]);
                    let result = builder.inst_results(inst)[0];
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                } else {
                    let slot = self.field_slots.get(&field.0).copied().unwrap_or(0) as i64;
                    let slot_val = builder.ins().iconst(types::I64, slot);
                    let func_ref = self.obj.declare_func_in_func(self.rt_obj_get_field, builder.func);
                    let inst = builder.ins().call(func_ref, &[ptr, slot_val]);
                    let result = builder.inst_results(inst)[0];
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                }
            }
            RirInstr::GetStatic { ret, .. } => {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &ret.0, v);
            }
            RirInstr::SetField { obj: obj_val, field, val } => {
                let ptr = self.use_val(builder, var_map, var_counter, &obj_val.0);
                let slot = self.field_slots.get(&field.0).copied().unwrap_or(0) as i64;
                let slot_val = builder.ins().iconst(types::I64, slot);
                let value = self.use_val(builder, var_map, var_counter, &val.0);
                let func_ref = self.obj.declare_func_in_func(self.rt_obj_set_field, builder.func);
                builder.ins().call(func_ref, &[ptr, slot_val, value]);
            }
            RirInstr::SetStatic { .. } => {}
            RirInstr::NewArray { len, ret, .. } => {
                let length = self.use_val(builder, var_map, var_counter, &len.0);
                let func_ref = self.obj.declare_func_in_func(self.rt_arr_alloc, builder.func);
                let inst = builder.ins().call(func_ref, &[length]);
                let result = builder.inst_results(inst)[0];
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::NewMultiArray { dims, ret, .. } => {
                if let Some(first) = dims.first() {
                    let length = self.use_val(builder, var_map, var_counter, &first.0);
                    let func_ref = self.obj.declare_func_in_func(self.rt_arr_alloc, builder.func);
                    let inst = builder.ins().call(func_ref, &[length]);
                    let result = builder.inst_results(inst)[0];
                    self.def_val(builder, var_map, var_counter, &ret.0, result);
                }
            }
            RirInstr::ArrayLoad { arr, idx, ret } => {
                let arr_ptr = self.use_val(builder, var_map, var_counter, &arr.0);
                let index = self.use_val(builder, var_map, var_counter, &idx.0);
                let func_ref = self.obj.declare_func_in_func(self.rt_arr_load, builder.func);
                let inst = builder.ins().call(func_ref, &[arr_ptr, index]);
                let result = builder.inst_results(inst)[0];
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::ArrayStore { arr, idx, val } => {
                let arr_ptr = self.use_val(builder, var_map, var_counter, &arr.0);
                let index = self.use_val(builder, var_map, var_counter, &idx.0);
                let value = self.use_val(builder, var_map, var_counter, &val.0);
                let func_ref = self.obj.declare_func_in_func(self.rt_arr_store, builder.func);
                builder.ins().call(func_ref, &[arr_ptr, index, value]);
            }
            RirInstr::ArrayLen { arr, ret } => {
                let arr_ptr = self.use_val(builder, var_map, var_counter, &arr.0);
                let func_ref = self.obj.declare_func_in_func(self.rt_arr_len, builder.func);
                let inst = builder.ins().call(func_ref, &[arr_ptr]);
                let result = builder.inst_results(inst)[0];
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::Instanceof { obj, class, ret } => {
                let obj_val = self.use_val(builder, var_map, var_counter, &obj.0);
                let get_tag_ref = self.obj.declare_func_in_func(self.rt_obj_get_tag, builder.func);
                let inst = builder.ins().call(get_tag_ref, &[obj_val]);
                let actual_tag = builder.inst_results(inst)[0];
                let expected_tag = self.class_tags.get(&class.0).copied().unwrap_or(0);
                let expected = builder.ins().iconst(types::I64, expected_tag);
                let cmp = builder.ins().icmp(IntCC::Equal, actual_tag, expected);
                let result = builder.ins().uextend(types::I64, cmp);
                self.def_val(builder, var_map, var_counter, &ret.0, result);
            }
            RirInstr::Checkcast { .. } => {}
            RirInstr::Throw(_) => {
                builder.ins().trap(ir::TrapCode::unwrap_user(0));
            }
            RirInstr::Unreachable => {
                builder.ins().trap(ir::TrapCode::unwrap_user(1));
            }
            RirInstr::MonitorEnter(_) | RirInstr::MonitorExit(_) => {}
            RirInstr::CallVirtual { receiver, method, args, ret } |
            RirInstr::CallInterface { receiver, method, args, ret } => {
                let recv = self.use_val(builder, var_map, var_counter, &receiver.0);
                let mut arg_vals = vec![recv];
                for a in args {
                    arg_vals.push(self.use_val(builder, var_map, var_counter, &a.0));
                }
                let mut found = false;
                for (name, &clif_id) in &self.func_ids {
                    use rava_frontend::lowerer::encode_builtin;
                    let name_hash = encode_builtin(name);
                    let short_hash = name.rsplit('.').next()
                        .map(|s| encode_builtin(s))
                        .unwrap_or(0);
                    if name_hash == method.0 || short_hash == method.0 {
                        let func_ref = self.obj.declare_func_in_func(clif_id, builder.func);
                        let sig = builder.func.dfg.ext_funcs[func_ref].signature;
                        let expected = builder.func.dfg.signatures[sig].params.len();
                        let call_args = if arg_vals.len() > expected {
                            &arg_vals[..expected]
                        } else {
                            &arg_vals
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
                        found = true;
                        break;
                    }
                }
                if !found {
                    if let Some(r) = ret {
                        let v = builder.ins().iconst(types::I64, 0);
                        self.def_val(builder, var_map, var_counter, &r.0, v);
                    }
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

    fn translate_binop(&self, builder: &mut FunctionBuilder, op: &BinOp, l: ir::Value, r: ir::Value, is_float: bool) -> ir::Value {
        if is_float {
            let lf = builder.ins().bitcast(types::F64, MemFlags::new(), l);
            let rf = builder.ins().bitcast(types::F64, MemFlags::new(), r);
            match op {
                BinOp::Add => {
                    let res = builder.ins().fadd(lf, rf);
                    builder.ins().bitcast(types::I64, MemFlags::new(), res)
                }
                BinOp::Sub => {
                    let res = builder.ins().fsub(lf, rf);
                    builder.ins().bitcast(types::I64, MemFlags::new(), res)
                }
                BinOp::Mul => {
                    let res = builder.ins().fmul(lf, rf);
                    builder.ins().bitcast(types::I64, MemFlags::new(), res)
                }
                BinOp::Div => {
                    let res = builder.ins().fdiv(lf, rf);
                    builder.ins().bitcast(types::I64, MemFlags::new(), res)
                }
                BinOp::Rem => {
                    let div = builder.ins().fdiv(lf, rf);
                    let trunc = builder.ins().trunc(div);
                    let prod = builder.ins().fmul(trunc, rf);
                    let res = builder.ins().fsub(lf, prod);
                    builder.ins().bitcast(types::I64, MemFlags::new(), res)
                }
                BinOp::Eq => {
                    let cmp = builder.ins().fcmp(FloatCC::Equal, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                BinOp::Ne => {
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                BinOp::Lt => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThan, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                BinOp::Le => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                BinOp::Gt => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                BinOp::Ge => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lf, rf);
                    builder.ins().uextend(types::I64, cmp)
                }
                _ => self.translate_binop(builder, op, l, r, false),
            }
        } else {
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
    }

    fn translate_call(
        &mut self,
        builder: &mut FunctionBuilder,
        func_id: u32,
        args: &[ir::Value],
        ret: Option<&rava_rir::Value>,
        var_map: &mut HashMap<String, Variable>,
        var_counter: &mut u32,
        first_arg_type: Option<ValType>,
        val_types: &mut HashMap<String, ValType>,
    ) -> Result<()> {
        use rava_frontend::lowerer::encode_builtin;

        if func_id == encode_builtin("System.out.println") {
            if args.is_empty() {
                let void_ref = self.obj.declare_func_in_func(self.rt_println_void, builder.func);
                builder.ins().call(void_ref, &[]);
            } else {
                let rt_func = match first_arg_type {
                    Some(ValType::Str) => self.rt_println_str,
                    Some(ValType::Bool) => self.rt_println_bool,
                    Some(ValType::Float) => {
                        let f = builder.ins().bitcast(types::F64, MemFlags::new(), args[0]);
                        let func_ref = self.obj.declare_func_in_func(self.rt_println_float, builder.func);
                        builder.ins().call(func_ref, &[f]);
                        if let Some(r) = ret {
                            let v = builder.ins().iconst(types::I64, 0);
                            self.def_val(builder, var_map, var_counter, &r.0, v);
                        }
                        return Ok(());
                    }
                    _ => self.rt_println_int,
                };
                let func_ref = self.obj.declare_func_in_func(rt_func, builder.func);
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
                let rt_func = match first_arg_type {
                    Some(ValType::Str) => self.rt_print_str,
                    _ => self.rt_print_int,
                };
                let func_ref = self.obj.declare_func_in_func(rt_func, builder.func);
                builder.ins().call(func_ref, &[args[0]]);
            }
            if let Some(r) = ret {
                let v = builder.ins().iconst(types::I64, 0);
                self.def_val(builder, var_map, var_counter, &r.0, v);
            }
            return Ok(());
        }

        if func_id == encode_builtin("__str_concat__") {
            let func_ref = self.obj.declare_func_in_func(self.rt_str_concat, builder.func);
            let inst = builder.ins().call(func_ref, &[args[0], args[1]]);
            if let Some(r) = ret {
                let results = builder.inst_results(inst);
                self.def_val(builder, var_map, var_counter, &r.0, results[0]);
                val_types.insert(r.0.clone(), ValType::Str);
            }
            return Ok(());
        }

        if func_id == encode_builtin("__int_to_str__") {
            let func_ref = self.obj.declare_func_in_func(self.rt_int_to_str, builder.func);
            let inst = builder.ins().call(func_ref, &[args[0]]);
            if let Some(r) = ret {
                let results = builder.inst_results(inst);
                self.def_val(builder, var_map, var_counter, &r.0, results[0]);
                val_types.insert(r.0.clone(), ValType::Str);
            }
            return Ok(());
        }

        for (name, &clif_id) in &self.func_ids {
            let name_match = encode_builtin(name) == func_id
                || name.rsplit('.').next()
                    .map(|s| encode_builtin(s) == func_id)
                    .unwrap_or(false)
                || name.rsplit('.').next()
                    .map(|s| encode_builtin(&format!("__method__{s}")) == func_id)
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
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(v, zero);
            v
        };
        builder.use_var(var)
    }

    fn get_string_ptr(&mut self, builder: &mut FunctionBuilder, s: &str) -> Result<ir::Value> {
        let data_id = if let Some(&id) = self.str_constants.get(s) {
            id
        } else {
            let mut data_bytes = s.as_bytes().to_vec();
            data_bytes.push(0);

            let data_id = self.obj.declare_data(
                &format!(".str.{}", self.str_constants.len()),
                Linkage::Local,
                false,
                false,
            ).map_err(|e| RavaError::Codegen(format!("declare string data failed: {e}")))?;

            let mut data_desc = cranelift_module::DataDescription::new();
            data_desc.define(data_bytes.into_boxed_slice());
            self.obj.define_data(data_id, &data_desc)
                .map_err(|e| RavaError::Codegen(format!("define string data failed: {e}")))?;

            self.str_constants.insert(s.to_string(), data_id);
            data_id
        };

        let gv = self.obj.declare_data_in_func(data_id, builder.func);
        let ptr = builder.ins().global_value(types::I64, gv);
        Ok(ptr)
    }
}
