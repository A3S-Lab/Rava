//! Built-in object method dispatch: StringBuilder, HashMap, and Stream operations.

use std::cell::RefCell;
use std::rc::Rc;
use rava_common::error::Result;
use super::rval::{ObjId, RVal};
use super::RirInterpreter;

impl RirInterpreter {
    // ── StringBuilder methods ─────────────────────────────────────────────────

    pub(super) fn dispatch_string_builder(&self, id: ObjId, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
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

    // ── HashMap methods ───────────────────────────────────────────────────────

    pub(super) fn dispatch_hash_map(&self, id: ObjId, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
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

    // ── Stream operations ─────────────────────────────────────────────────────

    /// Dispatch stream operations on arrays (eager evaluation model).
    pub(super) fn dispatch_stream(&self, receiver: &RVal, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        match method {
            "stream" | "toList" => Some(Ok(receiver.clone())),
            "of" => Some(Ok(RVal::Array(Rc::new(RefCell::new(args.to_vec()))))),
            "mapToInt" | "mapToLong" | "mapToDouble" | "map" => {
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
            "collect" => Some(Ok(receiver.clone())),
            "reduce" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                if items.is_empty() { return Some(Ok(RVal::Null)); }
                if args.len() >= 2 {
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
            "sum" => {
                let arr = self.as_array(receiver)?;
                let total: i64 = arr.borrow().iter().map(|v| v.as_int()).sum();
                Some(Ok(RVal::Int(total)))
            }
            "average" => {
                let arr = self.as_array(receiver)?;
                let v = arr.borrow();
                if v.is_empty() { return Some(Ok(RVal::Null)); }
                let total: f64 = v.iter().map(|x| x.as_float()).sum();
                Some(Ok(RVal::Float(total / v.len() as f64)))
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
    pub(super) fn as_array<'b>(&self, val: &'b RVal) -> Option<&'b Rc<RefCell<Vec<RVal>>>> {
        if let RVal::Array(arr) = val { Some(arr) } else { None }
    }
}
