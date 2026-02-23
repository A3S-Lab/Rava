//! Java Collections, Arrays, List, Set, Map factory methods.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::RefCell;
use std::rc::Rc;
use super::format::fnv;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        // ── ArrayList ─────────────────────────────────────────────────────────
        id if id == fnv("ArrayList") || id == fnv("ArrayList.<init>") => {
            // If arg is a collection, copy it
            if let Some(RVal::Array(src)) = args.first() {
                return Some(Ok(RVal::Array(Rc::new(RefCell::new(src.borrow().clone())))));
            }
            Some(Ok(RVal::Array(Rc::new(RefCell::new(Vec::new())))))
        }

        // ── List factory ──────────────────────────────────────────────────────
        id if id == fnv("List.of") || id == fnv("Arrays.asList") => {
            if args.len() == 1 {
                if let Some(RVal::Array(arr)) = args.first() {
                    return Some(Ok(RVal::Array(arr.clone())));
                }
            }
            Some(Ok(RVal::Array(Rc::new(RefCell::new(args.to_vec())))))
        }
        id if id == fnv("List.copyOf") => {
            if let Some(RVal::Array(arr)) = args.first() {
                return Some(Ok(RVal::Array(Rc::new(RefCell::new(arr.borrow().clone())))));
            }
            Some(Ok(RVal::Array(Rc::new(RefCell::new(Vec::new())))))
        }

        // ── Set factory ───────────────────────────────────────────────────────
        id if id == fnv("Set.of") || id == fnv("Set.copyOf") => {
            // Deduplicate
            let mut seen = std::collections::HashSet::new();
            let items: Vec<RVal> = args.iter().filter(|v| seen.insert(v.to_display())).cloned().collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(items)))))
        }

        // ── Map factory ───────────────────────────────────────────────────────
        id if id == fnv("Map.of") => {
            // Map.of(k1,v1, k2,v2, ...) — store as flat array, interpreter handles as HashMap
            Some(Ok(RVal::Array(Rc::new(RefCell::new(args.to_vec())))))
        }

        // ── Arrays ────────────────────────────────────────────────────────────
        id if id == fnv("Arrays.sort") => {
            if let Some(RVal::Array(arr)) = args.first() {
                arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b));
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Arrays.toString") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(", ");
                return Some(Ok(RVal::Str(format!("[{}]", s))));
            }
            Some(Ok(RVal::Str("[]".into())))
        }
        id if id == fnv("Arrays.deepToString") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(", ");
                return Some(Ok(RVal::Str(format!("[{}]", s))));
            }
            Some(Ok(RVal::Str("[]".into())))
        }
        id if id == fnv("Arrays.copyOf") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let len = args.get(1).map(|v| v.as_int()).unwrap_or(0) as usize;
                let v = arr.borrow();
                let mut copy: Vec<RVal> = v.iter().take(len).cloned().collect();
                while copy.len() < len { copy.push(RVal::Int(0)); }
                return Some(Ok(RVal::Array(Rc::new(RefCell::new(copy)))));
            }
            Some(Ok(RVal::Null))
        }
        id if id == fnv("Arrays.copyOfRange") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let from = args.get(1).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
                let to   = args.get(2).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
                let v = arr.borrow();
                let copy = v.get(from..to.min(v.len())).unwrap_or(&[]).to_vec();
                return Some(Ok(RVal::Array(Rc::new(RefCell::new(copy)))));
            }
            Some(Ok(RVal::Null))
        }
        id if id == fnv("Arrays.fill") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let val = args.get(1).cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().iter_mut().for_each(|e| *e = val.clone());
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Arrays.equals") => {
            if let (Some(RVal::Array(a)), Some(RVal::Array(b))) = (args.first(), args.get(1)) {
                let eq = a.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>()
                    == b.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>();
                return Some(Ok(RVal::Bool(eq)));
            }
            Some(Ok(RVal::Bool(false)))
        }
        id if id == fnv("Arrays.stream") => {
            if let Some(RVal::Array(arr)) = args.first() {
                return Some(Ok(RVal::Array(arr.clone())));
            }
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vec![])))))
        }

        // ── Collections ───────────────────────────────────────────────────────
        id if id == fnv("Collections.sort") => {
            if let Some(RVal::Array(arr)) = args.first() {
                arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b));
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.reverse") => {
            if let Some(RVal::Array(arr)) = args.first() { arr.borrow_mut().reverse(); }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.shuffle") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let mut v = arr.borrow_mut();
                let n = v.len();
                for i in (1..n).rev() {
                    let j = (super::format::rand_f64() * (i + 1) as f64) as usize;
                    v.swap(i, j);
                }
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.min") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let v = arr.borrow();
                return Some(Ok(v.iter().min_by(|a, b| rval_cmp(a, b)).cloned().unwrap_or(RVal::Null)));
            }
            Some(Ok(RVal::Null))
        }
        id if id == fnv("Collections.max") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let v = arr.borrow();
                return Some(Ok(v.iter().max_by(|a, b| rval_cmp(a, b)).cloned().unwrap_or(RVal::Null)));
            }
            Some(Ok(RVal::Null))
        }
        id if id == fnv("Collections.frequency") => {
            if let (Some(RVal::Array(arr)), Some(target)) = (args.first(), args.get(1)) {
                let t = target.to_display();
                return Some(Ok(RVal::Int(arr.borrow().iter().filter(|v| v.to_display() == t).count() as i64)));
            }
            Some(Ok(RVal::Int(0)))
        }
        id if id == fnv("Collections.binarySearch") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let target = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                let v = arr.borrow();
                return Some(Ok(RVal::Int(v.iter().position(|x| x.to_display() == target).map(|i| i as i64).unwrap_or(-1))));
            }
            Some(Ok(RVal::Int(-1)))
        }
        id if id == fnv("Collections.rotate") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let dist = args.get(1).map(|v| v.as_int()).unwrap_or(0);
                let mut v = arr.borrow_mut();
                let n = v.len();
                if n > 0 {
                    let d = ((dist % n as i64) + n as i64) as usize % n;
                    v.rotate_right(d);
                }
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.swap") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let i = args.get(1).map(|v| v.as_int()).unwrap_or(0) as usize;
                let j = args.get(2).map(|v| v.as_int()).unwrap_or(0) as usize;
                let mut v = arr.borrow_mut();
                if i < v.len() && j < v.len() { v.swap(i, j); }
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.fill") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let val = args.get(1).cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().iter_mut().for_each(|e| *e = val.clone());
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Collections.nCopies") => {
            let n   = args.first().map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
            let val = args.get(1).cloned().unwrap_or(RVal::Null);
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vec![val; n])))))
        }
        id if id == fnv("Collections.singletonList") || id == fnv("Collections.singleton") => {
            let v = args.first().cloned().unwrap_or(RVal::Null);
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vec![v])))))
        }
        id if id == fnv("Collections.emptyList") || id == fnv("Collections.emptySet") => {
            Some(Ok(RVal::Array(Rc::new(RefCell::new(Vec::new())))))
        }
        id if id == fnv("Collections.emptyMap") => Some(Ok(RVal::Null)),
        id if id == fnv("Collections.unmodifiableList")
            || id == fnv("Collections.unmodifiableSet")
            || id == fnv("Collections.unmodifiableMap")
            || id == fnv("Collections.synchronizedList")
            || id == fnv("Collections.synchronizedMap")
            || id == fnv("Collections.checkedList") =>
        {
            Some(Ok(args.first().cloned().unwrap_or(RVal::Null)))
        }
        id if id == fnv("Collections.disjoint") => {
            if let (Some(RVal::Array(a)), Some(RVal::Array(b))) = (args.first(), args.get(1)) {
                let set: std::collections::HashSet<String> = a.borrow().iter().map(|v| v.to_display()).collect();
                let disjoint = b.borrow().iter().all(|v| !set.contains(&v.to_display()));
                return Some(Ok(RVal::Bool(disjoint)));
            }
            Some(Ok(RVal::Bool(true)))
        }
        id if id == fnv("Collections.indexOfSubList") => {
            if let (Some(RVal::Array(src)), Some(RVal::Array(target))) = (args.first(), args.get(1)) {
                let s: Vec<_> = src.borrow().iter().map(|v| v.to_display()).collect();
                let t: Vec<_> = target.borrow().iter().map(|v| v.to_display()).collect();
                let result = s.windows(t.len()).position(|w| w == t.as_slice()).map(|i| i as i64).unwrap_or(-1);
                return Some(Ok(RVal::Int(result)));
            }
            Some(Ok(RVal::Int(-1)))
        }
        id if id == fnv("Collections.replaceAll") => {
            if let Some(RVal::Array(arr)) = args.first() {
                let old = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                let new = args.get(2).cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().iter_mut().for_each(|e| { if e.to_display() == old { *e = new.clone(); } });
            }
            Some(Ok(RVal::Bool(true)))
        }

        _ => None,
    }
}

/// Natural ordering for RVal.
pub fn rval_cmp(a: &RVal, b: &RVal) -> std::cmp::Ordering {
    match (a, b) {
        (RVal::Int(x), RVal::Int(y))     => x.cmp(y),
        (RVal::Float(x), RVal::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (RVal::Int(x), RVal::Float(y))   => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (RVal::Float(x), RVal::Int(y))   => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_display().cmp(&b.to_display()),
    }
}

/// ArrayList instance methods.
pub fn dispatch_array_named(arr: &Rc<RefCell<Vec<RVal>>>, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    use std::cell::Cell;
    match method {
        "size" | "length" => Some(Ok(RVal::Int(arr.borrow().len() as i64))),
        "isEmpty"         => Some(Ok(RVal::Bool(arr.borrow().is_empty()))),
        "add" => {
            if args.len() == 2 {
                // add(index, element)
                let i   = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                let val = args.get(1).cloned().unwrap_or(RVal::Null);
                let mut b = arr.borrow_mut();
                if i <= b.len() { b.insert(i, val); }
                Some(Ok(RVal::Void))
            } else {
                let val = args.first().cloned().unwrap_or(RVal::Null);
                arr.borrow_mut().push(val);
                Some(Ok(RVal::Bool(true)))
            }
        }
        "addAll" => {
            if let Some(RVal::Array(other)) = args.first() {
                let items = other.borrow().clone();
                arr.borrow_mut().extend(items);
            }
            Some(Ok(RVal::Bool(true)))
        }
        "get" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            Some(Ok(arr.borrow().get(i).cloned().unwrap_or(RVal::Null)))
        }
        "set" => {
            let i   = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let val = args.get(1).cloned().unwrap_or(RVal::Null);
            let mut b = arr.borrow_mut();
            let old = if i < b.len() { let o = b[i].clone(); b[i] = val; o } else { RVal::Null };
            Some(Ok(old))
        }
        "remove" => {
            let mut b = arr.borrow_mut();
            if let Some(RVal::Int(_)) = args.first() {
                let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
                if i < b.len() { return Some(Ok(b.remove(i))); }
            } else {
                let target = args.first().map(|v| v.to_display()).unwrap_or_default();
                if let Some(pos) = b.iter().position(|v| v.to_display() == target) {
                    b.remove(pos);
                    return Some(Ok(RVal::Bool(true)));
                }
                return Some(Ok(RVal::Bool(false)));
            }
            Some(Ok(RVal::Null))
        }
        "removeIf" => Some(Ok(RVal::Bool(false))), // lambda-based, handled by interpreter
        "contains" => {
            let target = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(arr.borrow().iter().any(|v| v.to_display() == target))))
        }
        "indexOf" => {
            let target = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(arr.borrow().iter().position(|v| v.to_display() == target).map(|i| i as i64).unwrap_or(-1))))
        }
        "lastIndexOf" => {
            let target = args.first().map(|v| v.to_display()).unwrap_or_default();
            let v = arr.borrow();
            Some(Ok(RVal::Int(v.iter().rposition(|v| v.to_display() == target).map(|i| i as i64).unwrap_or(-1))))
        }
        "clear"    => { arr.borrow_mut().clear(); Some(Ok(RVal::Void)) }
        "sort"     => { arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b)); Some(Ok(RVal::Void)) }
        "reverse"  => { arr.borrow_mut().reverse(); Some(Ok(RVal::Void)) }
        "toString" => {
            let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(", ");
            Some(Ok(RVal::Str(format!("[{s}]"))))
        }
        "toArray"  => Some(Ok(RVal::Array(arr.clone()))),
        "iterator" => Some(Ok(RVal::ArrayIter(arr.clone(), Rc::new(Cell::new(0))))),
        "stream"   => Some(Ok(RVal::Array(arr.clone()))),
        "subList"  => {
            let from = args.first().map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
            let to   = args.get(1).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
            let v = arr.borrow();
            let sub = v.get(from..to.min(v.len())).unwrap_or(&[]).to_vec();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(sub)))))
        }
        _ => None,
    }
}
