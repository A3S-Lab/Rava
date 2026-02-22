//! RIR interpreter — executes RIR directly with full Java semantics.
//!
//! Supports: all arithmetic (int + float), string operations, object fields,
//! arrays, static fields, user-defined methods, break/continue (via control flow),
//! ternary (via branching), type conversion, instanceof, try/catch (simplified).

use std::collections::HashMap;
use std::cell::{Cell, RefCell};
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
    "getClass", "notify", "wait", "getMessage",
    // Map.Entry
    "getKey", "getValue",
    // Stream
    "stream", "map", "filter", "forEach", "collect", "reduce",
    "sorted", "count", "toList", "distinct", "limit", "skip",
    "findFirst", "anyMatch", "allMatch", "noneMatch",
    // Iterable / Iterator
    "of", "iterator", "hasNext", "next",
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
    /// Array iterator: (backing array, current index)
    ArrayIter(Rc<RefCell<Vec<RVal>>>, Rc<Cell<usize>>),
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
            RVal::ArrayIter(..) => true,
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
            RVal::ArrayIter(..) => "ArrayIterator".into(),
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
        // Run all <clinit> functions first (static initializers)
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
        // Exception handler stack: (catch_block_id, exception_types)
        let mut exception_handlers: Vec<(u32, Vec<String>)> = Vec::new();

        loop {
            let block = func.basic_blocks.get(block_idx)
                .ok_or_else(|| RavaError::Other("invalid block index".into()))?;

            let mut next_block: Option<usize> = None;
            let mut returned:   Option<RVal>  = None;
            let mut thrown: Option<(String, String)> = None;

            for instr in &block.instrs {
                // Execute instruction, catching JavaExceptions from calls
                let result = self.exec_instr(instr, &mut env, func, &mut next_block, &mut returned, &mut exception_handlers);
                match result {
                    Ok(()) => {}
                    Err(RavaError::JavaException { exception_type, message }) => {
                        thrown = Some((exception_type, message));
                        break;
                    }
                    Err(e) => return Err(e),
                }
                if next_block.is_some() || returned.is_some() { break; }
            }

            // Handle thrown exception
            if let Some((exc_type, exc_msg)) = thrown {
                // Find a matching handler by walking the stack from top
                let mut matched = None;
                while let Some((block_id, types)) = exception_handlers.pop() {
                    if types.is_empty() || types.iter().any(|t| self.exception_matches(&exc_type, t)) {
                        matched = Some(block_id);
                        break;
                    }
                }
                if let Some(catch_block_id) = matched {
                    // Create an exception object so catch variable can call getMessage() etc.
                    let exc_obj_id = self.alloc_object(&exc_type);
                    {
                        let mut heap = self.heap.borrow_mut();
                        if let Some(obj) = heap.get_mut(&exc_obj_id) {
                            obj.fields.insert("message".into(), RVal::Str(exc_msg.clone()));
                        }
                    }
                    env.insert("__exception__".into(), RVal::Object(exc_obj_id));
                    env.insert("__exception_type__".into(), RVal::Str(exc_type));
                    next_block = self.find_block_idx(func, catch_block_id);
                } else {
                    // No handler — propagate
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

    fn exec_instr(
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
                        // Handle synthetic markers from the lowerer
                        if let Some(rest) = value.strip_prefix("__try_catch__") {
                            // Format: __try_catch__<block_id>:<type1|type2>
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
                                // Legacy format without types — catch all
                                exception_handlers.push((id, vec![]));
                            }
                        } else if value == "__try_end__" {
                            exception_handlers.pop();
                        } else if value == "__exception__" {
                            // Resolve to the current exception value
                            let val = env.get("__exception__").cloned().unwrap_or(RVal::Null);
                            env.insert(ret.0.clone(), val);
                        } else if let Some(src_name) = value.strip_prefix("__copy__") {
                            let val = env.get(src_name).cloned().unwrap_or(RVal::Null);
                            env.insert(ret.0.clone(), val);
                        } else if let Some(resolved) = self.resolve_synthetic(value, env) {
                            env.insert(ret.0.clone(), resolved);
                        } else {
                            env.insert(ret.0.clone(), RVal::Str(value.clone()));
                        }
                    }
                    RirInstr::ConstNull { ret } => {
                        env.insert(ret.0.clone(), RVal::Null);
                    }
                    RirInstr::BinOp { op, lhs, rhs, ret } => {
                        let l = self.resolve(env, lhs);
                        let r = self.resolve(env, rhs);
                        env.insert(ret.0.clone(), self.eval_binop(op, &l, &r));
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
                        let key = format!("static#{}", field.0);
                        let val = self.static_fields.borrow().get(&key).cloned()
                            .unwrap_or(RVal::Null);
                        env.insert(ret.0.clone(), val);
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
                        let i = self.resolve(env, idx).as_int() as usize;
                        let val = match &arr_val {
                            RVal::Array(a) => a.borrow().get(i).cloned().unwrap_or(RVal::Null),
                            _ => RVal::Null,
                        };
                        env.insert(ret.0.clone(), val);
                    }
                    RirInstr::ArrayStore { arr, idx, val } => {
                        let arr_val = self.resolve(env, arr);
                        let i   = self.resolve(env, idx).as_int() as usize;
                        let v   = self.resolve(env, val);
                        if let RVal::Array(a) = &arr_val {
                            let mut borrow = a.borrow_mut();
                            // Auto-grow for ArrayList-style usage
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
                        let (exc_type, msg) = match &thrown_val {
                            RVal::Object(id) => {
                                let class_name = self.heap.borrow().get(id)
                                    .map(|o| o.class_name.clone())
                                    .unwrap_or_else(|| "Exception".into());
                                // Try to get the message from the object's constructor arg
                                // stored as field "message" or "__arg0__"
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
                            RVal::Str(_) => class_name == "String" || class_name == "Object",
                            RVal::Array(_) => class_name.ends_with("[]") || class_name == "Object",
                            _ => false,
                        };
                        env.insert(ret.0.clone(), RVal::Bool(result));
                    }
                    RirInstr::CallVirtual { receiver, method: _, args: arg_vals, ret } => {
                        let recv = self.resolve(env, receiver);
                        let result = self.dispatch_virtual(recv, arg_vals, env)?;
                        if let Some(r) = ret { env.insert(r.0.clone(), result); }
                    }
                    RirInstr::CallInterface { receiver, method: _, args: arg_vals, ret } => {
                        // Same as virtual dispatch for now
                        let recv = self.resolve(env, receiver);
                        let result = self.dispatch_virtual(recv, arg_vals, env)?;
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
            Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Find a method by walking the superclass chain and checking interfaces.
    fn find_method_in_chain(&self, class_name: &str, method_name: &str, effective_count: usize) -> Option<usize> {
        let mut current = class_name.to_string();
        loop {
            let full_name = format!("{}.{}", current, method_name);
            let idx = self.module.functions.iter()
                .position(|f| f.name == full_name && f.params.len() == effective_count)
                .or_else(|| self.module.functions.iter()
                    .position(|f| f.name == full_name));
            if idx.is_some() { return idx; }
            // Check implemented interfaces for default methods
            for (key, iface) in &self.module.class_hierarchy {
                if key.starts_with(&format!("{}:", current)) {
                    let iface_full = format!("{}.{}", iface, method_name);
                    let idx = self.module.functions.iter()
                        .position(|f| f.name == iface_full && f.params.len() == effective_count)
                        .or_else(|| self.module.functions.iter()
                            .position(|f| f.name == iface_full));
                    if idx.is_some() { return idx; }
                }
            }
            // Walk to superclass
            if let Some(parent) = self.module.class_hierarchy.get(&current) {
                current = parent.clone();
            } else {
                break;
            }
        }
        None
    }

    /// Check if a thrown exception type matches a catch clause type.
    fn exception_matches(&self, thrown: &str, catch_type: &str) -> bool {
        if catch_type == "Exception" || catch_type == "Throwable" { return true; }
        if thrown == catch_type { return true; }
        // Walk exception hierarchy
        // RuntimeException -> Exception -> Throwable
        // NullPointerException -> RuntimeException
        // IllegalArgumentException -> RuntimeException
        // etc.
        let builtin_parents: &[(&str, &str)] = &[
            ("RuntimeException", "Exception"),
            ("NullPointerException", "RuntimeException"),
            ("IllegalArgumentException", "RuntimeException"),
            ("IllegalStateException", "RuntimeException"),
            ("IndexOutOfBoundsException", "RuntimeException"),
            ("ArrayIndexOutOfBoundsException", "IndexOutOfBoundsException"),
            ("ClassCastException", "RuntimeException"),
            ("ArithmeticException", "RuntimeException"),
            ("UnsupportedOperationException", "RuntimeException"),
            ("IOException", "Exception"),
            ("FileNotFoundException", "IOException"),
            ("Error", "Throwable"),
            ("StackOverflowError", "Error"),
            ("OutOfMemoryError", "Error"),
        ];
        let mut current = thrown;
        loop {
            // Check user-defined hierarchy
            if let Some(parent) = self.module.class_hierarchy.get(current) {
                if parent == catch_type { return true; }
                current = parent;
                continue;
            }
            // Check builtin hierarchy
            if let Some((_, parent)) = builtin_parents.iter().find(|(c, _)| *c == current) {
                if *parent == catch_type { return true; }
                current = parent;
                continue;
            }
            break;
        }
        false
    }

    /// Check if `obj_class` is an instance of `target_class`, walking the inheritance chain.
    fn is_instance_of(&self, obj_class: &str, target_class: &str) -> bool {
        // Exact match
        if obj_class == target_class { return true; }
        // Everything is an Object
        if target_class == "Object" { return true; }
        // Check implemented interfaces: "ClassName:InterfaceName" entries
        let iface_key = format!("{}:{}", obj_class, target_class);
        if self.module.class_hierarchy.contains_key(&iface_key) { return true; }
        // Walk superclass chain
        if let Some(parent) = self.module.class_hierarchy.get(obj_class) {
            return self.is_instance_of(parent, target_class);
        }
        false
    }

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
        // Check the class_names registry first (populated by lowerer for all classes)
        if let Some(name) = self.module.class_names.get(&class_id) {
            return name.clone();
        }
        // Check common class names as fallback
        for name in &["String", "Integer", "Long", "Double", "Float", "Boolean",
                       "ArrayList", "HashMap", "Object", "Exception",
                       "RuntimeException", "NullPointerException",
                       "IllegalArgumentException", "IllegalStateException",
                       "IndexOutOfBoundsException", "ArrayIndexOutOfBoundsException",
                       "ClassCastException", "UnsupportedOperationException",
                       "ArithmeticException", "NumberFormatException",
                       "IOException", "FileNotFoundException",
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
                // Stream operations on arrays
                if let Some(result) = self.dispatch_stream(receiver, &method_name, method_args) {
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
                    // Walk superclass chain to find the method
                    let effective_count = method_args.len() + 1; // +1 for `this`
                    let func_idx = self.find_method_in_chain(&class_name, &method_name, effective_count);
                    if let Some(idx) = func_idx {
                        let func = &self.module.functions[idx];
                        let mut call_env: HashMap<String, RVal> = HashMap::new();
                        let mut all_args = vec![receiver.clone()];
                        all_args.extend(method_args.iter().cloned());
                        for ((param_name, _), val) in func.params.iter().zip(all_args.iter()) {
                            call_env.insert(param_name.0.clone(), val.clone());
                        }
                        return self.exec_function_idx(idx, call_env);
                    }
                    // Fallback: common object methods (getMessage, toString, getClass, getKey, getValue)
                    match method_name.as_str() {
                        "getMessage" => {
                            let msg = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("message").cloned())
                                .unwrap_or(RVal::Null);
                            return Ok(msg);
                        }
                        "toString" => {
                            let s = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("message").cloned())
                                .map(|v| format!("{}: {}", class_name, v.to_display()))
                                .unwrap_or_else(|| format!("{}@{}", class_name, id));
                            return Ok(RVal::Str(s));
                        }
                        "getClass" => {
                            return Ok(RVal::Str(class_name.clone()));
                        }
                        "getKey" => {
                            let key = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("__key__").cloned())
                                .unwrap_or(RVal::Null);
                            return Ok(key);
                        }
                        "getValue" => {
                            let val = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("__value__").cloned())
                                .unwrap_or(RVal::Null);
                            return Ok(val);
                        }
                        _ => {}
                    }
                }
                // Lambda / functional interface dispatch:
                // If receiver is a __lambda_N string, dispatch to that lambda function
                if let RVal::Str(ref s) = receiver {
                    if s.starts_with("__lambda_") {
                        return self.call_lambda_by_name(s, method_args);
                    }
                    // Method reference: __methodref__Class::method
                    if let Some(rest) = s.strip_prefix("__methodref__") {
                        if let Some((_cls, meth)) = rest.split_once("::") {
                            // Dispatch as a static call with method_args
                            let full = rest.replace("::", ".");
                            if let Some(idx) = self.module.functions.iter()
                                .position(|f| f.name == full)
                            {
                                let func = &self.module.functions[idx];
                                let mut call_env: HashMap<String, RVal> = HashMap::new();
                                for ((param_name, _), val) in func.params.iter().zip(method_args.iter()) {
                                    call_env.insert(param_name.0.clone(), val.clone());
                                }
                                return self.exec_function_idx(idx, call_env);
                            }
                            // Try as builtin
                            let key = format!("__method__{}", meth);
                            let hash = crate::lowerer_hash::encode_builtin(&key);
                            return self.dispatch_call(hash, arg_vals, env);
                        }
                    }
                }
            }
            return Ok(RVal::Void);
        }

        // Collections.sort with Comparator lambda (2 args: list, comparator)
        {
            use crate::lowerer_hash::encode_builtin;
            if func_id == encode_builtin("Collections.sort") && args.len() == 2 {
                if let RVal::Array(arr) = &args[0] {
                    let comparator = args[1].clone();
                    // Collect elements, sort with comparator, put back
                    let mut elems: Vec<RVal> = arr.borrow().clone();
                    // Use a simple insertion sort to avoid borrow issues with lambda calls
                    for i in 1..elems.len() {
                        let mut j = i;
                        while j > 0 {
                            let cmp_result = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                            if cmp_result.as_int() > 0 {
                                elems.swap(j - 1, j);
                                j -= 1;
                            } else {
                                break;
                            }
                        }
                    }
                    *arr.borrow_mut() = elems;
                    return Ok(RVal::Void);
                }
            }
        }

        // this(...) constructor delegation — find same-class constructor by arg count
        {
            use crate::lowerer_hash::encode_builtin;
            if func_id == encode_builtin("this.<init>") {
                if let Some(RVal::Object(id)) = args.first() {
                    let class_name = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    let ctor_name = format!("{}.<init>", class_name);
                    let arg_count = args.len();
                    if let Some(idx) = self.module.functions.iter()
                        .position(|f| f.name == ctor_name && f.params.len() == arg_count)
                    {
                        let func = &self.module.functions[idx];
                        let mut call_env: HashMap<String, RVal> = HashMap::new();
                        for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                            call_env.insert(param_name.0.clone(), val.clone());
                        }
                        return self.exec_function_idx(idx, call_env);
                    }
                }
                return Ok(RVal::Void);
            }
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
            // `this` is now included in params for constructors and instance methods
            f.params.len() == effective_arg_count
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
            // `this` is now a regular param — map all params directly
            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                call_env.insert(param_name.0.clone(), val.clone());
            }
            return self.exec_function_idx(idx, call_env);
        }

        // Lambda fallback: if first arg is a __lambda_N string, it's a functional
        // interface call where the method name wasn't in KNOWN_METHODS
        if let Some(first) = args.first() {
            if let RVal::Str(ref s) = first {
                if s.starts_with("__lambda_") {
                    let method_args = &args[1..];
                    return self.call_lambda_by_name(s, method_args);
                }
                if s.starts_with("__methodref__") {
                    let method_args = &args[1..];
                    if let Some(rest) = s.strip_prefix("__methodref__") {
                        let full = rest.replace("::", ".");
                        if let Some(idx) = self.module.functions.iter().position(|f| f.name == full) {
                            let func = &self.module.functions[idx];
                            let mut call_env: HashMap<String, RVal> = HashMap::new();
                            for ((param_name, _), val) in func.params.iter().zip(method_args.iter()) {
                                call_env.insert(param_name.0.clone(), val.clone());
                            }
                            return self.exec_function_idx(idx, call_env);
                        }
                    }
                }
            }
        }

        // Exception/unknown constructor fallback: store args as fields on the object
        // This handles `new RuntimeException("msg")` where no user-defined <init> exists
        if let Some(first) = args.first() {
            if let RVal::Object(id) = first {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(id) {
                    for (i, arg) in args.iter().skip(1).enumerate() {
                        obj.fields.insert(format!("__arg{}__", i), arg.clone());
                    }
                    // Also store first string arg as "message" for exception getMessage()
                    if let Some(msg_arg) = args.get(1) {
                        obj.fields.insert("message".into(), msg_arg.clone());
                    }
                }
            }
        }

        Ok(RVal::Void)
    }

    fn dispatch_virtual(&self, receiver: RVal, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();
        // Lambda / functional interface: receiver is a __lambda_N string
        if let RVal::Str(ref s) = receiver {
            if s.starts_with("__lambda_") {
                return self.call_lambda_by_name(s, &args);
            }
            if let Some(rest) = s.strip_prefix("__methodref__") {
                let full = rest.replace("::", ".");
                if let Some(idx) = self.module.functions.iter().position(|f| f.name == full) {
                    let func = &self.module.functions[idx];
                    let mut call_env: HashMap<String, RVal> = HashMap::new();
                    for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                        call_env.insert(param_name.0.clone(), val.clone());
                    }
                    return self.exec_function_idx(idx, call_env);
                }
            }
        }
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
            // Try to find a method in the module, walking superclass chain
            let effective_count = args.len() + 1; // +1 for `this`
            // Virtual dispatch: try to find any non-constructor method on this class or parents
            let func_idx = {
                let mut found = None;
                let mut cur = class_name.clone();
                loop {
                    let prefix = format!("{}.", cur);
                    let idx = self.module.functions.iter()
                        .position(|f| f.name.starts_with(&prefix)
                            && !f.flags.is_constructor && !f.flags.is_clinit
                            && f.params.len() == effective_count)
                        .or_else(|| self.module.functions.iter()
                            .position(|f| f.name.starts_with(&prefix)
                                && !f.flags.is_constructor && !f.flags.is_clinit));
                    if idx.is_some() { found = idx; break; }
                    // Check interfaces
                    for (key, iface) in &self.module.class_hierarchy {
                        if key.starts_with(&format!("{}:", cur)) {
                            let iface_prefix = format!("{}.", iface);
                            let idx = self.module.functions.iter()
                                .position(|f| f.name.starts_with(&iface_prefix)
                                    && !f.flags.is_constructor && !f.flags.is_clinit
                                    && f.params.len() == effective_count)
                                .or_else(|| self.module.functions.iter()
                                    .position(|f| f.name.starts_with(&iface_prefix)
                                        && !f.flags.is_constructor && !f.flags.is_clinit));
                            if idx.is_some() { found = idx; break; }
                        }
                    }
                    if found.is_some() { break; }
                    if let Some(parent) = self.module.class_hierarchy.get(&cur) {
                        cur = parent.clone();
                    } else { break; }
                }
                found
            };
            if let Some(idx) = func_idx {
                let func = &self.module.functions[idx];
                let mut call_env: HashMap<String, RVal> = HashMap::new();
                // Prepend receiver as `this`, then map remaining params to args
                let mut all_args = vec![receiver.clone()];
                all_args.extend(args.iter().cloned());
                for ((param_name, _), val) in func.params.iter().zip(all_args.iter()) {
                    call_env.insert(param_name.0.clone(), val.clone());
                }
                return self.exec_function_idx(idx, call_env);
            }
        }
        Ok(RVal::Void)
    }

    /// Dispatch a call to a lambda function by its `__lambda_N` name.
    fn call_lambda_by_name(&self, name: &str, args: &[RVal]) -> Result<RVal> {
        if let Some(idx) = self.module.functions.iter().position(|f| f.name == name) {
            let func = &self.module.functions[idx];
            let mut call_env: HashMap<String, RVal> = HashMap::new();
            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                call_env.insert(param_name.0.clone(), val.clone());
            }
            return self.exec_function_idx(idx, call_env);
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
                if method == "entrySet" {
                    // Collect pairs first, then create entry objects
                    let pairs: Vec<(String, RVal)> = {
                        let heap = self.heap.borrow();
                        if let Some(obj) = heap.get(&id) {
                            obj.fields.iter()
                                .filter(|(k, _)| !k.starts_with("__"))
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        } else { vec![] }
                    };
                    let entries: Vec<RVal> = pairs.into_iter().map(|(k, v)| {
                        let eid = self.alloc_object("Map.Entry");
                        {
                            let mut heap = self.heap.borrow_mut();
                            if let Some(entry) = heap.get_mut(&eid) {
                                entry.fields.insert("__key__".into(), RVal::Str(k));
                                entry.fields.insert("__value__".into(), v);
                            }
                        }
                        RVal::Object(eid)
                    }).collect();
                    return Some(Ok(RVal::Array(Rc::new(RefCell::new(entries)))));
                }
                // keySet / values
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

    /// Dispatch stream operations on arrays (and ArrayList which are arrays internally).
    fn dispatch_stream(&self, receiver: &RVal, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        match method {
            "stream" | "toList" => {
                // stream() returns the array itself (eager evaluation)
                // toList() also returns the array as-is
                Some(Ok(receiver.clone()))
            }
            "of" => {
                // List.of(a, b, c) — args are the elements
                Some(Ok(RVal::Array(Rc::new(RefCell::new(args.to_vec())))))
            }
            "map" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                let mut result = Vec::with_capacity(items.len());
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => result.push(v),
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
            }
            "filter" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                let mut result = Vec::new();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => { if v.is_truthy() { result.push(item.clone()); } }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
            }
            "forEach" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow().clone();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(_) => {}
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Void))
            }
            "collect" => {
                // collect() just returns the array (already materialized)
                Some(Ok(receiver.clone()))
            }
            "reduce" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                if items.is_empty() { return Some(Ok(RVal::Null)); }
                if args.len() >= 2 {
                    // reduce(identity, accumulator)
                    let mut acc = args[0].clone();
                    let lambda = &args[1];
                    for item in items.iter() {
                        match self.invoke_lambda(lambda, &[acc.clone(), item.clone()]) {
                            Ok(v) => acc = v,
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    Some(Ok(acc))
                } else {
                    // reduce(accumulator) — no identity, use first element
                    let lambda = &args[0];
                    let mut acc = items[0].clone();
                    for item in items.iter().skip(1) {
                        match self.invoke_lambda(lambda, &[acc.clone(), item.clone()]) {
                            Ok(v) => acc = v,
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    Some(Ok(acc))
                }
            }
            "sorted" => {
                let arr = self.as_array(receiver)?;
                let mut items = arr.borrow().clone();
                items.sort_by(|a, b| a.as_int().cmp(&b.as_int()));
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "count" => {
                let arr = self.as_array(receiver)?;
                Some(Ok(RVal::Int(arr.borrow().len() as i64)))
            }
            "distinct" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                let mut seen = Vec::new();
                let mut result = Vec::new();
                for item in items.iter() {
                    let key = item.to_display();
                    if !seen.contains(&key) {
                        seen.push(key);
                        result.push(item.clone());
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
            }
            "limit" => {
                let arr = self.as_array(receiver)?;
                let n = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                let items: Vec<RVal> = arr.borrow().iter().take(n).cloned().collect();
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "skip" => {
                let arr = self.as_array(receiver)?;
                let n = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                let items: Vec<RVal> = arr.borrow().iter().skip(n).cloned().collect();
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "findFirst" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                Some(Ok(items.first().cloned().unwrap_or(RVal::Null)))
            }
            "anyMatch" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => { if v.is_truthy() { return Some(Ok(RVal::Bool(true))); } }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Bool(false)))
            }
            "allMatch" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => { if !v.is_truthy() { return Some(Ok(RVal::Bool(false))); } }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Bool(true)))
            }
            "noneMatch" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => { if v.is_truthy() { return Some(Ok(RVal::Bool(false))); } }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Bool(true)))
            }
            _ => None,
        }
    }

    /// Extract the inner Rc<RefCell<Vec<RVal>>> from an Array value.
    fn as_array<'b>(&self, val: &'b RVal) -> Option<&'b Rc<RefCell<Vec<RVal>>>> {
        if let RVal::Array(arr) = val { Some(arr) } else { None }
    }

    /// Recursively create a multi-dimensional array.
    fn create_multi_array(&self, dims: &[usize], depth: usize) -> RVal {
        let size = dims[depth];
        if depth == dims.len() - 1 {
            // Innermost dimension: array of zeros
            RVal::Array(Rc::new(RefCell::new(vec![RVal::Int(0); size])))
        } else {
            // Outer dimension: array of sub-arrays
            let elements: Vec<RVal> = (0..size)
                .map(|_| self.create_multi_array(dims, depth + 1))
                .collect();
            RVal::Array(Rc::new(RefCell::new(elements)))
        }
    }

    /// Invoke a lambda or method reference with the given arguments.
    fn invoke_lambda(&self, lambda: &RVal, args: &[RVal]) -> Result<RVal> {
        match lambda {
            RVal::Str(s) if s.starts_with("__lambda_") => {
                self.call_lambda_by_name(s, args)
            }
            RVal::Str(s) if s.starts_with("__methodref__") => {
                let rest = s.strip_prefix("__methodref__").unwrap();
                let full = rest.replace("::", ".");
                if let Some(idx) = self.module.functions.iter().position(|f| f.name == full) {
                    let func = &self.module.functions[idx];
                    let mut call_env: HashMap<String, RVal> = HashMap::new();
                    for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                        call_env.insert(param_name.0.clone(), val.clone());
                    }
                    self.exec_function_idx(idx, call_env)
                } else {
                    Ok(RVal::Null)
                }
            }
            _ => Ok(RVal::Null),
        }
    }

    /// Convert a value to string, using object's toString/message for Objects.
    fn obj_to_string(&self, val: &RVal) -> String {
        if let RVal::Object(id) = val {
            let heap = self.heap.borrow();
            if let Some(obj) = heap.get(id) {
                // Try message field first (exceptions), then __name__ (enums)
                if let Some(msg) = obj.fields.get("message") {
                    return msg.to_display();
                }
                if let Some(name) = obj.fields.get("__name__") {
                    return name.to_display();
                }
                return format!("{}@{}", obj.class_name, id);
            }
        }
        val.to_display()
    }

    fn eval_binop(&self, op: &rava_rir::BinOp, l: &RVal, r: &RVal) -> RVal {
        use rava_rir::BinOp::*;

        // String concatenation: if either side is a string and op is Add
        if matches!(op, Add) {
            if matches!(l, RVal::Str(_)) || matches!(r, RVal::Str(_)) {
                let ls = self.obj_to_string(&l);
                let rs = self.obj_to_string(&r);
                return RVal::Str(format!("{}{}", ls, rs));
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