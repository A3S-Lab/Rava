//! Core interpreter methods: new, run_main, exec_function, exec_function_idx, exec_instr.

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use rava_common::error::{RavaError, Result};
use rava_rir::RirInstr;
use super::rval::RVal;
use super::RirInterpreter;

impl RirInterpreter {
    pub fn run_main(&self) -> Result<()> {
        let clinit_names: Vec<String> = self.module.functions.iter()
            .filter(|f| f.flags.is_clinit)
            .map(|f| f.name.clone())
            .collect();
        for name in clinit_names {
            self.exec_function(&name, HashMap::new())?;
        }
        let main_name = self.module.functions.iter()
            .find(|f| f.name.ends_with(".main"))
            .map(|f| f.name.clone())
            .ok_or_else(|| RavaError::Other("no main method found".into()))?;
        let mut env: HashMap<String, RVal> = HashMap::new();
        env.insert("args".into(), RVal::Null);
        self.exec_function(&main_name, env)?;
        Ok(())
    }

    pub(super) fn exec_function(&self, name: &str, args: HashMap<String, RVal>) -> Result<RVal> {
        let idx = self.module.functions.iter()
            .position(|f| f.name == name)
            .ok_or_else(|| RavaError::Other(format!("function not found: {name}")))?;
        self.exec_function_idx(idx, args)
    }

    pub(super) fn exec_function_idx(&self, func_idx: usize, args: HashMap<String, RVal>) -> Result<RVal> {
        let func = &self.module.functions[func_idx];
        let mut env = args;
        let mut block_idx = 0usize;
        let mut exception_handlers: Vec<(u32, Vec<String>)> = Vec::new();

        loop {
            let block = func.basic_blocks.get(block_idx)
                .ok_or_else(|| RavaError::Other("invalid block index".into()))?;

            let mut next_block: Option<usize> = None;
            let mut returned:   Option<RVal>  = None;
            let mut thrown: Option<(String, String, Option<RVal>)> = None;

            for instr in &block.instrs {
                let result = self.exec_instr(instr, &mut env, func, &mut next_block, &mut returned, &mut exception_handlers);
                match result {
                    Ok(()) => {}
                    Err(RavaError::JavaException { exception_type, message }) => {
                        // Preserve the original thrown object if it's in env as the last Throw operand
                        let obj = env.get("__last_thrown__").cloned();
                        thrown = Some((exception_type, message, obj));
                        break;
                    }
                    Err(e) => return Err(e),
                }
                if next_block.is_some() || returned.is_some() { break; }
            }

            if let Some((exc_type, exc_msg, orig_obj)) = thrown {
                let mut matched = None;
                while let Some((block_id, types)) = exception_handlers.pop() {
                    if types.is_empty() || types.iter().any(|t| self.exception_matches(&exc_type, t)) {
                        matched = Some(block_id);
                        break;
                    }
                }
                if let Some(catch_block_id) = matched {
                    // Use original thrown object if available, otherwise create a new one
                    let orig_obj = super::THROWN_OBJ.with(|t: &RefCell<Option<RVal>>| t.borrow_mut().take());
                    let exc_rval = if let Some(RVal::Object(_)) = orig_obj {
                        orig_obj.unwrap()
                    } else {
                        let exc_obj_id = self.alloc_object(&exc_type);
                        {
                            let mut heap = self.heap.borrow_mut();
                            if let Some(obj) = heap.get_mut(&exc_obj_id) {
                                obj.fields.insert("message".into(), RVal::Str(exc_msg.clone()));
                            }
                        }
                        RVal::Object(exc_obj_id)
                    };
                    env.insert("__exception__".into(), exc_rval);
                    env.insert("__exception_type__".into(), RVal::Str(exc_type));
                    next_block = self.find_block_idx(func, catch_block_id);
                } else {
                    return Err(RavaError::JavaException {
                        exception_type: exc_type,
                        message: exc_msg,
                    });
                }
            }

            if let Some(val) = returned { return Ok(val); }
            match next_block {
                Some(idx) => block_idx = idx,
                None      => block_idx += 1,
            }
        }
    }

    pub(super) fn exec_instr(
        &self,
        instr: &RirInstr,
        env: &mut HashMap<String, RVal>,
        func: &rava_rir::RirFunction,
        next_block: &mut Option<usize>,
        returned: &mut Option<RVal>,
        exception_handlers: &mut Vec<(u32, Vec<String>)>,
    ) -> Result<()> {
        match instr {
            RirInstr::ConstInt { ret, value } => {
                env.insert(ret.0.clone(), RVal::Int(*value));
            }
            RirInstr::ConstFloat { ret, value } => {
                env.insert(ret.0.clone(), RVal::Float(*value));
            }
            RirInstr::ConstBool { ret, value } => {
                env.insert(ret.0.clone(), RVal::Bool(*value));
            }
            RirInstr::ConstStr { ret, value } => {
                if let Some(rest) = value.strip_prefix("__try_catch__") {
                    if let Some((id_str, types_str)) = rest.split_once(':') {
                        if let Ok(id) = id_str.parse::<u32>() {
                            let types: Vec<String> = if types_str.is_empty() {
                                vec![]
                            } else {
                                types_str.split('|').map(|s| s.to_string()).collect()
                            };
                            exception_handlers.push((id, types));
                        }
                    } else if let Ok(id) = rest.parse::<u32>() {
                        exception_handlers.push((id, vec![]));
                    }
                } else if value == "__try_end__" {
                    exception_handlers.pop();
                } else if value == "__exception__" {
                    let val = env.get("__exception__").cloned().unwrap_or(RVal::Null);
                    env.insert(ret.0.clone(), val);
                } else if let Some(src_name) = value.strip_prefix("__copy__") {
                    let val = env.get(src_name).cloned().unwrap_or(RVal::Null);
                    env.insert(ret.0.clone(), val);
                } else if let Some(resolved) = self.resolve_synthetic(value, env) {
                    env.insert(ret.0.clone(), resolved);
                } else {
                    // If this is a lambda, store the current env as its captures
                    if value.starts_with("__lambda_") {
                        if let Some(func) = self.module.functions.iter().find(|f| f.name == *value) {
                            // Captures are params that aren't in the lambda's declared params
                            // They're stored as Value(name) in the function body
                            let param_names: std::collections::HashSet<&str> =
                                func.params.iter().map(|(v, _)| v.0.as_str()).collect();
                            // Find all vars referenced in the function that aren't params
                            let captures: HashMap<String, RVal> = env.iter()
                                .filter(|(k, _)| !param_names.contains(k.as_str())
                                    && !k.starts_with("__")
                                    && k.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false))
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            super::LAMBDA_CAPTURES.with(|lc| {
                                lc.borrow_mut().insert(value.clone(), captures);
                            });
                        }
                    }
                    env.insert(ret.0.clone(), RVal::Str(value.clone()));
                }
            }
            RirInstr::ConstNull { ret } => {
                env.insert(ret.0.clone(), RVal::Null);
            }
            RirInstr::BinOp { op, lhs, rhs, ret } => {
                let l = self.resolve(env, lhs);
                let r = self.resolve(env, rhs);
                env.insert(ret.0.clone(), self.eval_binop(op, &l, &r)?);
            }
            RirInstr::UnaryOp { op, operand, ret } => {
                let v = self.resolve(env, operand);
                let result = match op {
                    rava_rir::UnaryOp::Neg => {
                        if v.is_float() { RVal::Float(-v.as_float()) }
                        else { RVal::Int(-v.as_int()) }
                    }
                    rava_rir::UnaryOp::Not => RVal::Bool(!v.is_truthy()),
                };
                env.insert(ret.0.clone(), result);
            }
            RirInstr::Call { func: func_id, args: arg_vals, ret } => {
                let result = self.dispatch_call(func_id.0, arg_vals, env)?;
                if let Some(r) = ret {
                    env.insert(r.0.clone(), result);
                } else if let RVal::Array(_) = &result {
                    // <init> calls (e.g. ArrayList.<init>) return the constructed collection
                    // but have ret=None; update the `this` variable (first arg) in env.
                    if let Some(this_var) = arg_vals.first() {
                        env.insert(this_var.0.clone(), result);
                    }
                }
            }
            RirInstr::New { class, ret } => {
                let class_name = self.class_name_for(class.0);
                if class_name == "ArrayList" || class_name == "LinkedList" {
                    env.insert(ret.0.clone(), RVal::Array(Rc::new(RefCell::new(Vec::new()))));
                } else if class_name == "HashSet" || class_name == "TreeSet" || class_name == "LinkedHashSet" {
                    let id = self.alloc_object(&class_name);
                    {
                        let mut heap = self.heap.borrow_mut();
                        if let Some(obj) = heap.get_mut(&id) {
                            obj.fields.insert("__items__".into(), RVal::Array(Rc::new(RefCell::new(vec![]))));
                            obj.fields.insert("__type__".into(), RVal::Str("set".into()));
                        }
                    }
                    env.insert(ret.0.clone(), RVal::Object(id));
                } else if class_name == "PriorityQueue" {
                    let id = self.alloc_object("PriorityQueue");
                    {
                        let mut heap = self.heap.borrow_mut();
                        if let Some(obj) = heap.get_mut(&id) {
                            obj.fields.insert("__items__".into(), RVal::Array(Rc::new(RefCell::new(vec![]))));
                        }
                    }
                    env.insert(ret.0.clone(), RVal::Object(id));
                } else {
                    let id = self.alloc_object(&class_name);
                    env.insert(ret.0.clone(), RVal::Object(id));
                }
            }
            RirInstr::GetField { obj, field, ret } => {
                let obj_val = self.resolve(env, obj);
                let val = self.get_field(&obj_val, field.0);
                env.insert(ret.0.clone(), val);
            }
            RirInstr::SetField { obj, field, val } => {
                let obj_val = self.resolve(env, obj);
                let v = self.resolve(env, val);
                self.set_field(&obj_val, field.0, v);
            }
            RirInstr::GetStatic { field, ret } => {
                use crate::lowerer_hash::encode_builtin;
                // Built-in constants
                let builtin_val = if field.0 == encode_builtin("Math.PI") {
                    Some(RVal::Float(std::f64::consts::PI))
                } else if field.0 == encode_builtin("Math.E") {
                    Some(RVal::Float(std::f64::consts::E))
                } else if field.0 == encode_builtin("Integer.MAX_VALUE") {
                    Some(RVal::Int(i32::MAX as i64))
                } else if field.0 == encode_builtin("Integer.MIN_VALUE") {
                    Some(RVal::Int(i32::MIN as i64))
                } else if field.0 == encode_builtin("Long.MAX_VALUE") {
                    Some(RVal::Int(i64::MAX))
                } else if field.0 == encode_builtin("Long.MIN_VALUE") {
                    Some(RVal::Int(i64::MIN))
                } else {
                    None
                };
                if let Some(v) = builtin_val {
                    env.insert(ret.0.clone(), v);
                } else {
                    let key = format!("static#{}", field.0);
                    let val = self.static_fields.borrow().get(&key).cloned().unwrap_or(RVal::Null);
                    env.insert(ret.0.clone(), val);
                }
            }
            RirInstr::SetStatic { field, val } => {
                let key = format!("static#{}", field.0);
                let v = self.resolve(env, val);
                self.static_fields.borrow_mut().insert(key, v);
            }
            RirInstr::NewArray { len, ret, .. } => {
                let n = self.resolve(env, len).as_int().max(0) as usize;
                let arr = Rc::new(RefCell::new(vec![RVal::Int(0); n]));
                env.insert(ret.0.clone(), RVal::Array(arr));
            }
            RirInstr::NewMultiArray { dims, ret, .. } => {
                let dim_sizes: Vec<usize> = dims.iter()
                    .map(|d| self.resolve(env, d).as_int().max(0) as usize)
                    .collect();
                let arr = self.create_multi_array(&dim_sizes, 0);
                env.insert(ret.0.clone(), arr);
            }
            RirInstr::ArrayLoad { arr, idx, ret } => {
                let arr_val = self.resolve(env, arr);
                let i = self.resolve(env, idx).as_int();
                let val = match &arr_val {
                    RVal::Array(a) => {
                        let borrow = a.borrow();
                        if i < 0 || i as usize >= borrow.len() {
                            return Err(RavaError::JavaException {
                                exception_type: "ArrayIndexOutOfBoundsException".into(),
                                message: format!("Index {} out of bounds for length {}", i, borrow.len()),
                            });
                        }
                        borrow[i as usize].clone()
                    }
                    RVal::Null => return Err(RavaError::JavaException {
                        exception_type: "NullPointerException".into(),
                        message: "null array access".into(),
                    }),
                    _ => RVal::Null,
                };
                env.insert(ret.0.clone(), val);
            }
            RirInstr::ArrayStore { arr, idx, val } => {
                let arr_val = self.resolve(env, arr);
                let i = self.resolve(env, idx).as_int() as usize;
                let v = self.resolve(env, val);
                if let RVal::Array(a) = &arr_val {
                    let mut borrow = a.borrow_mut();
                    while i >= borrow.len() { borrow.push(RVal::Null); }
                    borrow[i] = v;
                }
            }
            RirInstr::ArrayLen { arr, ret } => {
                let arr_val = self.resolve(env, arr);
                let len = match &arr_val {
                    RVal::Array(a) => a.borrow().len() as i64,
                    RVal::Str(s)   => s.len() as i64,
                    _ => 0,
                };
                env.insert(ret.0.clone(), RVal::Int(len));
            }
            RirInstr::Convert { val, to, ret, .. } => {
                let v = self.resolve(env, val);
                let converted = match to {
                    rava_rir::RirType::I32 | rava_rir::RirType::I64 => RVal::Int(v.as_int()),
                    rava_rir::RirType::F32 | rava_rir::RirType::F64 => RVal::Float(v.as_float()),
                    rava_rir::RirType::Bool => RVal::Bool(v.is_truthy()),
                    _ => v,
                };
                env.insert(ret.0.clone(), converted);
            }
            RirInstr::Checkcast { obj, class } => {
                let obj_val = self.resolve(env, obj);
                let class_name = self.class_name_for(class.0);
                if let RVal::Object(id) = &obj_val {
                    let actual = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    // Allow cast if same class, or if target is a known supertype
                    let ok = actual == class_name
                        || class_name == "Object"
                        || self.find_method_in_chain(&actual, "<init>", 1).is_some();
                    if !ok {
                        return Err(RavaError::JavaException {
                            exception_type: "ClassCastException".into(),
                            message: format!("{} cannot be cast to {}", actual, class_name),
                        });
                    }
                } else if let RVal::Str(_) = &obj_val {
                    // String cast to non-String type
                    if class_name != "String" && class_name != "Object" && class_name != "CharSequence" && class_name != "Comparable" {
                        return Err(RavaError::JavaException {
                            exception_type: "ClassCastException".into(),
                            message: format!("String cannot be cast to {}", class_name),
                        });
                    }
                }
            }
            RirInstr::Return(val) => {
                *returned = Some(match val {
                    Some(v) => self.resolve(env, v),
                    None    => RVal::Void,
                });
                return Ok(());
            }
            RirInstr::Jump(target) => {
                *next_block = self.find_block_idx(func, target.0);
                return Ok(());
            }
            RirInstr::Branch { cond, then_bb, else_bb } => {
                let cv = self.resolve(env, cond);
                let target = if cv.is_truthy() { then_bb.0 } else { else_bb.0 };
                *next_block = self.find_block_idx(func, target);
                return Ok(());
            }
            RirInstr::Throw(val) => {
                let thrown_val = self.resolve(env, val);
                // Store the original thrown object in thread-local so catch handlers can use it
                super::THROWN_OBJ.with(|t: &RefCell<Option<RVal>>| { *t.borrow_mut() = Some(thrown_val.clone()); });
                let (exc_type, msg) = match &thrown_val {
                    RVal::Object(id) => {
                        let class_name = self.heap.borrow().get(id)
                            .map(|o| o.class_name.clone())
                            .unwrap_or_else(|| "Exception".into());
                        let message = self.heap.borrow().get(id)
                            .and_then(|o| o.fields.get("__arg0__").or(o.fields.get("message")).cloned())
                            .map(|v| v.to_display())
                            .unwrap_or_default();
                        (class_name, message)
                    }
                    RVal::Str(s) => ("Exception".into(), s.clone()),
                    _ => ("Exception".into(), thrown_val.to_display()),
                };
                return Err(RavaError::JavaException {
                    exception_type: exc_type,
                    message: msg,
                });
            }
            RirInstr::Instanceof { obj, class, ret } => {
                let obj_val = self.resolve(env, obj);
                let class_name = self.class_name_for(class.0);
                let result = match &obj_val {
                    RVal::Object(id) => {
                        let obj_class = self.heap.borrow().get(id)
                            .map(|o| o.class_name.clone())
                            .unwrap_or_default();
                        self.is_instance_of(&obj_class, &class_name)
                    }
                    RVal::Str(_)   => class_name == "String" || class_name == "Object" || class_name == "CharSequence" || class_name == "Comparable",
                    RVal::Array(_) => class_name.ends_with("[]") || class_name == "Object",
                    RVal::Int(_)   => matches!(class_name.as_str(), "Integer" | "Long" | "Short" | "Byte" | "Number" | "Object" | "Comparable"),
                    RVal::Float(_) => matches!(class_name.as_str(), "Double" | "Float" | "Number" | "Object" | "Comparable"),
                    RVal::Bool(_)  => matches!(class_name.as_str(), "Boolean" | "Object" | "Comparable"),
                    _ => false,
                };
                env.insert(ret.0.clone(), RVal::Bool(result));
            }
            RirInstr::CallVirtual { receiver, args: arg_vals, ret, .. } => {
                let recv = self.resolve(env, receiver);
                let result = self.dispatch_virtual(recv, arg_vals, env)?;
                if let Some(r) = ret { env.insert(r.0.clone(), result); }
            }
            RirInstr::CallInterface { receiver, args: arg_vals, ret, .. } => {
                let recv = self.resolve(env, receiver);
                let result = self.dispatch_virtual(recv, arg_vals, env)?;
                if let Some(r) = ret { env.insert(r.0.clone(), result); }
            }
            RirInstr::Unreachable => {
                return Err(RavaError::Other("reached unreachable code".into()));
            }
            RirInstr::MonitorEnter(_) | RirInstr::MonitorExit(_) => {}
            RirInstr::MicroRtReflect { ret, .. } |
            RirInstr::MicroRtProxy { ret, .. } |
            RirInstr::MicroRtClassLoad { ret, .. } => {
                env.insert(ret.0.clone(), RVal::Null);
            }
        }
        Ok(())
    }
}
