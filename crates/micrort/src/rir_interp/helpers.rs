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
                       "left", "right", "parent", "count", "index", "head", "tail",
                       "code", "type", "message", "cause", "id", "val", "num",
                       "result", "error", "status", "flag", "mode", "level"] {
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
                       "PriorityQueue", "LinkedList", "HashSet", "TreeSet", "Stack",
                       "Object", "Exception",
                       "RuntimeException", "NullPointerException",
                       "IllegalArgumentException", "IllegalStateException",
                       "IndexOutOfBoundsException", "ArrayIndexOutOfBoundsException",
                       "ClassCastException", "UnsupportedOperationException",
                       "ArithmeticException", "NumberFormatException",
                       "IOException", "FileNotFoundException",
                       "AssertionError", "Error", "StackOverflowError", "OutOfMemoryError",
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
            ("NumberFormatException", "IllegalArgumentException"),
            ("IOException", "Exception"),
            ("FileNotFoundException", "IOException"),
            ("AssertionError", "Error"),
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
        if let Some(v) = env.get(&val.0) {
            return v.clone();
        }
        // SSA fallback: if val was never set (e.g. loop body never ran),
        // find a __copy__ instruction that copies val into some other name,
        // and return that name's value instead.
        for func in &self.module.functions {
            for bb in &func.basic_blocks {
                for instr in &bb.instrs {
                    if let rava_rir::RirInstr::ConstStr { ret, value } = instr {
                        if let Some(src) = value.strip_prefix("__copy__") {
                            if src == val.0 {
                                if let Some(v) = env.get(&ret.0) {
                                    return v.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
        RVal::Null
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
            // Capture bound method ref receiver: __capture_bound_methodref__(sentinel, receiver)
            if func_id == encode_builtin("__capture_bound_methodref__") {
                if let (Some(RVal::Str(sentinel)), Some(receiver)) = (args.first(), args.get(1)) {
                    super::LAMBDA_CAPTURES.with(|lc| {
                        let mut map = lc.borrow_mut();
                        let caps = map.entry(sentinel.clone()).or_default();
                        caps.insert("__receiver__".into(), receiver.clone());
                    });
                }
                return Ok(RVal::Void);
            }
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
            // Collectors factory methods — create Collector objects
            if func_id == encode_builtin("Collectors.toList") || func_id == encode_builtin("Collectors.toUnmodifiableList") {
                let id = self.alloc_object("Collector");
                self.heap.borrow_mut().get_mut(&id).map(|o| o.fields.insert("__ctype__".into(), RVal::Str("toList".into())));
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.toSet") {
                let id = self.alloc_object("Collector");
                self.heap.borrow_mut().get_mut(&id).map(|o| o.fields.insert("__ctype__".into(), RVal::Str("toSet".into())));
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.joining") {
                let delim = args.first().map(|v| v.to_display()).unwrap_or_default();
                let prefix = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                let suffix = args.get(2).map(|v| v.to_display()).unwrap_or_default();
                let id = self.alloc_object("Collector");
                {
                    let mut heap = self.heap.borrow_mut();
                    if let Some(o) = heap.get_mut(&id) {
                        o.fields.insert("__ctype__".into(), RVal::Str("joining".into()));
                        o.fields.insert("__delim__".into(), RVal::Str(delim));
                        o.fields.insert("__prefix__".into(), RVal::Str(prefix));
                        o.fields.insert("__suffix__".into(), RVal::Str(suffix));
                    }
                }
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.groupingBy") {
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                let id = self.alloc_object("Collector");
                {
                    let mut heap = self.heap.borrow_mut();
                    if let Some(o) = heap.get_mut(&id) {
                        o.fields.insert("__ctype__".into(), RVal::Str("groupingBy".into()));
                        o.fields.insert("__lambda__".into(), lambda);
                    }
                }
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.counting") {
                let id = self.alloc_object("Collector");
                self.heap.borrow_mut().get_mut(&id).map(|o| o.fields.insert("__ctype__".into(), RVal::Str("counting".into())));
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.partitioningBy") {
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                let id = self.alloc_object("Collector");
                {
                    let mut heap = self.heap.borrow_mut();
                    if let Some(o) = heap.get_mut(&id) {
                        o.fields.insert("__ctype__".into(), RVal::Str("partitioningBy".into()));
                        o.fields.insert("__lambda__".into(), lambda);
                    }
                }
                return Ok(RVal::Object(id));
            }
            if func_id == encode_builtin("Collectors.toMap") {
                let key_fn = args.first().cloned().unwrap_or(RVal::Null);
                let val_fn = args.get(1).cloned().unwrap_or(RVal::Null);
                let id = self.alloc_object("Collector");
                {
                    let mut heap = self.heap.borrow_mut();
                    if let Some(o) = heap.get_mut(&id) {
                        o.fields.insert("__ctype__".into(), RVal::Str("toMap".into()));
                        o.fields.insert("__keyfn__".into(), key_fn);
                        o.fields.insert("__valfn__".into(), val_fn);
                    }
                }
                return Ok(RVal::Object(id));
            }
            // Comparator.naturalOrder() / reverseOrder()
            if func_id == encode_builtin("Comparator.naturalOrder") {
                return Ok(RVal::Str("__comparator__natural__".into()));
            }
            if func_id == encode_builtin("Comparator.reverseOrder") {
                return Ok(RVal::Str("__comparator__reverse__".into()));
            }
            // Comparator.comparingInt(keyExtractor) / Comparator.comparing(keyExtractor)
            if func_id == encode_builtin("Comparator.comparingInt")
                || func_id == encode_builtin("Comparator.comparing")
                || func_id == encode_builtin("Comparator.comparingLong")
                || func_id == encode_builtin("Comparator.comparingDouble")
            {
                if let Some(key_fn) = args.first() {
                    let name = format!("__comparator__by__{}", key_fn.to_display());
                    crate::rir_interp::LAMBDA_CAPTURES.with(|lc| {
                        let mut map = lc.borrow_mut();
                        let captures = map.entry(name.clone()).or_default();
                        captures.insert("__keyfn__".into(), key_fn.clone());
                    });
                    return Ok(RVal::Str(name));
                }
                return Ok(RVal::Str("__comparator__natural__".into()));
            }
            // Stream.of(...) / Arrays.stream(...) — create a stream from args
            if func_id == encode_builtin("Stream.of") || func_id == encode_builtin("Arrays.stream") {
                // If single arg is already an array, use it directly; otherwise collect all args
                let items = if args.len() == 1 {
                    if let RVal::Array(a) = &args[0] { a.borrow().clone() } else { args.clone() }
                } else {
                    args.clone()
                };
                return Ok(RVal::Array(Rc::new(RefCell::new(items))));
            }
            // Stream.generate(supplier) — produce 10 elements (lazy streams not supported)
            if func_id == encode_builtin("Stream.generate") {
                if let Some(supplier) = args.first() {
                    let supplier = supplier.clone();
                    let mut items = Vec::new();
                    for _ in 0..10 {
                        items.push(self.invoke_lambda(&supplier, &[])?);
                    }
                    return Ok(RVal::Array(Rc::new(RefCell::new(items))));
                }
                return Ok(RVal::Array(Rc::new(RefCell::new(vec![]))));
            }
            // Stream.iterate(seed, f) — produce 10 elements
            if func_id == encode_builtin("Stream.iterate") {
                let seed = args.first().cloned().unwrap_or(RVal::Int(0));
                if let Some(f) = args.get(1) {
                    let f = f.clone();
                    let mut items = vec![seed.clone()];
                    let mut cur = seed;
                    for _ in 0..9 {
                        cur = self.invoke_lambda(&f, &[cur.clone()])?;
                        items.push(cur.clone());
                    }
                    return Ok(RVal::Array(Rc::new(RefCell::new(items))));
                }
                return Ok(RVal::Array(Rc::new(RefCell::new(vec![seed]))));
            }
            // IntStream.range(start, end) / rangeClosed(start, end)
            if func_id == encode_builtin("IntStream.range") {
                let start = args.first().map(|v| v.as_int()).unwrap_or(0);
                let end   = args.get(1).map(|v| v.as_int()).unwrap_or(0);
                let items = (start..end).map(RVal::Int).collect();
                return Ok(RVal::Array(Rc::new(RefCell::new(items))));
            }
            if func_id == encode_builtin("IntStream.rangeClosed") {
                let start = args.first().map(|v| v.as_int()).unwrap_or(0);
                let end   = args.get(1).map(|v| v.as_int()).unwrap_or(0);
                let items = (start..=end).map(RVal::Int).collect();
                return Ok(RVal::Array(Rc::new(RefCell::new(items))));
            }
            // IntStream.of(...)
            if func_id == encode_builtin("IntStream.of") {
                return Ok(RVal::Array(Rc::new(RefCell::new(args.to_vec()))));
            }
        }

        if let Some(method_name) = self.resolve_method_name(func_id) {
            if let Some(receiver) = args.first() {
                // NullPointerException on null receiver — only for builtin methods,
                // not user-defined ones (which are dispatched below via find_method_in_chain)
                let is_builtin_method = super::KNOWN_METHODS.contains(&method_name.as_str());
                if matches!(receiver, RVal::Null) && is_builtin_method {
                    return Err(RavaError::JavaException {
                        exception_type: "NullPointerException".into(),
                        message: format!("Cannot invoke method '{}' on null", method_name),
                    });
                }
                let method_args = &args[1..];
                if let Some(result) = builtins::dispatch_named_method(receiver, &method_name, method_args) {
                    return result;
                }
                // sort(comparator) on ArrayList — needs interpreter for lambda invocation
                if method_name == "sort" {
                    if let RVal::Array(arr) = receiver {
                        let comparator = method_args.first().cloned().unwrap_or(RVal::Null);
                        let mut elems = arr.borrow().clone();
                        if matches!(comparator, RVal::Null) {
                            elems.sort_by(|a, b| crate::builtins::rval_cmp(a, b));
                        } else {
                            for i in 1..elems.len() {
                                let mut j = i;
                                while j > 0 {
                                    let cmp = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                                    if cmp.as_int() > 0 {
                                        elems.swap(j-1, j);
                                        j -= 1;
                                    } else { break; }
                                }
                            }
                        }
                        *arr.borrow_mut() = elems;
                        return Ok(RVal::Void);
                    }
                }
                // Optional.orElseGet / ifPresent / map — need interpreter for lambda invocation
                if let RVal::Array(arr) = receiver {
                    match method_name.as_str() {
                        "orElseGet" => {
                            let is_empty = matches!(arr.borrow().first(), Some(RVal::Null) | None);
                            if is_empty {
                                let supplier = method_args.first().cloned().unwrap_or(RVal::Null);
                                return self.invoke_lambda(&supplier, &[]);
                            } else {
                                return Ok(arr.borrow().first().cloned().unwrap_or(RVal::Null));
                            }
                        }
                        "ifPresent" => {
                            let val = arr.borrow().first().cloned().unwrap_or(RVal::Null);
                            if !matches!(val, RVal::Null) {
                                let consumer = method_args.first().cloned().unwrap_or(RVal::Null);
                                self.invoke_lambda(&consumer, &[val])?;
                            }
                            return Ok(RVal::Void);
                        }
                        "map" if arr.borrow().len() == 1 => {
                            let val = arr.borrow().first().cloned().unwrap_or(RVal::Null);
                            if matches!(val, RVal::Null) {
                                return Ok(RVal::Array(Rc::new(RefCell::new(vec![RVal::Null]))));
                            }
                            let mapper = method_args.first().cloned().unwrap_or(RVal::Null);
                            let result = self.invoke_lambda(&mapper, &[val])?;
                            return Ok(RVal::Array(Rc::new(RefCell::new(vec![result]))));
                        }
                        _ => {}
                    }
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
                    // Stack — LIFO operations
                    if class_name == "Stack" {
                        let items = self.heap.borrow().get(id)
                            .and_then(|o| o.fields.get("__items__").cloned())
                            .unwrap_or(RVal::Null);
                        match method_name.as_str() {
                            "push" => {
                                let val = method_args.first().cloned().unwrap_or(RVal::Null);
                                if let RVal::Array(a) = &items { a.borrow_mut().push(val.clone()); }
                                return Ok(val);
                            }
                            "pop" => {
                                if let RVal::Array(a) = &items {
                                    return Ok(a.borrow_mut().pop().unwrap_or(RVal::Null));
                                }
                                return Ok(RVal::Null);
                            }
                            "peek" => {
                                if let RVal::Array(a) = &items {
                                    return Ok(a.borrow().last().cloned().unwrap_or(RVal::Null));
                                }
                                return Ok(RVal::Null);
                            }
                            "size" => {
                                if let RVal::Array(a) = &items { return Ok(RVal::Int(a.borrow().len() as i64)); }
                                return Ok(RVal::Int(0));
                            }
                            "isEmpty" => {
                                if let RVal::Array(a) = &items { return Ok(RVal::Bool(a.borrow().is_empty())); }
                                return Ok(RVal::Bool(true));
                            }
                            "add" => {
                                let val = method_args.first().cloned().unwrap_or(RVal::Null);
                                if let RVal::Array(a) = &items { a.borrow_mut().push(val); }
                                return Ok(RVal::Bool(true));
                            }
                            _ => {}
                        }
                    }
                    if class_name == "HashMap" || class_name == "TreeMap" || class_name == "LinkedHashMap" {
                        // TreeMap firstKey/lastKey
                        if class_name == "TreeMap" && (method_name == "firstKey" || method_name == "lastKey") {
                            let mut keys: Vec<String> = {
                                let heap = self.heap.borrow();
                                heap.get(id).map(|o| o.fields.keys()
                                    .filter(|k| !k.starts_with("__"))
                                    .cloned().collect())
                                    .unwrap_or_default()
                            };
                            keys.sort();
                            let key = if method_name == "firstKey" { keys.first() } else { keys.last() };
                            return Ok(key.map(|k| {
                                // Try to parse as int for numeric keys
                                k.parse::<i64>().map(RVal::Int).unwrap_or_else(|_| RVal::Str(k.clone()))
                            }).unwrap_or(RVal::Null));
                        }
                        // TreeMap forEach must iterate in sorted key order
                        if class_name == "TreeMap" && method_name == "forEach" {
                            let mut pairs: Vec<(String, RVal)> = {
                                let heap = self.heap.borrow();
                                if let Some(obj) = heap.get(id) {
                                    obj.fields.iter()
                                        .filter(|(k, _)| !k.starts_with("__"))
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect()
                                } else { vec![] }
                            };
                            pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
                            let lambda = method_args.first().cloned().unwrap_or(RVal::Null);
                            for (k, v) in pairs {
                                self.invoke_lambda(&lambda, &[RVal::Str(k), v])?;
                            }
                            return Ok(RVal::Void);
                        }
                        if let Some(result) = self.dispatch_hash_map(*id, &method_name, method_args) {
                            // For TreeMap, sort keySet/entrySet/values results by key
                            if class_name == "TreeMap" && (method_name == "keySet" || method_name == "values" || method_name == "entrySet") {
                                if let Ok(RVal::Array(arr)) = &result {
                                    if method_name == "entrySet" {
                                        // Sort entry objects by their __key__ field
                                        let heap = self.heap.borrow();
                                        arr.borrow_mut().sort_by(|a, b| {
                                            let ka = if let RVal::Object(id) = a {
                                                heap.get(id).and_then(|o| o.fields.get("__key__")).map(|v| v.to_display()).unwrap_or_default()
                                            } else { a.to_display() };
                                            let kb = if let RVal::Object(id) = b {
                                                heap.get(id).and_then(|o| o.fields.get("__key__")).map(|v| v.to_display()).unwrap_or_default()
                                            } else { b.to_display() };
                                            ka.cmp(&kb)
                                        });
                                    } else {
                                        arr.borrow_mut().sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                                    }
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
                    if class_name == "HashSet" || class_name == "TreeSet" || class_name == "LinkedHashSet" {
                        if let Some(result) = self.dispatch_set(*id, &class_name, &method_name, method_args) {
                            return result;
                        }
                    }
                    let effective_count = method_args.len() + 1;
                    let func_idx = self.find_method_in_chain(&class_name, &method_name, effective_count);
                    if let Some(idx) = func_idx {
                        let func = &self.module.functions[idx];
                        let mut call_env: HashMap<String, RVal> = HashMap::new();
                        // For anonymous classes, inject captured fields into env
                        if class_name.starts_with("__anon_") {
                            let captures: HashMap<String, RVal> = self.heap.borrow()
                                .get(id)
                                .map(|o| o.fields.iter()
                                    .filter(|(k, _)| k.starts_with("__cap__"))
                                    .map(|(k, v)| (k.strip_prefix("__cap__").unwrap().to_string(), v.clone()))
                                    .collect())
                                .unwrap_or_default();
                            call_env.extend(captures);
                        }
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
                        "getCause" => {
                            let cause = self.heap.borrow().get(id)
                                .and_then(|o| o.fields.get("__cause__").cloned())
                                .unwrap_or(RVal::Null);
                            return Ok(cause);
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
                    // Functional interface composition methods on lambdas
                    if s.starts_with("__lambda_") || s.starts_with("__composed_") || s.starts_with("__methodref__") || s.starts_with("__comparator__") {
                        if let Some(result) = self.dispatch_stream(&receiver, &method_name, method_args) {
                            return result;
                        }
                        // Default: invoke the lambda/method-ref directly for any functional interface method
                        if method_name == "apply" || method_name == "test" || method_name == "accept"
                            || method_name == "run" || method_name == "get" || method_name == "execute"
                            || method_name == "call" || method_name == "invoke" || method_name == "transform"
                            || method_name == "compare" || method_name == "compareTo"
                        {
                            // For method refs, call the builtin method on the first arg
                            if s.starts_with("__methodref__") {
                                if let Some(rest) = s.strip_prefix("__methodref__") {
                                    if let Some((_cls, meth)) = rest.split_once("::") {
                                        if let Some(first_arg) = method_args.first() {
                                            if let Some(result) = builtins::dispatch_named_method(first_arg, meth, &method_args[1..]) {
                                                return result;
                                            }
                                        }
                                    }
                                }
                            }
                            return self.invoke_lambda(&receiver, method_args);
                        }
                    }
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
                    match &comparator {
                        RVal::Str(s) if s == "__comparator__reverse__" => {
                            elems.sort_by(|a, b| crate::builtins::rval_cmp(b, a));
                        }
                        RVal::Str(s) if s == "__comparator__natural__" => {
                            elems.sort_by(|a, b| crate::builtins::rval_cmp(a, b));
                        }
                        _ => {
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
                        }
                    }
                    *arr.borrow_mut() = elems;
                    return Ok(RVal::Void);
                }
            }
            // Collections.sort with 1 arg — use compareTo for user objects
            if func_id == encode_builtin("Collections.sort") && args.len() == 1 {
                if let RVal::Array(arr) = &args[0] {
                    let items = arr.borrow().clone();
                    // Check if elements are user objects with compareTo
                    let has_user_compare = items.first().map(|v| matches!(v, RVal::Object(_))).unwrap_or(false);
                    if has_user_compare {
                        let mut elems = items;
                        let compare_hash = encode_builtin("__method__compareTo");
                        for i in 1..elems.len() {
                            let mut j = i;
                            while j > 0 {
                                let dummy_env = std::collections::HashMap::new();
                                let a_val = rava_rir::Value(format!("__a{}", j));
                                let b_val = rava_rir::Value(format!("__b{}", j));
                                let mut tmp_env = dummy_env.clone();
                                tmp_env.insert(a_val.0.clone(), elems[j-1].clone());
                                tmp_env.insert(b_val.0.clone(), elems[j].clone());
                                let cmp_result = self.dispatch_call(
                                    compare_hash,
                                    &[a_val, b_val],
                                    &tmp_env,
                                )?;
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
        }

        // this(...) constructor delegation
        {
            use crate::lowerer_hash::encode_builtin;
            // StringBuilder.<init>(String) — initialize __buf__ with the string argument
            if func_id == encode_builtin("StringBuilder.<init>") {
                if let Some(RVal::Object(id)) = args.first() {
                    let init_str = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                    let mut heap = self.heap.borrow_mut();
                    if let Some(obj) = heap.get_mut(id) {
                        obj.fields.insert("__buf__".into(), RVal::Str(init_str));
                    }
                }
                return Ok(RVal::Void);
            }
            // Stack.<init> — no-op, object already initialized by New instruction
            if func_id == encode_builtin("Stack.<init>") {
                return Ok(RVal::Void);
            }
            // HashMap/TreeMap/LinkedHashMap copy constructor — copy entries from source map
            for map_type in &["HashMap", "TreeMap", "LinkedHashMap"] {
                if func_id == encode_builtin(&format!("{}.<init>", map_type)) {
                    if let (Some(RVal::Object(dst_id)), Some(RVal::Object(src_id))) = (args.first(), args.get(1)) {
                        let src_fields: Vec<(String, RVal)> = {
                            let heap = self.heap.borrow();
                            heap.get(src_id)
                                .map(|o| o.fields.iter()
                                    .filter(|(k, _)| !k.starts_with("__"))
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect())
                                .unwrap_or_default()
                        };
                        let mut heap = self.heap.borrow_mut();
                        if let Some(dst) = heap.get_mut(dst_id) {
                            for (k, v) in src_fields {
                                dst.fields.insert(k, v);
                            }
                        }
                    }
                    return Ok(RVal::Void);
                }
            }
            // HashSet/TreeSet/LinkedHashSet copy constructor — copy items from source collection
            for set_type in &["HashSet", "TreeSet", "LinkedHashSet"] {
                if func_id == encode_builtin(&format!("{}.<init>", set_type)) {
                    if let (Some(RVal::Object(dst_id)), Some(src)) = (args.first(), args.get(1)) {
                        let src_items: Vec<RVal> = match src {
                            RVal::Array(arr) => arr.borrow().clone(),
                            RVal::Object(src_id) => {
                                let heap = self.heap.borrow();
                                heap.get(src_id)
                                    .and_then(|o| o.fields.get("__items__"))
                                    .and_then(|v| if let RVal::Array(a) = v { Some(a.borrow().clone()) } else { None })
                                    .unwrap_or_default()
                            }
                            _ => vec![],
                        };
                        let mut heap = self.heap.borrow_mut();
                        if let Some(dst) = heap.get_mut(dst_id) {
                            if let Some(RVal::Array(items)) = dst.fields.get("__items__") {
                                *items.borrow_mut() = src_items;
                            }
                        }
                    }
                    return Ok(RVal::Void);
                }
            }
            // ArrayList copy constructor — copy from source array/collection
            if func_id == encode_builtin("ArrayList.<init>") {
                if let (Some(RVal::Array(dst)), Some(src)) = (args.first(), args.get(1)) {
                    match src {
                        RVal::Array(src_arr) => {
                            *dst.borrow_mut() = src_arr.borrow().clone();
                        }
                        _ => {}
                    }
                }
                return Ok(RVal::Void);
            }
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
            // super.<method>(...) — non-virtual dispatch to parent class method
            if let Some(method_name) = {
                use crate::lowerer_hash::encode_builtin;
                // Scan all known method names to find which one matches this func_id
                super::KNOWN_METHODS.iter()
                    .find(|&&m| encode_builtin(&format!("super.{}", m)) == func_id)
                    .map(|&m| m.to_string())
                    .or_else(|| {
                        // Fall back to scanning user-defined function names
                        self.module.functions.iter()
                            .find(|f| {
                                f.name.contains('.') && {
                                    let parts: Vec<&str> = f.name.splitn(2, '.').collect();
                                    parts.len() == 2 && encode_builtin(&format!("super.{}", parts[1])) == func_id
                                }
                            })
                            .map(|f| f.name.splitn(2, '.').nth(1).unwrap_or("").to_string())
                    })
            } {
                // Find the parent class of `this`, then walk up until we find the method
                if let Some(RVal::Object(id)) = args.first() {
                    let class_name = self.heap.borrow().get(id)
                        .map(|o| o.class_name.clone())
                        .unwrap_or_default();
                    let arg_count = args.len();
                    // Walk up the hierarchy starting from the immediate parent
                    let mut cur = self.module.class_hierarchy.get(&class_name).cloned();
                    while let Some(parent_class) = cur {
                        let target_name = format!("{}.{}", parent_class, method_name);
                        let idx = self.module.functions.iter()
                            .position(|f| f.name == target_name && f.params.len() == arg_count)
                            .or_else(|| self.module.functions.iter()
                                .position(|f| f.name == target_name));
                        if let Some(idx) = idx {
                            let func = &self.module.functions[idx];
                            let mut call_env: HashMap<String, RVal> = HashMap::new();
                            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                                call_env.insert(param_name.0.clone(), val.clone());
                            }
                            return self.exec_function_idx(idx, call_env);
                        }
                        cur = self.module.class_hierarchy.get(&parent_class).cloned();
                    }
                }
            }

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

        // List.sort(comparator) — needs interpreter access for lambda invocation
        {
            use crate::lowerer_hash::encode_builtin;
            if func_id == encode_builtin("__method__sort") {
                if let (Some(RVal::Array(arr)), Some(comparator)) = (args.first(), args.get(1)) {
                    let comparator = comparator.clone();
                    let mut elems = arr.borrow().clone();
                    for i in 1..elems.len() {
                        let mut j = i;
                        while j > 0 {
                            let cmp = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                            if cmp.as_int() > 0 {
                                elems.swap(j-1, j);
                                j -= 1;
                            } else { break; }
                        }
                    }
                    *arr.borrow_mut() = elems;
                    return Ok(RVal::Void);
                }
            }
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
                        if !matches!(msg_arg, RVal::Object(_)) {
                            obj.fields.insert("message".into(), msg_arg.clone());
                        }
                    }
                    // RuntimeException(String msg, Throwable cause) — store cause
                    if let Some(cause_arg) = args.get(2) {
                        if matches!(cause_arg, RVal::Object(_)) {
                            obj.fields.insert("__cause__".into(), cause_arg.clone());
                        }
                    }
                    // RuntimeException(Throwable cause) — single Object arg is the cause
                    if args.len() == 2 {
                        if let Some(RVal::Object(_)) = args.get(1) {
                            obj.fields.insert("__cause__".into(), args[1].clone());
                        }
                    }
                }
            }
        }

        Ok(RVal::Void)
    }

    // ── dispatch_virtual ──────────────────────────────────────────────────────

    pub(super) fn dispatch_virtual_named(&self, receiver: RVal, method_name: Option<&str>, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();

        // Lambda / method-ref dispatch
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
                // Builtin instance method ref: Class::method — call method on first arg
                if let Some((_cls, meth)) = rest.split_once("::") {
                    if let Some(first_arg) = args.first() {
                        if let Some(result) = builtins::dispatch_named_method(first_arg, meth, &args[1..]) {
                            return result;
                        }
                    }
                }
            }
            // java.time and other tagged-string instance methods
            if let Some(method) = method_name {
                if let Some(result) = builtins::dispatch_named_method(&receiver, method, &args) {
                    return result;
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

            // Named method dispatch on known instance methods
            if let Some(method) = method_name {
                if let Some(result) = builtins::dispatch_named_method(&receiver, method, &args) {
                    return result;
                }

                // Map.Entry key/value
                if method == "getKey" {
                    let key = self.heap.borrow().get(id)
                        .and_then(|o| o.fields.get("__key__").cloned())
                        .unwrap_or(RVal::Null);
                    return Ok(key);
                }
                if method == "getValue" {
                    let val = self.heap.borrow().get(id)
                        .and_then(|o| o.fields.get("__value__").cloned())
                        .unwrap_or(RVal::Null);
                    return Ok(val);
                }

                // StringBuilder
                if class_name == "StringBuilder" {
                    if let Some(result) = self.dispatch_string_builder(*id, method, &args) {
                        return result;
                    }
                }

                // User-defined method: look up by class + method name in hierarchy
                let effective_count = args.len() + 1;
                if let Some(idx) = self.find_method_in_chain(&class_name, method, effective_count) {
                    let func = &self.module.functions[idx];
                    let mut call_env: HashMap<String, RVal> = HashMap::new();
                    let mut all_args = vec![receiver.clone()];
                    all_args.extend(args.iter().cloned());
                    for ((param_name, _), val) in func.params.iter().zip(all_args.iter()) {
                        call_env.insert(param_name.0.clone(), val.clone());
                    }
                    return self.exec_function_idx(idx, call_env);
                }
            } else {
                // Fallback: old behavior (no method name known)
                return self.dispatch_virtual(receiver, 0, arg_vals, env);
            }
        }

        // Fallback for non-object receivers with known method name
        if let Some(method) = method_name {
            if let Some(result) = builtins::dispatch_named_method(&receiver, method, &args) {
                return result;
            }
        }

        Ok(RVal::Void)
    }

    pub(super) fn dispatch_virtual(&self, receiver: RVal, method_id: u32, arg_vals: &[Value], env: &HashMap<String, RVal>) -> Result<RVal> {
        use crate::lowerer_hash::encode_builtin;
        let args: Vec<RVal> = arg_vals.iter().map(|v| self.resolve(env, v)).collect();
        // Resolve method name from hash
        let method_name = self.module.method_names.get(&method_id).cloned()
            .or_else(|| {
                for m in super::KNOWN_METHODS {
                    if encode_builtin(m) == method_id { return Some(m.to_string()); }
                }
                None
            });
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
            // Comparator/lambda method dispatch
            if s.starts_with("__comparator__") || s.starts_with("__composed_") {
                if let Some(ref mname) = method_name {
                    if let Some(result) = self.dispatch_stream(&receiver, mname, &args) {
                        return result;
                    }
                }
                return self.invoke_lambda(&receiver, &args);
            }
        }
        if let Some(result) = builtins::dispatch_method(&receiver, &args) {
            return result;
        }
        // Named method dispatch using method_id
        if let Some(ref mname) = method_name {
            if let Some(result) = builtins::dispatch_named_method(&receiver, mname, &args) {
                return result;
            }
            // sort(comparator) on ArrayList
            if mname == "sort" {
                if let RVal::Array(arr) = &receiver {
                    if let Some(comparator) = args.first() {
                        let comparator = comparator.clone();
                        let mut elems = arr.borrow().clone();
                        for i in 1..elems.len() {
                            let mut j = i;
                            while j > 0 {
                                let cmp = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                                if cmp.as_int() > 0 { elems.swap(j-1, j); j -= 1; } else { break; }
                            }
                        }
                        *arr.borrow_mut() = elems;
                        return Ok(RVal::Void);
                    }
                }
            }
            // forEach on array with lambda
            if mname == "forEach" {
                if let RVal::Array(arr) = &receiver {
                    if let Some(lambda) = args.first() {
                        let lambda = lambda.clone();
                        let items = arr.borrow().clone();
                        for item in &items {
                            self.invoke_lambda(&lambda, &[item.clone()])?;
                        }
                        return Ok(RVal::Void);
                    }
                }
            }
        }
        // sort(comparator) on ArrayList — fallback for unknown method name
        if let RVal::Array(arr) = &receiver {
            if args.len() == 1 && method_name.as_deref() != Some("forEach") {
                let comparator = args[0].clone();
                let mut elems = arr.borrow().clone();
                for i in 1..elems.len() {
                    let mut j = i;
                    while j > 0 {
                        let cmp = self.invoke_lambda(&comparator, &[elems[j-1].clone(), elems[j].clone()])?;
                        if cmp.as_int() > 0 { elems.swap(j-1, j); j -= 1; } else { break; }
                    }
                }
                *arr.borrow_mut() = elems;
                return Ok(RVal::Void);
            }
        }
        if let RVal::Object(id) = &receiver {
            let class_name = self.heap.borrow().get(id)
                .map(|o| o.class_name.clone())
                .unwrap_or_default();
            if class_name == "StringBuilder" {
                let mname = method_name.as_deref().unwrap_or("toString");
                if let Some(result) = self.dispatch_string_builder(*id, mname, &args) {
                    return result;
                }
            }
            // Stack — LIFO operations on __items__ array
            if class_name == "Stack" {
                let items = self.heap.borrow().get(id)
                    .and_then(|o| o.fields.get("__items__").cloned())
                    .unwrap_or(RVal::Null);
                let mname = method_name.as_deref().unwrap_or("");
                match mname {
                    "push" => {
                        let val = args.first().cloned().unwrap_or(RVal::Null);
                        if let RVal::Array(a) = &items { a.borrow_mut().push(val.clone()); }
                        return Ok(val);
                    }
                    "pop" => {
                        if let RVal::Array(a) = &items {
                            let v = a.borrow_mut().pop().unwrap_or(RVal::Null);
                            return Ok(v);
                        }
                        return Ok(RVal::Null);
                    }
                    "peek" => {
                        if let RVal::Array(a) = &items {
                            return Ok(a.borrow().last().cloned().unwrap_or(RVal::Null));
                        }
                        return Ok(RVal::Null);
                    }
                    "size" => {
                        if let RVal::Array(a) = &items { return Ok(RVal::Int(a.borrow().len() as i64)); }
                        return Ok(RVal::Int(0));
                    }
                    "isEmpty" => {
                        if let RVal::Array(a) = &items { return Ok(RVal::Bool(a.borrow().is_empty())); }
                        return Ok(RVal::Bool(true));
                    }
                    "add" => {
                        let val = args.first().cloned().unwrap_or(RVal::Null);
                        if let RVal::Array(a) = &items { a.borrow_mut().push(val); }
                        return Ok(RVal::Bool(true));
                    }
                    _ => {}
                }
            }
            let effective_count = args.len() + 1;
            let func_idx = {
                let mut found = None;
                let mut cur = class_name.clone();
                loop {
                    let prefix = format!("{}.", cur);
                    let idx = self.module.functions.iter()
                        .position(|f| {
                            let name_ok = if let Some(ref mname) = method_name {
                                f.name == format!("{}{}", prefix, mname)
                            } else {
                                f.name.starts_with(&prefix)
                            };
                            name_ok && !f.flags.is_constructor && !f.flags.is_clinit
                                && f.params.len() == effective_count
                        })
                        .or_else(|| self.module.functions.iter()
                            .position(|f| {
                                let name_ok = if let Some(ref mname) = method_name {
                                    f.name == format!("{}{}", prefix, mname)
                                } else {
                                    f.name.starts_with(&prefix)
                                };
                                name_ok && !f.flags.is_constructor && !f.flags.is_clinit
                            }));
                    if idx.is_some() { found = idx; break; }
                    for (key, iface) in &self.module.class_hierarchy {
                        if key.starts_with(&format!("{}:", cur)) {
                            let iface_prefix = format!("{}.", iface);
                            let idx = self.module.functions.iter()
                                .position(|f| {
                                    let name_ok = if let Some(ref mname) = method_name {
                                        f.name == format!("{}{}", iface_prefix, mname)
                                    } else {
                                        f.name.starts_with(&iface_prefix)
                                    };
                                    name_ok && !f.flags.is_constructor && !f.flags.is_clinit
                                        && f.params.len() == effective_count
                                })
                                .or_else(|| self.module.functions.iter()
                                    .position(|f| {
                                        let name_ok = if let Some(ref mname) = method_name {
                                            f.name == format!("{}{}", iface_prefix, mname)
                                        } else {
                                            f.name.starts_with(&iface_prefix)
                                        };
                                        name_ok && !f.flags.is_constructor && !f.flags.is_clinit
                                    }));
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
            // Inject captured variables first
            super::LAMBDA_CAPTURES.with(|lc| {
                if let Some(caps) = lc.borrow().get(name) {
                    call_env.extend(caps.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
            });
            for ((param_name, _), val) in func.params.iter().zip(args.iter()) {
                call_env.insert(param_name.0.clone(), val.clone());
            }
            return self.exec_function_idx(idx, call_env);
        }
        Ok(RVal::Void)
    }

    pub(super) fn invoke_lambda(&self, lambda: &RVal, args: &[RVal]) -> Result<RVal> {
        match lambda {
            RVal::Str(s) if s.starts_with("__composed_andThen__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let after = caps.get("__after__").cloned().unwrap_or(RVal::Null);
                    let mid = self.invoke_lambda(&f, args)?;
                    return self.invoke_lambda(&after, &[mid]);
                }
                Ok(RVal::Null)
            }
            RVal::Str(s) if s.starts_with("__composed_compose__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let before = caps.get("__before__").cloned().unwrap_or(RVal::Null);
                    let mid = self.invoke_lambda(&before, args)?;
                    return self.invoke_lambda(&f, &[mid]);
                }
                Ok(RVal::Null)
            }
            RVal::Str(s) if s.starts_with("__composed_and__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let other = caps.get("__other__").cloned().unwrap_or(RVal::Null);
                    let r1 = self.invoke_lambda(&f, args)?;
                    if !r1.is_truthy() { return Ok(RVal::Bool(false)); }
                    return self.invoke_lambda(&other, args);
                }
                Ok(RVal::Bool(false))
            }
            RVal::Str(s) if s.starts_with("__composed_or__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let other = caps.get("__other__").cloned().unwrap_or(RVal::Null);
                    let r1 = self.invoke_lambda(&f, args)?;
                    if r1.is_truthy() { return Ok(RVal::Bool(true)); }
                    return self.invoke_lambda(&other, args);
                }
                Ok(RVal::Bool(false))
            }
            RVal::Str(s) if s.starts_with("__composed_negate__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let r = self.invoke_lambda(&f, args)?;
                    return Ok(RVal::Bool(!r.is_truthy()));
                }
                Ok(RVal::Bool(false))
            }
            // Comparator sentinels — natural/reverse order
            RVal::Str(s) if s == "__comparator__natural__" => {
                let a = args.first().cloned().unwrap_or(RVal::Null);
                let b = args.get(1).cloned().unwrap_or(RVal::Null);
                Ok(RVal::Int(crate::builtins::rval_cmp(&a, &b) as i64))
            }
            RVal::Str(s) if s == "__comparator__reverse__" => {
                let a = args.first().cloned().unwrap_or(RVal::Null);
                let b = args.get(1).cloned().unwrap_or(RVal::Null);
                Ok(RVal::Int(crate::builtins::rval_cmp(&b, &a) as i64))
            }
            // Comparator.comparingInt(keyFn) — compare by extracted key
            RVal::Str(s) if s.starts_with("__comparator__by__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let key_fn = caps.get("__keyfn__").cloned().unwrap_or(RVal::Null);
                    let a = args.first().cloned().unwrap_or(RVal::Null);
                    let b = args.get(1).cloned().unwrap_or(RVal::Null);
                    let ka = self.invoke_lambda(&key_fn, &[a])?;
                    let kb = self.invoke_lambda(&key_fn, &[b])?;
                    return Ok(RVal::Int(crate::builtins::rval_cmp(&ka, &kb) as i64));
                }
                Ok(RVal::Int(0))
            }
            // Comparator.thenComparing — chain two comparators
            RVal::Str(s) if s.starts_with("__comparator__then__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let primary = caps.get("__primary__").cloned().unwrap_or(RVal::Null);
                    let secondary = caps.get("__secondary__").cloned().unwrap_or(RVal::Null);
                    let r1 = self.invoke_lambda(&primary, args)?;
                    if r1.as_int() != 0 { return Ok(r1); }
                    return self.invoke_lambda(&secondary, args);
                }
                Ok(RVal::Int(0))
            }
            // Comparator.reversed()
            RVal::Str(s) if s.starts_with("__comparator__reversed__") => {
                let captures = super::LAMBDA_CAPTURES.with(|lc| lc.borrow().get(s.as_str()).cloned());
                if let Some(caps) = captures {
                    let f = caps.get("__f__").cloned().unwrap_or(RVal::Null);
                    let r = self.invoke_lambda(&f, args)?;
                    return Ok(RVal::Int(-r.as_int()));
                }
                Ok(RVal::Int(0))
            }
            RVal::Str(s) if s.starts_with("__lambda_") => self.call_lambda_by_name(s, args),
            RVal::Str(s) if s.starts_with("__bound_methodref__") => {
                // Instance method reference: obj::method where obj was a variable.
                // The receiver was captured via __capture_bound_methodref__.
                let rest = s.strip_prefix("__bound_methodref__").unwrap();
                let receiver = super::LAMBDA_CAPTURES.with(|lc| {
                    lc.borrow().get(s.as_str())
                        .and_then(|caps| caps.get("__receiver__").cloned())
                });
                if let (Some((_var, meth)), Some(recv)) = (rest.split_once("::"), receiver) {
                    if let Some(result) = crate::builtins::dispatch_named_method(&recv, meth, args) {
                        return result;
                    }
                    // Try user-defined method on object
                    if let RVal::Object(id) = &recv {
                        let class_name = self.heap.borrow().get(id)
                            .map(|o| o.class_name.clone())
                            .unwrap_or_default();
                        if let Some(idx) = self.find_method_in_chain(&class_name, meth, args.len() + 1) {
                            let func = &self.module.functions[idx];
                            let mut call_env: HashMap<String, RVal> = HashMap::new();
                            let mut all_args = vec![recv.clone()];
                            all_args.extend_from_slice(args);
                            for ((param_name, _), val) in func.params.iter().zip(all_args.iter()) {
                                call_env.insert(param_name.0.clone(), val.clone());
                            }
                            return self.exec_function_idx(idx, call_env);
                        }
                    }
                }
                Ok(RVal::Null)
            }
            RVal::Str(s) if s.starts_with("__methodref__") => {
                let rest = s.strip_prefix("__methodref__").unwrap();
                // System.out::println / System.out::print
                if rest == "System.out::println" {
                    let val = args.first().cloned().unwrap_or(RVal::Null);
                    super::write_output(&self.obj_to_string(&val));
                    return Ok(RVal::Void);
                }
                if rest == "System.out::print" {
                    let val = args.first().cloned().unwrap_or(RVal::Null);
                    super::write_output_no_nl(&self.obj_to_string(&val));
                    return Ok(RVal::Void);
                }
                // Static method refs: Integer::parseInt, String::valueOf, etc.
                // These are called with args as the method arguments (no receiver)
                {
                    use crate::lowerer_hash::encode_builtin;
                    let full_static = rest.replace("::", ".");
                    let static_id = encode_builtin(&full_static);
                    if let Some(result) = crate::builtins::dispatch(static_id, args) {
                        return result;
                    }
                }
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
            let class_name = self.heap.borrow().get(id)
                .map(|o| o.class_name.clone())
                .unwrap_or_default();
            // Try user-defined toString() first
            if let Some(idx) = self.find_method_in_chain(&class_name, "toString", 1) {
                let func = &self.module.functions[idx];
                let mut call_env: HashMap<String, RVal> = HashMap::new();
                if let Some((param_name, _)) = func.params.first() {
                    call_env.insert(param_name.0.clone(), val.clone());
                }
                if let Ok(result) = self.exec_function_idx(idx, call_env) {
                    return result.to_display();
                }
            }
            // Fallback: check known fields
            let heap = self.heap.borrow();
            if let Some(obj) = heap.get(id) {
                if let Some(msg) = obj.fields.get("message") { return msg.to_display(); }
                if let Some(name) = obj.fields.get("__name__") { return name.to_display(); }
                if let Some(name) = obj.fields.get("_name") { return name.to_display(); }
                // Set types: display as sorted [items] for TreeSet, unsorted for others
                if obj.class_name == "HashSet" || obj.class_name == "TreeSet"
                    || obj.class_name == "LinkedHashSet"
                {
                    if let Some(RVal::Array(items)) = obj.fields.get("__items__") {
                        let mut strs: Vec<String> = items.borrow().iter()
                            .map(|v| v.to_display()).collect();
                        if obj.class_name == "TreeSet" {
                            strs.sort_by(|a, b| {
                                match (a.parse::<i64>(), b.parse::<i64>()) {
                                    (Ok(x), Ok(y)) => x.cmp(&y),
                                    _ => a.cmp(b),
                                }
                            });
                            strs.dedup();
                        }
                        return format!("[{}]", strs.join(", "));
                    }
                }
                // LinkedHashMap: display in insertion order via __keys__
                if obj.class_name == "LinkedHashMap" {
                    if let Some(RVal::Array(keys_arr)) = obj.fields.get("__keys__") {
                        let pairs: Vec<String> = keys_arr.borrow().iter()
                            .filter_map(|k| {
                                let ks = k.to_display();
                                obj.fields.get(&ks).map(|v| format!("{}={}", ks, v.to_display()))
                            }).collect();
                        return format!("{{{}}}", pairs.join(", "));
                    }
                }
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
            UShr   => RVal::Int(((l.as_int() as u32) >> (r.as_int() & 31)) as i64),
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
            // char comparison: int code point vs single-char string
            (RVal::Int(a), RVal::Str(s)) if s.len() == 1 => {
                *a == s.chars().next().map(|c| c as i64).unwrap_or(-1)
            }
            (RVal::Str(s), RVal::Int(b)) if s.len() == 1 => {
                s.chars().next().map(|c| c as i64).unwrap_or(-1) == *b
            }
            // enum comparison: object with __name__ field vs string (for switch on enum)
            (RVal::Object(id), RVal::Str(s)) => {
                let heap = self.heap.borrow();
                heap.get(id)
                    .and_then(|o| o.fields.get("__name__"))
                    .map(|n| n.to_display() == *s)
                    .unwrap_or(false)
            }
            (RVal::Str(s), RVal::Object(id)) => {
                let heap = self.heap.borrow();
                heap.get(id)
                    .and_then(|o| o.fields.get("__name__"))
                    .map(|n| n.to_display() == *s)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
}
