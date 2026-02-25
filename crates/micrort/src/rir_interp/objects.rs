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
            "forEach" => {
                // forEach((k, v) -> ...)
                let pairs: Vec<(String, RVal)> = {
                    let heap = self.heap.borrow();
                    if let Some(obj) = heap.get(&id) {
                        obj.fields.iter()
                            .filter(|(k, _)| !k.starts_with("__"))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    } else { vec![] }
                };
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                for (k, v) in pairs {
                    match self.invoke_lambda(&lambda, &[RVal::Str(k), v]) {
                        Ok(_) => {}
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Void))
            }
            "replaceAll" => {
                // replaceAll((k, v) -> newV)
                let pairs: Vec<(String, RVal)> = {
                    let heap = self.heap.borrow();
                    if let Some(obj) = heap.get(&id) {
                        obj.fields.iter()
                            .filter(|(k, _)| !k.starts_with("__"))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    } else { vec![] }
                };
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                for (k, v) in pairs {
                    let new_v = match self.invoke_lambda(&lambda, &[RVal::Str(k.clone()), v]) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
                    let mut heap = self.heap.borrow_mut();
                    if let Some(obj) = heap.get_mut(&id) {
                        obj.fields.insert(k, new_v);
                    }
                }
                Some(Ok(RVal::Void))
            }
            "merge" => {
                // merge(key, value, remappingFunction)
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let new_val = args.get(1).cloned().unwrap_or(RVal::Null);
                let func = args.get(2).cloned();
                let old_val = {
                    let heap = self.heap.borrow();
                    heap.get(&id).and_then(|o| o.fields.get(&key).cloned())
                };
                let result = match (old_val, func) {
                    (Some(old), Some(f)) if !matches!(old, RVal::Null) => {
                        match self.invoke_lambda(&f, &[old, new_val]) {
                            Ok(v) => v,
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    _ => new_val,
                };
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.insert(key, result.clone());
                }
                Some(Ok(result))
            }
            "computeIfAbsent" => {
                // computeIfAbsent(key, mappingFunction)
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let existing = {
                    let heap = self.heap.borrow();
                    heap.get(&id).and_then(|o| o.fields.get(&key).cloned())
                };
                if let Some(v) = existing {
                    return Some(Ok(v));
                }
                let func = args.get(1).cloned().unwrap_or(RVal::Null);
                let computed = match self.invoke_lambda(&func, &[RVal::Str(key.clone())]) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.insert(key, computed.clone());
                }
                Some(Ok(computed))
            }
            "compute" => {
                // compute(key, remappingFunction(key, oldVal))
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let old_val = {
                    let heap = self.heap.borrow();
                    heap.get(&id).and_then(|o| o.fields.get(&key).cloned()).unwrap_or(RVal::Null)
                };
                let func = args.get(1).cloned().unwrap_or(RVal::Null);
                let computed = match self.invoke_lambda(&func, &[RVal::Str(key.clone()), old_val]) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.insert(key, computed.clone());
                }
                Some(Ok(computed))
            }
            "putIfAbsent" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let val = args.get(1).cloned().unwrap_or(RVal::Null);
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    if !obj.fields.contains_key(&key) {
                        obj.fields.insert(key, val);
                        return Some(Ok(RVal::Null));
                    }
                    return Some(Ok(obj.fields.get(&key).cloned().unwrap_or(RVal::Null)));
                }
                Some(Ok(RVal::Null))
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
            "collect" => {
                let arr = self.as_array(receiver)?;
                if let Some(RVal::Object(cid)) = args.first() {
                    let (ctype, delim, lambda, key_fn, val_fn) = {
                        let heap = self.heap.borrow();
                        if let Some(cobj) = heap.get(cid) {
                            let ctype = cobj.fields.get("__ctype__").map(|v| v.to_display()).unwrap_or_default();
                            let delim = cobj.fields.get("__delim__").map(|v| v.to_display()).unwrap_or_default();
                            let lambda = cobj.fields.get("__lambda__").cloned();
                            let key_fn = cobj.fields.get("__keyfn__").cloned();
                            let val_fn = cobj.fields.get("__valfn__").cloned();
                            (ctype, delim, lambda, key_fn, val_fn)
                        } else { (String::new(), String::new(), None, None, None) }
                    };
                    match ctype.as_str() {
                        "joining" => {
                            let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(&delim);
                            return Some(Ok(RVal::Str(s)));
                        }
                        "toList" => return Some(Ok(receiver.clone())),
                        "toSet" => {
                            let mut seen = std::collections::HashSet::new();
                            let deduped: Vec<RVal> = arr.borrow().iter()
                                .filter(|v| seen.insert(v.to_display()))
                                .cloned().collect();
                            return Some(Ok(RVal::Array(Rc::new(RefCell::new(deduped)))));
                        }
                        "groupingBy" => {
                            let items = arr.borrow().clone();
                            let map_id = self.alloc_object("HashMap");
                            for item in &items {
                                let key = if let Some(ref lam) = lambda {
                                    match self.invoke_lambda(lam, &[item.clone()]) {
                                        Ok(k) => k.to_display(),
                                        Err(_) => continue,
                                    }
                                } else { item.to_display() };
                                let mut heap = self.heap.borrow_mut();
                                if let Some(map_obj) = heap.get_mut(&map_id) {
                                    let bucket = map_obj.fields.entry(key)
                                        .or_insert_with(|| RVal::Array(Rc::new(RefCell::new(vec![]))));
                                    if let RVal::Array(a) = bucket { a.borrow_mut().push(item.clone()); }
                                }
                            }
                            return Some(Ok(RVal::Object(map_id)));
                        }
                        "toMap" => {
                            let items = arr.borrow().clone();
                            let map_id = self.alloc_object("HashMap");
                            for item in &items {
                                let k = if let Some(ref kf) = key_fn {
                                    match self.invoke_lambda(kf, &[item.clone()]) {
                                        Ok(v) => v.to_display(),
                                        Err(_) => continue,
                                    }
                                } else { item.to_display() };
                                let v = if let Some(ref vf) = val_fn {
                                    match self.invoke_lambda(vf, &[item.clone()]) {
                                        Ok(v) => v,
                                        Err(_) => continue,
                                    }
                                } else { item.clone() };
                                let mut heap = self.heap.borrow_mut();
                                if let Some(map_obj) = heap.get_mut(&map_id) {
                                    map_obj.fields.insert(k, v);
                                }
                            }
                            return Some(Ok(RVal::Object(map_id)));
                        }
                        _ => {}
                    }
                }
                Some(Ok(receiver.clone()))
            }
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
                if let Some(comparator) = args.first() {
                    match comparator {
                        RVal::Str(s) if s == "__comparator__reverse__" => {
                            items.sort_by(|a, b| b.to_display().cmp(&a.to_display()));
                        }
                        RVal::Str(s) if s == "__comparator__natural__" => {
                            items.sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                        }
                        _ => {
                            // lambda comparator
                            let comp = comparator.clone();
                            let mut err = None;
                            items.sort_by(|a, b| {
                                if err.is_some() { return std::cmp::Ordering::Equal; }
                                match self.invoke_lambda(&comp, &[a.clone(), b.clone()]) {
                                    Ok(v) => v.as_int().cmp(&0),
                                    Err(e) => { err = Some(e); std::cmp::Ordering::Equal }
                                }
                            });
                            if let Some(e) = err { return Some(Err(e)); }
                        }
                    }
                } else {
                    // natural order: numeric if all ints, else string
                    let all_int = items.iter().all(|v| matches!(v, RVal::Int(_)));
                    if all_int {
                        items.sort_by(|a, b| a.as_int().cmp(&b.as_int()));
                    } else {
                        items.sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "flatMap" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow().clone();
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                let mut result = Vec::new();
                for item in &items {
                    match self.invoke_lambda(&lambda, &[item.clone()]) {
                        Ok(RVal::Array(a)) => result.extend(a.borrow().clone()),
                        Ok(v) => result.push(v),
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
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
            "peek" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow().clone();
                for item in &items {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(_) => {}
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "takeWhile" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                let mut result = Vec::new();
                for item in items.iter() {
                    match self.invoke_lambda(lambda, &[item.clone()]) {
                        Ok(v) => { if v.is_truthy() { result.push(item.clone()); } else { break; } }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
            }
            "dropWhile" => {
                let arr = self.as_array(receiver)?;
                let lambda = args.first()?;
                let items = arr.borrow();
                let mut dropping = true;
                let mut result = Vec::new();
                for item in items.iter() {
                    if dropping {
                        match self.invoke_lambda(lambda, &[item.clone()]) {
                            Ok(v) => { if !v.is_truthy() { dropping = false; result.push(item.clone()); } }
                            Err(e) => return Some(Err(e)),
                        }
                    } else {
                        result.push(item.clone());
                    }
                }
                Some(Ok(RVal::Array(Rc::new(RefCell::new(result)))))
            }
            "min" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                if items.is_empty() { return Some(Ok(RVal::Null)); }
                if let Some(comparator) = args.first() {
                    let comp = comparator.clone();
                    let mut min = items[0].clone();
                    for item in items.iter().skip(1) {
                        match self.invoke_lambda(&comp, &[item.clone(), min.clone()]) {
                            Ok(v) => { if v.as_int() < 0 { min = item.clone(); } }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    Some(Ok(min))
                } else {
                    let all_int = items.iter().all(|v| matches!(v, RVal::Int(_)));
                    if all_int {
                        Some(Ok(items.iter().min_by_key(|v| v.as_int()).cloned().unwrap_or(RVal::Null)))
                    } else {
                        Some(Ok(items.iter().min_by(|a, b| a.to_display().cmp(&b.to_display())).cloned().unwrap_or(RVal::Null)))
                    }
                }
            }
            "max" => {
                let arr = self.as_array(receiver)?;
                let items = arr.borrow();
                if items.is_empty() { return Some(Ok(RVal::Null)); }
                if let Some(comparator) = args.first() {
                    let comp = comparator.clone();
                    let mut max = items[0].clone();
                    for item in items.iter().skip(1) {
                        match self.invoke_lambda(&comp, &[item.clone(), max.clone()]) {
                            Ok(v) => { if v.as_int() > 0 { max = item.clone(); } }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    Some(Ok(max))
                } else {
                    let all_int = items.iter().all(|v| matches!(v, RVal::Int(_)));
                    if all_int {
                        Some(Ok(items.iter().max_by_key(|v| v.as_int()).cloned().unwrap_or(RVal::Null)))
                    } else {
                        Some(Ok(items.iter().max_by(|a, b| a.to_display().cmp(&b.to_display())).cloned().unwrap_or(RVal::Null)))
                    }
                }
            }
            // Deque / LinkedList front/back operations
            "addFirst" | "offerFirst" | "push" => {
                let arr = self.as_array(receiver)?;
                let val = args.first().cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().insert(0, val);
                Some(Ok(RVal::Void))
            }
            "addLast" | "offerLast" => {
                let arr = self.as_array(receiver)?;
                let val = args.first().cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().push(val);
                Some(Ok(RVal::Void))
            }
            "getFirst" | "peekFirst" | "element" => {
                let arr = self.as_array(receiver)?;
                Some(Ok(arr.borrow().first().cloned().unwrap_or(RVal::Null)))
            }
            "getLast" | "peekLast" => {
                let arr = self.as_array(receiver)?;
                Some(Ok(arr.borrow().last().cloned().unwrap_or(RVal::Null)))
            }
            "removeFirst" | "pollFirst" | "pop" => {
                let arr = self.as_array(receiver)?;
                let mut b = arr.borrow_mut();
                if b.is_empty() { Some(Ok(RVal::Null)) } else { Some(Ok(b.remove(0))) }
            }
            "removeLast" | "pollLast" => {
                let arr = self.as_array(receiver)?;
                let mut b = arr.borrow_mut();
                Some(Ok(b.pop().unwrap_or(RVal::Null)))
            }
            _ => None,
        }
    }

    /// Extract the inner Rc<RefCell<Vec<RVal>>> from an Array value.
    pub(super) fn as_array<'b>(&self, val: &'b RVal) -> Option<&'b Rc<RefCell<Vec<RVal>>>> {
        if let RVal::Array(arr) = val { Some(arr) } else { None }
    }

    // ── HashSet / TreeSet methods ─────────────────────────────────────────────

    pub(super) fn dispatch_set(&self, id: ObjId, class_name: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        let sorted = class_name == "TreeSet";
        match method {
            "add" => {
                let val = args.first().cloned().unwrap_or(RVal::Null);
                let key = val.to_display();
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    if let Some(RVal::Array(arr)) = obj.fields.get("__items__") {
                        let arr = arr.clone(); // clone Rc to release heap borrow
                        drop(heap);
                        let already = arr.borrow().iter().any(|v| v.to_display() == key);
                        if !already {
                            arr.borrow_mut().push(val);
                            if sorted {
                                arr.borrow_mut().sort_by(|a, b| crate::builtins::rval_cmp(a, b));
                            }
                        }
                        return Some(Ok(RVal::Bool(!already)));
                    }
                }
                Some(Ok(RVal::Bool(false)))
            }
            "remove" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    if let Some(RVal::Array(arr)) = obj.fields.get("__items__") {
                        let mut v = arr.borrow_mut();
                        if let Some(pos) = v.iter().position(|x| x.to_display() == key) {
                            v.remove(pos);
                            return Some(Ok(RVal::Bool(true)));
                        }
                    }
                }
                Some(Ok(RVal::Bool(false)))
            }
            "contains" => {
                let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                let heap = self.heap.borrow();
                let found = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .map(|v| if let RVal::Array(a) = v { a.borrow().iter().any(|x| x.to_display() == key) } else { false })
                    .unwrap_or(false);
                Some(Ok(RVal::Bool(found)))
            }
            "size" => {
                let heap = self.heap.borrow();
                let n = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .map(|v| if let RVal::Array(a) = v { a.borrow().len() as i64 } else { 0 })
                    .unwrap_or(0);
                Some(Ok(RVal::Int(n)))
            }
            "isEmpty" => {
                let heap = self.heap.borrow();
                let empty = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .map(|v| if let RVal::Array(a) = v { a.borrow().is_empty() } else { true })
                    .unwrap_or(true);
                Some(Ok(RVal::Bool(empty)))
            }
            "first" => {
                let heap = self.heap.borrow();
                let val = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .and_then(|v| if let RVal::Array(a) = v { a.borrow().first().cloned() } else { None })
                    .unwrap_or(RVal::Null);
                Some(Ok(val))
            }
            "last" => {
                let heap = self.heap.borrow();
                let val = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .and_then(|v| if let RVal::Array(a) = v { a.borrow().last().cloned() } else { None })
                    .unwrap_or(RVal::Null);
                Some(Ok(val))
            }
            "clear" => {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.insert("__items__".into(), RVal::Array(Rc::new(RefCell::new(vec![]))));
                }
                Some(Ok(RVal::Void))
            }
            "iterator" | "stream" | "toList" => {
                let heap = self.heap.borrow();
                let items = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .map(|v| if let RVal::Array(a) = v { a.borrow().clone() } else { vec![] })
                    .unwrap_or_default();
                Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
            }
            "forEach" => {
                let items = {
                    let heap = self.heap.borrow();
                    heap.get(&id)
                        .and_then(|o| o.fields.get("__items__"))
                        .map(|v| if let RVal::Array(a) = v { a.borrow().clone() } else { vec![] })
                        .unwrap_or_default()
                };
                let lambda = args.first().cloned().unwrap_or(RVal::Null);
                for item in &items {
                    match self.invoke_lambda(&lambda, &[item.clone()]) {
                        Ok(_) => {}
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(RVal::Void))
            }
            "toString" => {
                let heap = self.heap.borrow();
                let s = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .map(|v| if let RVal::Array(a) = v {
                        let items: Vec<String> = a.borrow().iter().map(|x| x.to_display()).collect();
                        format!("[{}]", items.join(", "))
                    } else { "[]".into() })
                    .unwrap_or_else(|| "[]".into());
                Some(Ok(RVal::Str(s)))
            }
            _ => None,
        }
    }

    pub(super) fn dispatch_priority_queue(&self, id: ObjId, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
        match method {
            "offer" | "add" => {
                let val = args.first().cloned().unwrap_or(RVal::Null);
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    let items = obj.fields.entry("__items__".into()).or_insert(RVal::Array(Rc::new(RefCell::new(vec![]))));
                    if let RVal::Array(arr) = items {
                        arr.borrow_mut().push(val);
                        arr.borrow_mut().sort_by(|a, b| crate::builtins::rval_cmp(a, b));
                    }
                }
                Some(Ok(RVal::Bool(true)))
            }
            "poll" | "remove" => {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    if let Some(RVal::Array(arr)) = obj.fields.get("__items__") {
                        let mut v = arr.borrow_mut();
                        if !v.is_empty() { return Some(Ok(v.remove(0))); }
                    }
                }
                Some(Ok(RVal::Null))
            }
            "peek" => {
                let heap = self.heap.borrow();
                if let Some(obj) = heap.get(&id) {
                    if let Some(RVal::Array(arr)) = obj.fields.get("__items__") {
                        return Some(Ok(arr.borrow().first().cloned().unwrap_or(RVal::Null)));
                    }
                }
                Some(Ok(RVal::Null))
            }
            "size" => {
                let heap = self.heap.borrow();
                let n = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .and_then(|v| if let RVal::Array(a) = v { Some(a.borrow().len() as i64) } else { None })
                    .unwrap_or(0);
                Some(Ok(RVal::Int(n)))
            }
            "isEmpty" => {
                let heap = self.heap.borrow();
                let empty = heap.get(&id)
                    .and_then(|o| o.fields.get("__items__"))
                    .and_then(|v| if let RVal::Array(a) = v { Some(a.borrow().is_empty()) } else { None })
                    .unwrap_or(true);
                Some(Ok(RVal::Bool(empty)))
            }
            "clear" => {
                let mut heap = self.heap.borrow_mut();
                if let Some(obj) = heap.get_mut(&id) {
                    obj.fields.insert("__items__".into(), RVal::Array(Rc::new(RefCell::new(vec![]))));
                }
                Some(Ok(RVal::Void))
            }
            _ => None,
        }
    }
}
