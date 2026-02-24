//! Helper methods on RirInterpreter: field access, class/method resolution,
//! dispatch_call, dispatch_virtual, lambda invocation, eval_binop.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use rava_common::error::{RavaError, Result};
use rava_rir::Value;
use crate::builtins;
use super::rval::{ObjId, RVal};
use super::RirInterpreter;

impl RirInterpreter {
    // ── Object allocation ─────────────────────────────────────────────────────

    pub(super) fn alloc_object(&self, class_name: &str) -> ObjId {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        self.heap.borrow_mut().insert(id, super::rval::JavaObject {
            class_name: class_name.to_string(),
            fields: HashMap::new(),
        });
        id
    }

    // ── Name resolution ───────────────────────────────────────────────────────

    pub(super) fn field_name_for(&self, field_id: u32) -> String {
        if let Some(name) = self.module.field_names.get(&field_id) {
            return name.clone();
        }
        for name in &["length", "size", "value", "name", "x", "y", "z",
                       "width", "height", "key", "data", "next", "prev",
                       "left", "right", "parent", "count", "index", "head", "tail"] {
            if crate::lowerer_hash::encode_builtin(name) == field_id {
                return name.to_string();
            }
        }
        format!("__field_{field_id}")
    }

    pub(super) fn resolve_method_name(&self, func_id: u32) -> Option<String> {
        // Fast path: check the module's method_names table (populated by Lowerer)
        if let Some(name) = self.module.method_names.get(&func_id) {
            return Some(name.clone());
        }
        // Fallback: scan known stdlib method names
        for method_name in super::KNOWN_METHODS {
            let key = format!("__method__{}", method_name);
            if crate::lowerer_hash::encode_builtin(&key) == func_id {
                return Some(method_name.to_string());
            }
        }
        // Fallback: scan user-defined function names
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

    pub(super) fn class_name_for(&self, class_id: u32) -> String {
        if let Some(name) = self.module.class_names.get(&class_id) {
            return name.clone();
        }
        for name in &["String", "Integer", "Long", "Double", "Float", "Boolean",
                       "ArrayList", "HashMap", "TreeMap", "LinkedHashMap",
                       "PriorityQueue", "LinkedList", "HashSet", "TreeSet",
                       "Object", "Exception",
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
        for func in &self.module.functions {
            if let Some(class) = func.name.split('.').next() {
                if crate::lowerer_hash::encode_builtin(class) == class_id {
                    return class.to_string();
                }
            }
        }
        format!("Class@{class_id}")
    }

    // ── Inheritance / exception helpers ───────────────────────────────────────

    pub(super) fn find_method_in_chain(&self, class_name: &str, method_name: &str, effective_count: usize) -> Option<usize> {
        let mut current = class_name.to_string();
        loop {
            let full_name = format!("{}.{}", current, method_name);
            let idx = self.module.functions.iter()
                .position(|f| f.name == full_name && f.params.len() == effective_count)
                .or_else(|| self.module.functions.iter()
                    .position(|f| f.name == full_name));
            if idx.is_some() { return idx; }
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
            if let Some(parent) = self.module.class_hierarchy.get(&current) {
                current = parent.clone();
            } else { break; }
        }
        None
    }

    pub(super) fn exception_matches(&self, thrown: &str, catch_type: &str) -> bool {
        if catch_type == "Exception" || catch_type == "Throwable" { return true; }
        if thrown == catch_type { return true; }
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
            if let Some(parent) = self.module.class_hierarchy.get(current) {
                if parent == catch_type { return true; }
                current = parent;
                continue;
            }
            if let Some((_, parent)) = builtin_parents.iter().find(|(c, _)| *c == current) {
                if *parent == catch_type { return true; }
                current = parent;
                continue;
            }
            break;
        }
        false
    }

    pub(super) fn is_instance_of(&self, obj_class: &str, target_class: &str) -> bool {
        if obj_class == target_class { return true; }
        if target_class == "Object" { return true; }
        let iface_key = format!("{}:{}", obj_class, target_class);
        if self.module.class_hierarchy.contains_key(&iface_key) { return true; }
        if let Some(parent) = self.module.class_hierarchy.get(obj_class) {
            return self.is_instance_of(parent, target_class);
        }
        false
    }

    pub(super) fn find_block_idx(&self, func: &rava_rir::RirFunction, block_id: u32) -> Option<usize> {
        func.basic_blocks.iter().position(|b| b.id.0 == block_id)
    }

    // ── Value resolution ──────────────────────────────────────────────────────

    pub(super) fn resolve(&self, env: &HashMap<String, RVal>, val: &Value) -> RVal {
        env.get(&val.0).cloned().unwrap_or(RVal::Null)
    }

    pub(super) fn resolve_synthetic(&self, s: &str, env: &HashMap<String, RVal>) -> Option<RVal> {
        let rest = s.strip_prefix("__field__")?;
        let (obj_key, field_name) = rest.split_once('#')?;
        let obj_val = env.get(obj_key)?.clone();
        Some(self.get_field_by_name(&obj_val, field_name))
    }

    pub(super) fn get_field_by_name(&self, obj: &RVal, name: &str) -> RVal {
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

    pub(super) fn get_field(&self, obj: &RVal, field_id: u32) -> RVal {
        let field_name = self.field_name_for(field_id);
        match obj {
            RVal::Object(id) => {
                self.heap.borrow().get(id)
                    .and_then(|o| o.fields.get(&field_name).cloned())
                    .unwrap_or(RVal::Null)
            }
            RVal::Array(a) => {
                if field_name == "length" { RVal::Int(a.borrow().len() as i64) }
                else { RVal::Null }
            }
            RVal::Str(s) => {
                if field_name == "length" { RVal::Int(s.len() as i64) }
                else { RVal::Null }
            }
            _ => RVal::Null,
        }
    }

    pub(super) fn set_field(&self, obj: &RVal, field_id: u32, val: RVal) {
        if let RVal::Object(id) = obj {
            let field_name = self.field_name_for(field_id);
            if let Some(o) = self.heap.borrow_mut().get_mut(id) {
                o.fields.insert(field_name, val);
            }
        }
    }

    // ── Multi-dim array ───────────────────────────────────────────────────────

    pub(super) fn create_multi_array(&self, dims: &[usize], depth: usize) -> RVal {
        let size = dims[depth];
        if depth == dims.len() - 1 {
            RVal::Array(Rc::new(RefCell::new(vec![RVal::Int(0); size])))
        } else {
            let elements: Vec<RVal> = (0..size)
                .map(|_| self.create_multi_array(dims, depth + 1))
                .collect();
            RVal::Array(Rc::new(RefCell::new(elements)))
        }
    }

    // ── dispatch_call ─────────────────────────────────────────────────────────

    pub(super) fn dispatch_call(&self, func_id: u32, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();

        // Handle println/print here so we can use obj_to_string for enum/object display
        {
            use crate::lowerer_hash::encode_builtin;
            if func_id == encode_builtin("System.out.println") {
                let s = args.first().map(|v| self.obj_to_string(v)).unwrap_or_default();
                super::write_output(&s);
                return Ok(RVal::Void);
            }
            if func_id == encode_builtin("System.out.print") {
                let s = args.first().map(|v| self.obj_to_string(v)).unwrap_or_default();
                super::write_output_no_nl(&s);
                return Ok(RVal::Void);
            }
        }

        if let Some(method_name) = self.resolve_method_name(func_id) {
            if let Some(receiver) = args.first() {
                let method_args = &args[1..];
                if let Some(result) = builtins::dispatch_named_method(receiver, &method_name, method_args) {
                    return result;
                }
                if let Some(result) = self.dispatch_stream(receiver, &method_name, method_args) {
                    return result;
                }
                if let RVal::Object(id) = receiver {
                    let class_name = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    if class_name == "StringBuilder" {
                        if let Some(result) = self.dispatch_string_builder(*id, &method_name, method_args) {
                            return result;
                        }
                    }
                    if class_name == "HashMap" || class_name == "TreeMap" || class_name == "LinkedHashMap" {
                        if let Some(result) = self.dispatch_hash_map(*id, &method_name, method_args) {
                            // For TreeMap, sort keySet/entrySet results
                            if class_name == "TreeMap" && (method_name == "keySet" || method_name == "values" || method_name == "entrySet") {
                                if let Ok(RVal::Array(arr)) = &result {
                                    arr.borrow_mut().sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                                }
                            }
                            return result;
                        }
                    }
                    if class_name == "PriorityQueue" {
                        if let Some(result) = self.dispatch_priority_queue(*id, &method_name, method_args) {
                            return result;
                        }
                    }
                    let effective_count = method_args.len() + 1;
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
                    match method_name.as_str() {
                        "getMessage" => {
                            let msg = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("message").cloned())
                                .unwrap_or(RVal::Null);
                            return Ok(msg);
                        }
                        "toString" => {
                            let s = self.heap.borrow().get(id)
                                .map(|o| {
                                    if let Some(name) = o.fields.get("_name") {
                                        return name.to_display();
                                    }
                                    if let Some(msg) = o.fields.get("message") {
                                        return format!("{}: {}", o.class_name, msg.to_display());
                                    }
                                    format!("{}@{}", o.class_name, id)
                                })
                                .unwrap_or_else(|| format!("Object@{}", id));
                            return Ok(RVal::Str(s));
                        }
                        "getClass" => return Ok(RVal::Str(class_name.clone())),
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
                if let RVal::Str(ref s) = receiver {
                    if s.starts_with("__lambda_") {
                        return self.call_lambda_by_name(s, method_args);
                    }
                    if let Some(rest) = s.strip_prefix("__methodref__") {
                        if let Some((_cls, meth)) = rest.split_once("::") {
                            let full = rest.replace("::", ".");
                            if let Some(idx) = self.module.functions.iter().position(|f| f.name == full) {
                                let func = &self.module.functions[idx];
                                let mut call_env: HashMap<String, RVal> = HashMap::new();
                                for ((param_name, _), val) in func.params.iter().zip(method_args.iter()) {
                                    call_env.insert(param_name.0.clone(), val.clone());
                                }
                                return self.exec_function_idx(idx, call_env);
                            }
                            let key = format!("__method__{}", meth);
                            let hash = crate::lowerer_hash::encode_builtin(&key);
                            return self.dispatch_call(hash, arg_vals, env);
                        }
                    }
                }
            }
            return Ok(RVal::Void);
        }

        // Collections.sort with Comparator lambda
        {
            use crate::lowerer_hash::encode_builtin;
            if func_id == encode_builtin("Collections.sort") && args.len() == 2 {
                if let RVal::Array(arr) = &args[0] {
                    let comparator = args[1].clone();
                    let mut elems: Vec<RVal> = arr.borrow().clone();
                    for i in 1..elems.len() {
                        let mut j = i;
                        while j > 0 {
                            let cmp_result = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                            if cmp_result.as_int() > 0 {
                                elems.swap(j - 1, j);
                                j -= 1;
                            } else { break; }
                        }
                    }
                    *arr.borrow_mut() = elems;
                    return Ok(RVal::Void);
                }
            }
        }

        // this(...) constructor delegation
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

            // super(...) constructor delegation — find parent class constructor
            if func_id == encode_builtin("super.<init>") {
                if let Some(RVal::Object(id)) = args.first() {
                    let class_name = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    let parent = self.module.class_hierarchy.get(&class_name).cloned();
                    if let Some(parent_class) = parent {
                        let ctor_name = format!("{}.<init>", parent_class);
                        let arg_count = args.len();
                        let idx = self.module.functions.iter()
                            .position(|f| f.name == ctor_name && f.params.len() == arg_count)
                            .or_else(|| self.module.functions.iter()
                                .position(|f| f.name == ctor_name));
                        if let Some(idx) = idx {
                            let func = &self.module.functions[idx];
                            let mut call_env: HashMap<String, RVal> = HashMap::new();
                            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                                call_env.insert(param_name.0.clone(), val.clone());
                            }
                            return self.exec_function_idx(idx, call_env);
                        }
                    }
                }
                return Ok(RVal::Void);
            }
        }

        if let Some(result) = builtins::dispatch(func_id, &args) {
            return result;
        }

        let effective_arg_count = args.len();
        let func_idx = self.module.functions.iter().position(|f| {
            use crate::lowerer_hash::encode_builtin;
            let name_match = encode_builtin(&f.name) == func_id
                || f.name.rsplit('.').next()
                    .map(|s| encode_builtin(s) == func_id)
                    .unwrap_or(false);
            if !name_match { return false; }
            f.params.len() == effective_arg_count
        })
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
            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                call_env.insert(param_name.0.clone(), val.clone());
            }
            return self.exec_function_idx(idx, call_env);
        }

        if let Some(first) = args.first() {
            if let RVal::Str(ref s) = first {
                if s.starts_with("__lambda_") {
                    return self.call_lambda_by_name(s, &args[1..]);
                }
                if s.starts_with("__methodref__") {
                    if let Some(rest) = s.strip_prefix("__methodref__") {
                        let full = rest.replace("::", ".");
                        if let Some(idx) = self.module.functions.iter().position(|f| f.name == full) {
                            let func = &self.module.functions[idx];
                            let mut call_env: HashMap<String, RVal> = HashMap::new();
                            for ((param_name, _), val) in func.params.iter().zip(args[1..].iter()) {
                                call_env.insert(param_name.0.clone(), val.clone());
                            }
                            return self.exec_function_idx(idx, call_env);
                        }
                    }
                }
            }
        }

        // Exception/unknown constructor fallback
        if let Some(first) = args.first() {
            if let RVal::Object(id) = first {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(id) {
                    for (i, arg) in args.iter().skip(1).enumerate() {
                        obj.fields.insert(format!("__arg{}__", i), arg.clone());
                    }
                    if let Some(msg_arg) = args.get(1) {
                        obj.fields.insert("message".into(), msg_arg.clone());
                    }
                }
            }
        }

        Ok(RVal::Void)
    }

    // ── dispatch_virtual ──────────────────────────────────────────────────────

    pub(super) fn dispatch_virtual(&self, receiver: RVal, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();
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
        if let RVal::Object(id) = &receiver {
            let class_name = self.heap.borrow().get(id)
                .map(|o| o.class_name.clone())
                .unwrap_or_default();
            if class_name == "StringBuilder" {
                if let Some(result) = self.dispatch_string_builder(*id, "toString", &args) {
                    return result;
                }
            }
            let effective_count = args.len() + 1;
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

    // ── Lambda helpers ────────────────────────────────────────────────────────

    pub(super) fn call_lambda_by_name(&self, name: &str, args: &[RVal]) -> Result<RVal> {
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

    pub(super) fn invoke_lambda(&self, lambda: &RVal, args: &[RVal]) -> Result<RVal> {
        match lambda {
            RVal::Str(s) if s.starts_with("__lambda_") => self.call_lambda_by_name(s, args),
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
                    // Stdlib method ref — dispatch as named method on first arg
                    if let Some((_cls, meth)) = rest.split_once("::") {
                        if let Some(receiver) = args.first() {
                            let method_args = &args[1..];
                            if let Some(result) = crate::builtins::dispatch_named_method(receiver, meth, method_args) {
                                return result;
                            }
                            // Identity-like methods: intValue, doubleValue, longValue, etc.
                            match meth {
                                "intValue" | "longValue" | "shortValue" | "byteValue" => {
                                    return Ok(RVal::Int(receiver.as_int()));
                                }
                                "doubleValue" | "floatValue" => {
                                    return Ok(RVal::Float(receiver.as_float()));
                                }
                                "booleanValue" => {
                                    return Ok(RVal::Bool(receiver.is_truthy()));
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(RVal::Null)
                }
            }
            _ => Ok(RVal::Null),
        }
    }

    // ── eval_binop + values_equal ─────────────────────────────────────────────

    pub(super) fn obj_to_string(&self, val: &RVal) -> String {
        if let RVal::Object(id) = val {
            let heap = self.heap.borrow();
            if let Some(obj) = heap.get(id) {
                if let Some(msg) = obj.fields.get("message") { return msg.to_display(); }
                if let Some(name) = obj.fields.get("__name__") { return name.to_display(); }
                if let Some(name) = obj.fields.get("_name") { return name.to_display(); }
                return format!("{}@{}", obj.class_name, id);
            }
        }
        val.to_display()
    }

    pub(super) fn eval_binop(&self, op: &rava_rir::BinOp, l: &RVal, r: &RVal) -> Result<RVal> {
        use rava_rir::BinOp::*;
        if matches!(op, Add) {
            if matches!(l, RVal::Str(_)) || matches!(r, RVal::Str(_)) {
                return Ok(RVal::Str(format!("{}{}", self.obj_to_string(l), self.obj_to_string(r))));
            }
        }
        let use_float = l.is_float() || r.is_float();
        Ok(match op {
            Add    => if use_float { RVal::Float(l.as_float() + r.as_float()) } else { RVal::Int(l.as_int().wrapping_add(r.as_int())) },
            Sub    => if use_float { RVal::Float(l.as_float() - r.as_float()) } else { RVal::Int(l.as_int().wrapping_sub(r.as_int())) },
            Mul    => if use_float { RVal::Float(l.as_float() * r.as_float()) } else { RVal::Int(l.as_int().wrapping_mul(r.as_int())) },
            Div    => if use_float {
                let d = r.as_float(); if d == 0.0 { RVal::Float(f64::NAN) } else { RVal::Float(l.as_float() / d) }
            } else {
                let d = r.as_int();
                if d == 0 {
                    return Err(RavaError::JavaException {
                        exception_type: "ArithmeticException".into(),
                        message: "/ by zero".into(),
                    });
                }
                RVal::Int(l.as_int() / d)
            },
            Rem    => if use_float {
                let d = r.as_float(); if d == 0.0 { RVal::Float(f64::NAN) } else { RVal::Float(l.as_float() % d) }
            } else {
                let d = r.as_int();
                if d == 0 {
                    return Err(RavaError::JavaException {
                        exception_type: "ArithmeticException".into(),
                        message: "/ by zero".into(),
                    });
                }
                RVal::Int(l.as_int() % d)
            },
            Eq     => RVal::Bool(self.values_equal(l, r)),
            Ne     => RVal::Bool(!self.values_equal(l, r)),
            Lt     => if use_float { RVal::Bool(l.as_float() < r.as_float()) } else { RVal::Bool(l.as_int() < r.as_int()) },
            Le     => if use_float { RVal::Bool(l.as_float() <= r.as_float()) } else { RVal::Bool(l.as_int() <= r.as_int()) },
            Gt     => if use_float { RVal::Bool(l.as_float() > r.as_float()) } else { RVal::Bool(l.as_int() > r.as_int()) },
            Ge     => if use_float { RVal::Bool(l.as_float() >= r.as_float()) } else { RVal::Bool(l.as_int() >= r.as_int()) },
            And    => RVal::Bool(l.is_truthy() && r.is_truthy()),
            Or     => RVal::Bool(l.is_truthy() || r.is_truthy()),
            BitAnd => RVal::Int(l.as_int() & r.as_int()),
            BitOr  => RVal::Int(l.as_int() | r.as_int()),
            Xor    => RVal::Int(l.as_int() ^ r.as_int()),
            Shl    => RVal::Int(l.as_int() << (r.as_int() & 63)),
            Shr    => RVal::Int(l.as_int() >> (r.as_int() & 63)),
            UShr   => RVal::Int(((l.as_int() as u64) >> (r.as_int() & 63)) as i64),
        })
    }

    pub(super) fn values_equal(&self, l: &RVal, r: &RVal) -> bool {
        match (l, r) {
            (RVal::Int(a),    RVal::Int(b))    => a == b,
            (RVal::Float(a),  RVal::Float(b))  => a == b,
            (RVal::Int(a),    RVal::Float(b))  => (*a as f64) == *b,
            (RVal::Float(a),  RVal::Int(b))    => *a == (*b as f64),
            (RVal::Str(a),    RVal::Str(b))    => a == b,
            (RVal::Bool(a),   RVal::Bool(b))   => a == b,
            (RVal::Null,      RVal::Null)       => true,
            (RVal::Object(a), RVal::Object(b)) => a == b,
            (RVal::Int(a),    RVal::Bool(b))   => *a == (if *b { 1 } else { 0 }),
            (RVal::Bool(a),   RVal::Int(b))    => (if *a { 1i64 } else { 0 }) == *b,
            _ => false,
        }
    }
}
