//! RIR interpreter — executes RIR directly with full Java semantics.
//!
//! Supports: all arithmetic (int + float), string operations, object fields,
//! arrays, static fields, user-defined methods, break/continue (via control flow),
//! ternary (via branching), type conversion, instanceof, try/catch (simplified).

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use rava_common::error::{RavaError, Result};
use rava_rir::{RirInstr, RirModule, Value};
use crate::builtins;

/// All known instance method names — used to reverse-lookup __method__<name> calls.
const KNOWN_METHODS: &[&str] = &[
    // String
    "length", "isEmpty", "toUpperCase", "toLowerCase", "trim",
    "charAt", "substring", "contains", "startsWith", "endsWith",
    "equals", "equalsIgnoreCase", "replace", "indexOf", "split",
    "toString", "hashCode", "compareTo", "toCharArray", "valueOf",
    "format", "join", "strip", "stripLeading", "stripTrailing",
    "repeat", "chars", "codePointAt", "lastIndexOf",
    // ArrayList / array
    "size", "add", "get", "set", "remove", "clear",
    "addAll", "removeAll", "iterator", "toArray", "sort",
    "subList", "indexOf",
    // HashMap
    "put", "getOrDefault", "containsKey", "containsValue",
    "keySet", "values", "entrySet",
    // StringBuilder
    "append", "insert", "reverse", "deleteCharAt",
    // Object
    "getClass", "notify", "wait",
];

// ── Runtime values ────────────────────────────────────────────────────────────

pub type ObjId = u64;

/// A heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct JavaObject {
    pub class_name: String,
    pub fields:     HashMap<String, RVal>,
}

/// A runtime value.
#[derive(Debug, Clone)]
pub enum RVal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Object(ObjId),
    Array(Rc<RefCell<Vec<RVal>>>),
    Null,
    Void,
}

impl RVal {
    pub(crate) fn as_int(&self) -> i64 {
        match self {
            RVal::Int(n)    => *n,
            RVal::Float(f)  => *f as i64,
            RVal::Bool(b)   => if *b { 1 } else { 0 },
            RVal::Str(s)    => s.parse::<i64>().unwrap_or(0),
            _               => 0,
        }
    }

    pub(crate) fn as_float(&self) -> f64 {
        match self {
            RVal::Float(f) => *f,
            RVal::Int(n)   => *n as f64,
            RVal::Bool(b)  => if *b { 1.0 } else { 0.0 },
            RVal::Str(s)   => s.parse::<f64>().unwrap_or(0.0),
            _              => 0.0,
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            RVal::Bool(b)   => *b,
            RVal::Int(n)    => *n != 0,
            RVal::Float(f)  => *f != 0.0,
            RVal::Null      => false,
            RVal::Void      => false,
            RVal::Str(s)    => !s.is_empty(),
            RVal::Object(_) => true,
            RVal::Array(_)  => true,
        }
    }

    fn is_float(&self) -> bool {
        matches!(self, RVal::Float(_))
    }

    pub fn to_display(&self) -> String {
        match self {
            RVal::Int(n)    => n.to_string(),
            RVal::Float(f)  => {
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{:.1}", f) }
                else { f.to_string() }
            }
            RVal::Str(s)    => s.clone(),
            RVal::Bool(b)   => b.to_string(),
            RVal::Null      => "null".into(),
            RVal::Void      => "".into(),
            RVal::Object(id) => format!("Object@{id}"),
            RVal::Array(arr) => {
                let v = arr.borrow();
                let items: Vec<_> = v.iter().map(|x| x.to_display()).collect();
                format!("[{}]", items.join(", "))
            }
        }
    }
}

// ── Interpreter ───────────────────────────────────────────────────────────────

pub struct RirInterpreter {
    module:        RirModule,
    heap:          RefCell<HashMap<ObjId, JavaObject>>,
    next_id:       RefCell<ObjId>,
    static_fields: RefCell<HashMap<String, RVal>>,
}

impl RirInterpreter {
    pub fn new(module: RirModule) -> Self {
        Self {
            module,
            heap:          RefCell::new(HashMap::new()),
            next_id:       RefCell::new(1),
            static_fields: RefCell::new(HashMap::new()),
        }
    }

    fn alloc_object(&self, class_name: &str) -> ObjId {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        self.heap.borrow_mut().insert(id, JavaObject {
            class_name: class_name.to_string(),
            fields: HashMap::new(),
        });
        id
    }

    /// Reverse-lookup a field name from its hash, using the module's field_names registry.
    fn field_name_for(&self, field_id: u32) -> String {
        if let Some(name) = self.module.field_names.get(&field_id) {
            return name.clone();
        }
        // Fallback: check common names
        for name in &["length", "size", "value", "name", "x", "y", "z",
                       "width", "height", "key", "data", "next", "prev",
                       "left", "right", "parent", "count", "index", "head", "tail"] {
            if crate::lowerer_hash::encode_builtin(name) == field_id {
                return name.to_string();
            }
        }
        format!("__field_{field_id}")
    }

    /// Resolve a __method__<name> hash back to the method name.
    /// Checks known builtins first, then user-defined methods from the module.
    fn resolve_method_name(&self, func_id: u32) -> Option<String> {
        // Check known builtin method names
        for method_name in KNOWN_METHODS {
            let key = format!("__method__{}", method_name);
            if crate::lowerer_hash::encode_builtin(&key) == func_id {
                return Some(method_name.to_string());
            }
        }
        // Check user-defined method names from the module
        for func in &self.module.functions {
            if func.flags.is_constructor || func.flags.is_clinit { continue; }
            if let Some(method_name) = func.name.rsplit('.').next() {
                let key = format!("__method__{}", method_name);
                if crate::lowerer_hash::encode_builtin(&key) == func_id {
                    return Some(method_name.to_string());
                }
            }
        }
        None
    }

    pub fn run_main(&self) -> Result<()> {
        let main_name = self.module.functions.iter()
            .find(|f| f.name.ends_with(".main"))
            .map(|f| f.name.clone())
            .ok_or_else(|| RavaError::Other("no main method found".into()))?;

        let mut env: HashMap<String, RVal> = HashMap::new();
        env.insert("args".into(), RVal::Null);
        self.exec_function(&main_name, env)?;
        Ok(())
    }

    fn exec_function(&self, name: &str, args: HashMap<String, RVal>) -> Result<RVal> {
        let idx = self.module.functions.iter()
            .position(|f| f.name == name)
            .ok_or_else(|| RavaError::Other(format!("function not found: {name}")))?;
        self.exec_function_idx(idx, args)
    }

    fn exec_function_idx(&self, func_idx: usize, args: HashMap<String, RVal>) -> Result<RVal> {
        let func = &self.module.functions[func_idx];

        let mut env = args;
        let mut block_idx = 0usize;

        loop {
            let block = func.basic_blocks.get(block_idx)
                .ok_or_else(|| RavaError::Other("invalid block index".into()))?;

            let mut next_block: Option<usize> = None;
            let mut returned:   Option<RVal>  = None;

            for instr in &block.instrs {
                match instr {
                    RirInstr::ConstInt { ret, value } => {
                        env.insert(ret.0.clone(), RVal::Int(*value));
                    }
                    RirInstr::ConstFloat { ret, value } => {
                        env.insert(ret.0.clone(), RVal::Float(*value));
                    }
                    RirInstr::ConstStr { ret, value } => {
                        // Handle synthetic markers from the lowerer
                        if let Some(src_name) = value.strip_prefix("__copy__") {
                            let val = env.get(src_name).cloned().unwrap_or(RVal::Null);
                            env.insert(ret.0.clone(), val);
                        } else if let Some(resolved) = self.resolve_synthetic(value, &env) {
                            env.insert(ret.0.clone(), resolved);
                        } else {
                            env.insert(ret.0.clone(), RVal::Str(value.clone()));
                        }
                    }
                    RirInstr::ConstNull { ret } => {
                        env.insert(ret.0.clone(), RVal::Null);
                    }
                    RirInstr::BinOp { op, lhs, rhs, ret } => {
                        let l = self.resolve(&env, lhs);
                        let r = self.resolve(&env, rhs);
                        env.insert(ret.0.clone(), self.eval_binop(op, &l, &r));
                    }
                    RirInstr::UnaryOp { op, operand, ret } => {
                        let v = self.resolve(&env, operand);
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
                        let result = self.dispatch_call(func_id.0, arg_vals, &env)?;
                        if let Some(r) = ret { env.insert(r.0.clone(), result); }
                    }
                    RirInstr::New { class, ret } => {
                        let class_name = self.class_name_for(class.0);
                        // Special handling for known collection types
                        if class_name == "ArrayList" || class_name == "LinkedList" {
                            env.insert(ret.0.clone(), RVal::Array(Rc::new(RefCell::new(Vec::new()))));
                        } else {
                            let id = self.alloc_object(&class_name);
                            env.insert(ret.0.clone(), RVal::Object(id));
                        }
                    }
                    RirInstr::GetField { obj, field, ret } => {
                        let obj_val = self.resolve(&env, obj);
                        let val = self.get_field(&obj_val, field.0);
                        env.insert(ret.0.clone(), val);
                    }
                    RirInstr::SetField { obj, field, val } => {
                        let obj_val = self.resolve(&env, obj);
                        let v = self.resolve(&env, val);
                        self.set_field(&obj_val, field.0, v);
                    }
                    RirInstr::GetStatic { field, ret } => {
                        let key = format!("static#{}", field.0);
                        let val = self.static_fields.borrow().get(&key).cloned()
                            .unwrap_or(RVal::Null);
                        env.insert(ret.0.clone(), val);
                    }
                    RirInstr::SetStatic { field, val } => {
                        let key = format!("static#{}", field.0);
                        let v = self.resolve(&env, val);
                        self.static_fields.borrow_mut().insert(key, v);
                    }
                    RirInstr::NewArray { len, ret, .. } => {
                        let n = self.resolve(&env, len).as_int().max(0) as usize;
                        let arr = Rc::new(RefCell::new(vec![RVal::Int(0); n]));
                        env.insert(ret.0.clone(), RVal::Array(arr));
                    }
                    RirInstr::ArrayLoad { arr, idx, ret } => {
                        let arr_val = self.resolve(&env, arr);
                        let i = self.resolve(&env, idx).as_int() as usize;
                        let val = match &arr_val {
                            RVal::Array(a) => a.borrow().get(i).cloned().unwrap_or(RVal::Null),
                            _ => RVal::Null,
                        };
                        env.insert(ret.0.clone(), val);
                    }
                    RirInstr::ArrayStore { arr, idx, val } => {
                        let arr_val = self.resolve(&env, arr);
                        let i   = self.resolve(&env, idx).as_int() as usize;
                        let v   = self.resolve(&env, val);
                        if let RVal::Array(a) = &arr_val {
                            let mut borrow = a.borrow_mut();
                            // Auto-grow for ArrayList-style usage
                            while i >= borrow.len() { borrow.push(RVal::Null); }
                            borrow[i] = v;
                        }
                    }
                    RirInstr::ArrayLen { arr, ret } => {
                        let arr_val = self.resolve(&env, arr);
                        let len = match &arr_val {
                            RVal::Array(a) => a.borrow().len() as i64,
                            RVal::Str(s)   => s.len() as i64,
                            _ => 0,
                        };
                        env.insert(ret.0.clone(), RVal::Int(len));
                    }
                    RirInstr::Convert { val, to, ret, .. } => {
                        let v = self.resolve(&env, val);
                        let converted = match to {
                            rava_rir::RirType::I32 | rava_rir::RirType::I64 => RVal::Int(v.as_int()),
                            rava_rir::RirType::F32 | rava_rir::RirType::F64 => RVal::Float(v.as_float()),
                            rava_rir::RirType::Bool => RVal::Bool(v.is_truthy()),
                            _ => v,
                        };
                        env.insert(ret.0.clone(), converted);
                    }
                    RirInstr::Checkcast { obj, class } => {
                        let obj_val = self.resolve(&env, obj);
                        let class_name = self.class_name_for(class.0);
                        if let RVal::Object(id) = &obj_val {
                            let ok = self.heap.borrow().get(id)
                                .map(|o| o.class_name == class_name)
                                .unwrap_or(false);
                            if !ok {
                                return Err(RavaError::Other(
                                    format!("ClassCastException: cannot cast to {class_name}")
                                ));
                            }
                        }
                    }
                    RirInstr::Return(val) => {
                        returned = Some(match val {
                            Some(v) => self.resolve(&env, v),
                            None    => RVal::Void,
                        });
                        break;
                    }
                    RirInstr::Jump(target) => {
                        next_block = self.find_block_idx(func, target.0);
                        break;
                    }
                    RirInstr::Branch { cond, then_bb, else_bb } => {
                        let cv = self.resolve(&env, cond);
                        let target = if cv.is_truthy() { then_bb.0 } else { else_bb.0 };
                        next_block = self.find_block_idx(func, target);
                        break;
                    }
                    RirInstr::Throw(val) => {
                        let msg = self.resolve(&env, val).to_display();
                        return Err(RavaError::Other(format!("Java exception: {msg}")));
                    }
                    RirInstr::Instanceof { obj, class, ret } => {
                        let obj_val = self.resolve(&env, obj);
                        let class_name = self.class_name_for(class.0);
                        let result = match &obj_val {
                            RVal::Object(id) => {
                                self.heap.borrow().get(id)
                                    .map(|o| o.class_name == class_name)
                                    .unwrap_or(false)
                            }
                            RVal::Str(_) => class_name == "String",
                            RVal::Array(_) => class_name.ends_with("[]"),
                            _ => false,
                        };
                        env.insert(ret.0.clone(), RVal::Bool(result));
                    }
                    RirInstr::CallVirtual { receiver, method: _, args: arg_vals, ret } => {
                        let recv = self.resolve(&env, receiver);
                        let result = self.dispatch_virtual(recv, arg_vals, &env)?;
                        if let Some(r) = ret { env.insert(r.0.clone(), result); }
                    }
                    RirInstr::CallInterface { receiver, method: _, args: arg_vals, ret } => {
                        // Same as virtual dispatch for now
                        let recv = self.resolve(&env, receiver);
                        let result = self.dispatch_virtual(recv, arg_vals, &env)?;
                        if let Some(r) = ret { env.insert(r.0.clone(), result); }
                    }
                    RirInstr::Unreachable => {
                        return Err(RavaError::Other("reached unreachable code".into()));
                    }
                    RirInstr::MonitorEnter(_) | RirInstr::MonitorExit(_) => {
                        // Synchronization: no-op in single-threaded interpreter
                    }
                    RirInstr::MicroRtReflect { ret, .. } |
                    RirInstr::MicroRtProxy { ret, .. } |
                    RirInstr::MicroRtClassLoad { ret, .. } => {
                        env.insert(ret.0.clone(), RVal::Null);
                    }
                }
            }

            if let Some(rv) = returned { return Ok(rv); }
            match next_block {
                Some(idx) => block_idx = idx,
                None      => return Ok(RVal::Void),
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn find_block_idx(&self, func: &rava_rir::RirFunction, block_id: u32) -> Option<usize> {
        func.basic_blocks.iter().position(|b| b.id.0 == block_id)
    }

    fn resolve(&self, env: &HashMap<String, RVal>, val: &Value) -> RVal {
        env.get(&val.0).cloned().unwrap_or(RVal::Null)
    }

    /// Resolve synthetic `__field__<obj>#<name>` strings from the lowerer.
    fn resolve_synthetic(&self, s: &str, env: &HashMap<String, RVal>) -> Option<RVal> {
        let rest = s.strip_prefix("__field__")?;
        let (obj_key, field_name) = rest.split_once('#')?;
        let obj_val = env.get(obj_key)?.clone();
        Some(self.get_field_by_name(&obj_val, field_name))
    }

    fn get_field_by_name(&self, obj: &RVal, name: &str) -> RVal {
        // array.length / string.length
        if name == "length" {
            match obj {
                RVal::Array(a) => return RVal::Int(a.borrow().len() as i64),
                RVal::Str(s)   => return RVal::Int(s.len() as i64),
                _ => {}
            }
        }
        match obj {
            RVal::Object(id) => {
                self.heap.borrow().get(id)
                    .and_then(|o| o.fields.get(name).cloned())
                    .unwrap_or(RVal::Null)
            }
            _ => RVal::Null,
        }
    }

    fn get_field(&self, obj: &RVal, field_id: u32) -> RVal {
        let field_name = self.field_name_for(field_id);
        match obj {
            RVal::Object(id) => {
                self.heap.borrow().get(id)
                    .and_then(|o| o.fields.get(&field_name).cloned())
                    .unwrap_or(RVal::Null)
            }
            RVal::Array(a) => {
                if field_name == "length" {
                    RVal::Int(a.borrow().len() as i64)
                } else { RVal::Null }
            }
            RVal::Str(s) => {
                if field_name == "length" {
                    RVal::Int(s.len() as i64)
                } else { RVal::Null }
            }
            _ => RVal::Null,
        }
    }

    fn set_field(&self, obj: &RVal, field_id: u32, val: RVal) {
        if let RVal::Object(id) = obj {
            let field_name = self.field_name_for(field_id);
            if let Some(o) = self.heap.borrow_mut().get_mut(id) {
                o.fields.insert(field_name, val);
            }
        }
    }

    fn class_name_for(&self, class_id: u32) -> String {
        // Check common class names first
        for name in &["String", "Integer", "Long", "Double", "Float", "Boolean",
                       "ArrayList", "HashMap", "Object", "Exception",
                       "RuntimeException", "NullPointerException",
                       "StringBuilder", "System"] {
            if crate::lowerer_hash::encode_builtin(name) == class_id {
                return name.to_string();
            }
        }
        // Reverse-lookup from module functions
        for func in &self.module.functions {
            if let Some(class) = func.name.split('.').next() {
                if crate::lowerer_hash::encode_builtin(class) == class_id {
                    return class.to_string();
                }
            }
        }
        format!("Class@{class_id}")
    }

    fn dispatch_call(&self, func_id: u32, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();

        // Instance method call: __method__<name> with receiver as first arg
        // Try known builtin method names first, then user-defined methods
        if let Some(method_name) = self.resolve_method_name(func_id) {
            if let Some(receiver) = args.first() {
                let method_args = &args[1..];
                // Builtin string/array methods
                if let Some(result) = builtins::dispatch_named_method(receiver, &method_name, method_args) {
                    return result;
                }
                // Object-based builtins (StringBuilder, HashMap) and user-defined methods
                if let RVal::Object(id) = receiver {
                    let class_name = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    if class_name == "StringBuilder" {
                        if let Some(result) = self.dispatch_string_builder(*id, &method_name, method_args) {
                            return result;
                        }
                    }
                    if class_name == "HashMap" {
                        if let Some(result) = self.dispatch_hash_map(*id, &method_name, method_args) {
                            return result;
                        }
                    }
                    // User-defined instance method (with overload resolution by param count)
                    let full_name = format!("{}.{}", class_name, method_name);
                    let arg_count = method_args.len();
                    let func_idx = self.module.functions.iter()
                        .position(|f| f.name == full_name && f.params.len() == arg_count)
                        .or_else(|| self.module.functions.iter()
                            .position(|f| f.name == full_name));
                    if let Some(idx) = func_idx {
                        let func = &self.module.functions[idx];
                        let mut call_env: HashMap<String, RVal> = HashMap::new();
                        call_env.insert("this".into(), receiver.clone());
                        for ((param_name, _), val) in func.params.iter().zip(method_args.iter()) {
                            call_env.insert(param_name.0.clone(), val.clone());
                        }
                        return self.exec_function_idx(idx, call_env);
                    }
                }
            }
            return Ok(RVal::Void);
        }

        // Builtins (static)
        if let Some(result) = builtins::dispatch(func_id, &args) {
            return result;
        }

        // User-defined: match full name or short name
        // For overloaded methods (same name, different params), match by arg count
        let effective_arg_count = args.len();
        let func_idx = self.module.functions.iter().position(|f| {
            use crate::lowerer_hash::encode_builtin;
            let name_match = encode_builtin(&f.name) == func_id
                || f.name.rsplit('.').next()
                    .map(|s| encode_builtin(s) == func_id)
                    .unwrap_or(false);
            if !name_match { return false; }
            // For constructors, args include implicit `this`, params don't
            if f.flags.is_constructor {
                f.params.len() + 1 == effective_arg_count
            } else {
                f.params.len() == effective_arg_count
            }
        })
        // Fallback: if no exact param count match, try any name match
        .or_else(|| self.module.functions.iter().position(|f| {
            use crate::lowerer_hash::encode_builtin;
            encode_builtin(&f.name) == func_id
                || f.name.rsplit('.').next()
                    .map(|s| encode_builtin(s) == func_id)
                    .unwrap_or(false)
        }));

        if let Some(idx) = func_idx {
            let func = &self.module.functions[idx];
            let mut call_env: HashMap<String, RVal> = HashMap::new();
            // For constructors, first arg is `this` (implicit, not in params list)
            let effective_args = if func.flags.is_constructor && !args.is_empty() {
                call_env.insert("this".into(), args[0].clone());
                &args[1..]
            } else {
                &args[..]
            };
            for ((param_name, _), val) in func.params.iter().zip(effective_args.iter()) {
                call_env.insert(param_name.0.clone(), val.clone());
            }
            return self.exec_function_idx(idx, call_env);
        }

        Ok(RVal::Void)
    }

    fn dispatch_virtual(&self, receiver: RVal, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();
        if let Some(result) = builtins::dispatch_method(&receiver, &args) {
            return result;
        }
        // Try user-defined virtual method or builtin object methods
        if let RVal::Object(id) = &receiver {
            let class_name = self.heap.borrow().get(id)
                .map(|o| o.class_name.clone())
                .unwrap_or_default();
            // StringBuilder
            if class_name == "StringBuilder" {
                // Try to find method name from the first arg pattern
                // Virtual dispatch without method name — fallback to toString
                if let Some(result) = self.dispatch_string_builder(*id, "toString", &args) {
                    return result;
                }
            }
            // Try to find a method in the module (with overload resolution)
            let prefix = format!("{}.", class_name);
            let arg_count = args.len();
            let func_idx = self.module.functions.iter()
                .position(|f| f.name.starts_with(&prefix)
                    && !f.flags.is_constructor && !f.flags.is_clinit
                    && f.params.len() == arg_count)
                .or_else(|| self.module.functions.iter()
                    .position(|f| f.name.starts_with(&prefix)
                        && !f.flags.is_constructor && !f.flags.is_clinit));
            if let Some(idx) = func_idx {
                let func = &self.module.functions[idx];
                let mut call_env: HashMap<String, RVal> = HashMap::new();
                call_env.insert("this".into(), receiver.clone());
                for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                    call_env.insert(param_name.0.clone(), val.clone());
                }
                return self.exec_function_idx(idx, call_env);
            }
        }
        Ok(RVal::Void)
    }

    // ── StringBuilder methods ──────────────────────────────────────────────

    fn dispatch_string_builder(&self, id: ObjId, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        match method {
            "append" => {
                let val = args.first().map(|v| v.to_display()).unwrap_or_default();
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    let current = obj.fields.get("__buf__")
                        .map(|v| v.to_display())
                        .unwrap_or_default();
                    obj.fields.insert("__buf__".into(), RVal::Str(format!("{}{}", current, val)));
                }
                // Return `this` for chaining
                Some(Ok(RVal::Object(id)))
            }
            "toString" => {
                let heap = self.heap.borrow();
                let s = heap.get(&id)
                    .and_then(|o| o.fields.get("__buf__"))
                    .map(|v| v.to_display())
                    .unwrap_or_default();
                Some(Ok(RVal::Str(s)))
            }
            "length" => {
                let heap = self.heap.borrow();
                let len = heap.get(&id)
                    .and_then(|o| o.fields.get("__buf__"))
                    .map(|v| v.to_display().len() as i64)
                    .unwrap_or(0);
                Some(Ok(RVal::Int(len)))
            }
            "insert" => {
                let offset = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                let val = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    let mut current = obj.fields.get("__buf__")
                        .map(|v| v.to_display())
                        .unwrap_or_default();
                    let offset = offset.min(current.len());
                    current.insert_str(offset, &val);
                    obj.fields.insert("__buf__".into(), RVal::Str(current));
                }
                Some(Ok(RVal::Object(id)))
            }
            "reverse" => {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    let current = obj.fields.get("__buf__")
                        .map(|v| v.to_display())
                        .unwrap_or_default();
                    let reversed: String = current.chars().rev().collect();
                    obj.fields.insert("__buf__".into(), RVal::Str(reversed));
                }
                Some(Ok(RVal::Object(id)))
            }
            "deleteCharAt" => {
                let idx = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    let mut current = obj.fields.get("__buf__")
                        .map(|v| v.to_display())
                        .unwrap_or_default();
                    if idx < current.len() {
                        current.remove(idx);
                    }
                    obj.fields.insert("__buf__".into(), RVal::Str(current));
                }
                Some(Ok(RVal::Object(id)))
            }
            _ => None,
        }
    }

    // ── HashMap methods ──────────────────────────────────────────────────────

    fn dispatch_hash_map(&self, id: ObjId, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        match method {
            "put" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let val = args.get(1).cloned().unwrap_or(RVal::Null);
                let mut heap = self.heap.borrow_mut();
                let old = if let Some(obj) = heap.get_mut(&id) {
                    let old = obj.fields.get(&key).cloned().unwrap_or(RVal::Null);
                    obj.fields.insert(key, val);
                    old
                } else { RVal::Null };
                Some(Ok(old))
            }
            "get" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let heap = self.heap.borrow();
                let val = heap.get(&id)
                    .and_then(|o| o.fields.get(&key).cloned())
                    .unwrap_or(RVal::Null);
                Some(Ok(val))
            }
            "getOrDefault" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let default = args.get(1).cloned().unwrap_or(RVal::Null);
                let heap = self.heap.borrow();
                let val = heap.get(&id)
                    .and_then(|o| o.fields.get(&key).cloned())
                    .unwrap_or(default);
                Some(Ok(val))
            }
            "containsKey" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let heap = self.heap.borrow();
                let found = heap.get(&id)
                    .map(|o| o.fields.contains_key(&key))
                    .unwrap_or(false);
                Some(Ok(RVal::Bool(found)))
            }
            "containsValue" => {
                let target = args.first().map(|v| v.to_display()).unwrap_or_default();
                let heap = self.heap.borrow();
                let found = heap.get(&id)
                    .map(|o| o.fields.values().any(|v| v.to_display() == target))
                    .unwrap_or(false);
                Some(Ok(RVal::Bool(found)))
            }
            "remove" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let mut heap = self.heap.borrow_mut();
                let old = if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.remove(&key).unwrap_or(RVal::Null)
                } else { RVal::Null };
                Some(Ok(old))
            }
            "size" => {
                let heap = self.heap.borrow();
                // Exclude internal fields (starting with __)
                let size = heap.get(&id)
                    .map(|o| o.fields.keys().filter(|k| !k.starts_with("__")).count() as i64)
                    .unwrap_or(0);
                Some(Ok(RVal::Int(size)))
            }
            "isEmpty" => {
                let heap = self.heap.borrow();
                let empty = heap.get(&id)
                    .map(|o| o.fields.keys().filter(|k| !k.starts_with("__")).count() == 0)
                    .unwrap_or(true);
                Some(Ok(RVal::Bool(empty)))
            }
            "clear" => {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.clear();
                }
                Some(Ok(RVal::Void))
            }
            "keySet" | "values" | "entrySet" => {
                // Return keys/values as an array for iteration
                let heap = self.heap.borrow();
                let items: Vec<RVal> = if let Some(obj) = heap.get(&id) {
                    match method {
                        "keySet" => obj.fields.keys()
                            .filter(|k| !k.starts_with("__"))
                            .map(|k| RVal::Str(k.clone()))
                            .collect(),
                        "values" => obj.fields.iter()
                            .filter(|(k, _)| !k.starts_with("__"))
                            .map(|(_, v)| v.clone())
                            .collect(),
                        _ => vec![],
                    }
                } else { vec![] };
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "toString" => {
                let heap = self.heap.borrow();
                let s = if let Some(obj) = heap.get(&id) {
                    let entries: Vec<String> = obj.fields.iter()
                        .filter(|(k, _)| !k.starts_with("__"))
                        .map(|(k, v)| format!("{}={}", k, v.to_display()))
                        .collect();
                    format!("{{{}}}", entries.join(", "))
                } else { "{}".into() };
                Some(Ok(RVal::Str(s)))
            }
            _ => None,
        }
    }

    fn eval_binop(&self, op: &rava_rir::BinOp, l: &RVal, r: &RVal) -> RVal {
        use rava_rir::BinOp::*;

        // String concatenation: if either side is a string and op is Add
        if matches!(op, Add) {
            if matches!(l, RVal::Str(_)) || matches!(r, RVal::Str(_)) {
                return RVal::Str(format!("{}{}", l.to_display(), r.to_display()));
            }
        }

        // Float arithmetic: if either operand is float, use float math
        let use_float = l.is_float() || r.is_float();

        match op {
            Add => {
                if use_float { RVal::Float(l.as_float() + r.as_float()) }
                else { RVal::Int(l.as_int().wrapping_add(r.as_int())) }
            }
            Sub => {
                if use_float { RVal::Float(l.as_float() - r.as_float()) }
                else { RVal::Int(l.as_int().wrapping_sub(r.as_int())) }
            }
            Mul => {
                if use_float { RVal::Float(l.as_float() * r.as_float()) }
                else { RVal::Int(l.as_int().wrapping_mul(r.as_int())) }
            }
            Div => {
                if use_float {
                    let d = r.as_float();
                    if d == 0.0 { RVal::Float(f64::NAN) } else { RVal::Float(l.as_float() / d) }
                } else {
                    let d = r.as_int();
                    if d == 0 { RVal::Int(0) } else { RVal::Int(l.as_int() / d) }
                }
            }
            Rem => {
                if use_float {
                    let d = r.as_float();
                    if d == 0.0 { RVal::Float(f64::NAN) } else { RVal::Float(l.as_float() % d) }
                } else {
                    let d = r.as_int();
                    if d == 0 { RVal::Int(0) } else { RVal::Int(l.as_int() % d) }
                }
            }
            // Equality: handle strings and objects properly
            Eq => RVal::Bool(self.values_equal(l, r)),
            Ne => RVal::Bool(!self.values_equal(l, r)),
            Lt => {
                if use_float { RVal::Bool(l.as_float() < r.as_float()) }
                else { RVal::Bool(l.as_int() < r.as_int()) }
            }
            Le => {
                if use_float { RVal::Bool(l.as_float() <= r.as_float()) }
                else { RVal::Bool(l.as_int() <= r.as_int()) }
            }
            Gt => {
                if use_float { RVal::Bool(l.as_float() > r.as_float()) }
                else { RVal::Bool(l.as_int() > r.as_int()) }
            }
            Ge => {
                if use_float { RVal::Bool(l.as_float() >= r.as_float()) }
                else { RVal::Bool(l.as_int() >= r.as_int()) }
            }
            And => RVal::Bool(l.is_truthy() && r.is_truthy()),
            Or  => RVal::Bool(l.is_truthy() || r.is_truthy()),
            BitAnd => RVal::Int(l.as_int() & r.as_int()),
            BitOr  => RVal::Int(l.as_int() | r.as_int()),
            Xor => RVal::Int(l.as_int() ^ r.as_int()),
            Shl => RVal::Int(l.as_int() << (r.as_int() & 63)),
            Shr => RVal::Int(l.as_int() >> (r.as_int() & 63)),
            UShr => RVal::Int(((l.as_int() as u64) >> (r.as_int() & 63)) as i64),
        }
    }

    /// Compare two RVal for equality, handling strings and references correctly.
    fn values_equal(&self, l: &RVal, r: &RVal) -> bool {
        match (l, r) {
            (RVal::Int(a), RVal::Int(b))       => a == b,
            (RVal::Float(a), RVal::Float(b))   => a == b,
            (RVal::Int(a), RVal::Float(b))     => (*a as f64) == *b,
            (RVal::Float(a), RVal::Int(b))     => *a == (*b as f64),
            (RVal::Str(a), RVal::Str(b))       => a == b,
            (RVal::Bool(a), RVal::Bool(b))     => a == b,
            (RVal::Null, RVal::Null)           => true,
            (RVal::Object(a), RVal::Object(b)) => a == b,
            (RVal::Int(a), RVal::Bool(b))      => *a == (if *b { 1 } else { 0 }),
            (RVal::Bool(a), RVal::Int(b))      => (if *a { 1i64 } else { 0 }) == *b,
            _ => false,
        }
    }
}