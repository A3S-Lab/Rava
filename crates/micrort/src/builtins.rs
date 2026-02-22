//! Built-in Java method dispatch for the RIR interpreter.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

fn fnv(name: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in name.bytes() { h ^= b as u32; h = h.wrapping_mul(16777619); }
    h
}

/// Dispatch a static/free builtin call by hashed func_id.
pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    // System.out
    if func_id == fnv("System.out.println") {
        println!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
        return Some(Ok(RVal::Void));
    }
    if func_id == fnv("System.out.print") {
        print!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
        return Some(Ok(RVal::Void));
    }
    if func_id == fnv("System.out.printf") || func_id == fnv("System.out.format") {
        let formatted = format_java_string(args);
        print!("{}", formatted);
        return Some(Ok(RVal::Void));
    }
    if func_id == fnv("System.err.println") {
        eprintln!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
        return Some(Ok(RVal::Void));
    }

    // String.valueOf
    if func_id == fnv("String.valueOf") {
        let s = args.first().map(|v| v.to_display()).unwrap_or_default();
        return Some(Ok(RVal::Str(s)));
    }
    // String.format
    if func_id == fnv("String.format") {
        return Some(Ok(RVal::Str(format_java_string(args))));
    }
    // String.join(delimiter, elements...)
    if func_id == fnv("String.join") {
        let delim = args.first().map(|v| v.to_display()).unwrap_or_default();
        // Second arg can be an array or individual strings
        if let Some(RVal::Array(arr)) = args.get(1) {
            let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(&delim);
            return Some(Ok(RVal::Str(s)));
        }
        let parts: Vec<String> = args[1..].iter().map(|v| v.to_display()).collect();
        return Some(Ok(RVal::Str(parts.join(&delim))));
    }

    // Integer / Long / Double parsing
    if func_id == fnv("Integer.parseInt") || func_id == fnv("Integer.valueOf") {
        let n = args.first().map(|v| v.to_display()).unwrap_or_default()
            .trim().parse::<i64>().unwrap_or(0);
        return Some(Ok(RVal::Int(n)));
    }
    if func_id == fnv("Long.parseLong") {
        let n = args.first().map(|v| v.to_display()).unwrap_or_default()
            .trim().parse::<i64>().unwrap_or(0);
        return Some(Ok(RVal::Int(n)));
    }
    if func_id == fnv("Double.parseDouble") {
        let f = args.first().map(|v| v.to_display()).unwrap_or_default()
            .trim().parse::<f64>().unwrap_or(0.0);
        return Some(Ok(RVal::Float(f)));
    }
    if func_id == fnv("Integer.toString") || func_id == fnv("String.valueOf") {
        let s = args.first().map(|v| v.to_display()).unwrap_or_default();
        return Some(Ok(RVal::Str(s)));
    }

    // Math
    if func_id == fnv("Math.max") {
        let a = args.first().map(|v| v.as_int()).unwrap_or(0);
        let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
        return Some(Ok(RVal::Int(a.max(b))));
    }
    if func_id == fnv("Math.min") {
        let a = args.first().map(|v| v.as_int()).unwrap_or(0);
        let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
        return Some(Ok(RVal::Int(a.min(b))));
    }
    if func_id == fnv("Math.abs") {
        let a = args.first().map(|v| v.as_int()).unwrap_or(0);
        return Some(Ok(RVal::Int(a.abs())));
    }
    if func_id == fnv("Math.pow") {
        let base = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        let exp  = args.get(1).map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(base.powf(exp))));
    }
    if func_id == fnv("Math.sqrt") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.sqrt())));
    }
    if func_id == fnv("Math.floor") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.floor())));
    }
    if func_id == fnv("Math.ceil") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.ceil())));
    }
    if func_id == fnv("Math.random") {
        let r: f64 = rand_f64();
        return Some(Ok(RVal::Float(r)));
    }
    if func_id == fnv("Math.round") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Int(a.round() as i64)));
    }
    if func_id == fnv("Math.log") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.ln())));
    }
    if func_id == fnv("Math.sin") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.sin())));
    }
    if func_id == fnv("Math.cos") {
        let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
        return Some(Ok(RVal::Float(a.cos())));
    }

    // System utilities
    if func_id == fnv("System.exit") {
        let code = args.first().map(|v| v.as_int()).unwrap_or(0);
        std::process::exit(code as i32);
    }
    if func_id == fnv("System.currentTimeMillis") {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        return Some(Ok(RVal::Int(ms)));
    }
    if func_id == fnv("System.nanoTime") {
        // Monotonic clock approximation
        let ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        return Some(Ok(RVal::Int(ns)));
    }

    // ArrayList constructor
    if func_id == fnv("ArrayList") || func_id == fnv("ArrayList.<init>") {
        let arr = Rc::new(RefCell::new(Vec::<RVal>::new()));
        return Some(Ok(RVal::Array(arr)));
    }

    // List.of / Arrays.asList — create an immutable-ish list from args
    if func_id == fnv("List.of") || func_id == fnv("Arrays.asList") {
        let arr = Rc::new(RefCell::new(args.to_vec()));
        return Some(Ok(RVal::Array(arr)));
    }
    // Arrays.sort — sort an array in-place
    if func_id == fnv("Arrays.sort") {
        if let Some(RVal::Array(arr)) = args.first() {
            arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b));
        }
        return Some(Ok(RVal::Void));
    }
    // Arrays.toString — "[1, 2, 3]"
    if func_id == fnv("Arrays.toString") {
        if let Some(RVal::Array(arr)) = args.first() {
            let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(", ");
            return Some(Ok(RVal::Str(format!("[{}]", s))));
        }
        return Some(Ok(RVal::Str("[]".into())));
    }
    // Arrays.copyOf
    if func_id == fnv("Arrays.copyOf") {
        if let Some(RVal::Array(arr)) = args.first() {
            let len = args.get(1).map(|v| v.as_int()).unwrap_or(0) as usize;
            let v = arr.borrow();
            let mut copy: Vec<RVal> = v.iter().take(len).cloned().collect();
            while copy.len() < len { copy.push(RVal::Int(0)); }
            return Some(Ok(RVal::Array(Rc::new(RefCell::new(copy)))));
        }
        return Some(Ok(RVal::Null));
    }
    // Arrays.fill
    if func_id == fnv("Arrays.fill") {
        if let Some(RVal::Array(arr)) = args.first() {
            let val = args.get(1).cloned().unwrap_or(RVal::Null);
            let mut v = arr.borrow_mut();
            for elem in v.iter_mut() { *elem = val.clone(); }
        }
        return Some(Ok(RVal::Void));
    }
    // Arrays.equals
    if func_id == fnv("Arrays.equals") {
        if let (Some(RVal::Array(a)), Some(RVal::Array(b))) = (args.first(), args.get(1)) {
            let eq = a.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>()
                == b.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>();
            return Some(Ok(RVal::Bool(eq)));
        }
        return Some(Ok(RVal::Bool(false)));
    }

    // Collections.sort — sort an array in-place (natural order)
    if func_id == fnv("Collections.sort") {
        if let Some(RVal::Array(arr)) = args.first() {
            arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b));
            return Some(Ok(RVal::Void));
        }
        return Some(Ok(RVal::Void));
    }
    // Collections.reverse
    if func_id == fnv("Collections.reverse") {
        if let Some(RVal::Array(arr)) = args.first() {
            arr.borrow_mut().reverse();
        }
        return Some(Ok(RVal::Void));
    }
    // Collections.min / Collections.max
    if func_id == fnv("Collections.min") {
        if let Some(RVal::Array(arr)) = args.first() {
            let v = arr.borrow();
            let min = v.iter().min_by(|a, b| rval_cmp(a, b)).cloned().unwrap_or(RVal::Null);
            return Some(Ok(min));
        }
        return Some(Ok(RVal::Null));
    }
    if func_id == fnv("Collections.max") {
        if let Some(RVal::Array(arr)) = args.first() {
            let v = arr.borrow();
            let max = v.iter().max_by(|a, b| rval_cmp(a, b)).cloned().unwrap_or(RVal::Null);
            return Some(Ok(max));
        }
        return Some(Ok(RVal::Null));
    }
    // Collections.unmodifiableList / Collections.emptyList
    if func_id == fnv("Collections.unmodifiableList") {
        return Some(Ok(args.first().cloned().unwrap_or(RVal::Null)));
    }
    if func_id == fnv("Collections.emptyList") {
        return Some(Ok(RVal::Array(Rc::new(RefCell::new(Vec::new())))));
    }

    // HashMap constructor — represented as Object with special class
    if func_id == fnv("HashMap") || func_id == fnv("HashMap.<init>") {
        // HashMap is handled at the interpreter level via alloc_object
        // Return Void here so the interpreter's New + Call flow handles it
        return None;
    }

    // StringBuilder constructor
    if func_id == fnv("StringBuilder") || func_id == fnv("StringBuilder.<init>") {
        // StringBuilder is handled at the interpreter level via alloc_object
        return None;
    }

    None
}

/// Dispatch an instance method call on a receiver value.
/// Called for `CallVirtual` and dotted method calls.
pub fn dispatch_method(receiver: &RVal, args: &[RVal]) -> Option<Result<RVal>> {
    match receiver {
        RVal::Str(s) => dispatch_string(s, args),
        RVal::Array(arr) => dispatch_array(arr, args),
        _ => None,
    }
}

/// Dispatch a named instance method (used when we know the method name).
pub fn dispatch_named_method(receiver: &RVal, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match receiver {
        RVal::Str(s) => dispatch_string_named(s, method, args),
        RVal::Array(arr) => dispatch_array_named(arr, method, args),
        RVal::ArrayIter(arr, idx) => dispatch_array_iter(arr, idx, method),
        _ => None,
    }
}

// ── String methods ────────────────────────────────────────────────────────────

fn dispatch_string(s: &str, args: &[RVal]) -> Option<Result<RVal>> {
    // Without method name we can't dispatch — needs named dispatch
    let _ = (s, args);
    None
}

fn dispatch_string_named(s: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        "length"      => Some(Ok(RVal::Int(s.len() as i64))),
        "isEmpty"     => Some(Ok(RVal::Bool(s.is_empty()))),
        "toUpperCase" => Some(Ok(RVal::Str(s.to_uppercase()))),
        "toLowerCase" => Some(Ok(RVal::Str(s.to_lowercase()))),
        "trim"        => Some(Ok(RVal::Str(s.trim().to_string()))),
        "charAt" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let c = s.chars().nth(i).unwrap_or('\0');
            Some(Ok(RVal::Int(c as i64)))
        }
        "substring" => {
            let start = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let end   = args.get(1).map(|v| v.as_int()).unwrap_or(s.len() as i64) as usize;
            let sub   = s.get(start..end.min(s.len())).unwrap_or("").to_string();
            Some(Ok(RVal::Str(sub)))
        }
        "contains" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s.contains(pat.as_str()))))
        }
        "startsWith" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s.starts_with(pat.as_str()))))
        }
        "endsWith" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s.ends_with(pat.as_str()))))
        }
        "equals" | "equalsIgnoreCase" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            let eq = if method == "equalsIgnoreCase" {
                s.to_lowercase() == other.to_lowercase()
            } else {
                s == other.as_str()
            };
            Some(Ok(RVal::Bool(eq)))
        }
        "replace" => {
            let from = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(s.replace(from.as_str(), to.as_str()))))
        }
        "indexOf" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let idx = s.find(pat.as_str()).map(|i| i as i64).unwrap_or(-1);
            Some(Ok(RVal::Int(idx)))
        }
        "split" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let parts: Vec<RVal> = s.split(pat.as_str())
                .map(|p| RVal::Str(p.to_string()))
                .collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(parts)))))
        }
        "toString" => Some(Ok(RVal::Str(s.to_string()))),
        "compareTo" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(s.cmp(&other.as_str()) as i64)))
        }
        "hashCode" => {
            let mut h: i32 = 0;
            for c in s.chars() { h = h.wrapping_mul(31).wrapping_add(c as i32); }
            Some(Ok(RVal::Int(h as i64)))
        }
        "toCharArray" => {
            let chars: Vec<RVal> = s.chars().map(|c| RVal::Int(c as i64)).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(chars)))))
        }
        "matches" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let matched = regex_lite_match(&pat, s);
            Some(Ok(RVal::Bool(matched)))
        }
        "replaceAll" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            // Simple: treat as literal replace for now
            Some(Ok(RVal::Str(s.replace(pat.as_str(), to.as_str()))))
        }
        "lastIndexOf" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let idx = s.rfind(pat.as_str()).map(|i| i as i64).unwrap_or(-1);
            Some(Ok(RVal::Int(idx)))
        }
        _ => None,
    }
}

// ── Array / ArrayList methods ─────────────────────────────────────────────────

fn dispatch_array(arr: &Rc<RefCell<Vec<RVal>>>, args: &[RVal]) -> Option<Result<RVal>> {
    let _ = (arr, args);
    None
}

fn dispatch_array_named(arr: &Rc<RefCell<Vec<RVal>>>, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        "size" | "length" => Some(Ok(RVal::Int(arr.borrow().len() as i64))),
        "isEmpty"         => Some(Ok(RVal::Bool(arr.borrow().is_empty()))),
        "add" => {
            let val = args.first().cloned().unwrap_or(RVal::Null);
            arr.borrow_mut().push(val);
            Some(Ok(RVal::Bool(true)))
        }
        "get" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let val = arr.borrow().get(i).cloned().unwrap_or(RVal::Null);
            Some(Ok(val))
        }
        "set" => {
            let i   = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let val = args.get(1).cloned().unwrap_or(RVal::Null);
            let old = {
                let mut b = arr.borrow_mut();
                if i < b.len() { let old = b[i].clone(); b[i] = val; old }
                else { RVal::Null }
            };
            Some(Ok(old))
        }
        "remove" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let val = {
                let mut b = arr.borrow_mut();
                if i < b.len() { b.remove(i) } else { RVal::Null }
            };
            Some(Ok(val))
        }
        "contains" => {
            let target = args.first().map(|v| v.to_display()).unwrap_or_default();
            let found = arr.borrow().iter().any(|v| v.to_display() == target);
            Some(Ok(RVal::Bool(found)))
        }
        "clear" => {
            arr.borrow_mut().clear();
            Some(Ok(RVal::Void))
        }
        "toString" => {
            let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(", ");
            Some(Ok(RVal::Str(format!("[{s}]"))))
        }
        "iterator" => {
            Some(Ok(RVal::ArrayIter(arr.clone(), Rc::new(Cell::new(0)))))
        }
        "sort" => {
            arr.borrow_mut().sort_by(|a, b| rval_cmp(a, b));
            Some(Ok(RVal::Void))
        }
        _ => None,
    }
}

// ── Comparison helper ─────────────────────────────────────────────────────────

/// Natural ordering for RVal (used by Collections.sort, Arrays.sort, etc.)
pub fn rval_cmp(a: &RVal, b: &RVal) -> Ordering {
    match (a, b) {
        (RVal::Int(x), RVal::Int(y)) => x.cmp(y),
        (RVal::Float(x), RVal::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (RVal::Int(x), RVal::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal),
        (RVal::Float(x), RVal::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal),
        _ => a.to_display().cmp(&b.to_display()),
    }
}

// ── ArrayIterator methods ────────────────────────────────────────────────────

fn dispatch_array_iter(arr: &Rc<RefCell<Vec<RVal>>>, idx: &Rc<Cell<usize>>, method: &str) -> Option<Result<RVal>> {
    match method {
        "hasNext" => {
            let has = idx.get() < arr.borrow().len();
            Some(Ok(RVal::Bool(has)))
        }
        "next" => {
            let i = idx.get();
            let val = arr.borrow().get(i).cloned().unwrap_or(RVal::Null);
            idx.set(i + 1);
            Some(Ok(val))
        }
        _ => None,
    }
}

// ── Java printf/format string support ────────────────────────────────────────

/// Format a Java-style format string with arguments.
/// Supports: %s, %d, %f, %n, %%, %b, %c, %x, %o.
pub fn format_java_string(args: &[RVal]) -> String {
    let fmt = args.first().map(|v| v.to_display()).unwrap_or_default();
    let mut result = String::new();
    let mut arg_idx = 1usize;
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() {
            i += 1;
            // Skip flags
            let mut flags = String::new();
            while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#') {
                flags.push(chars[i]);
                i += 1;
            }
            // Parse width
            let mut width: Option<usize> = None;
            let w_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
            if i > w_start {
                width = chars[w_start..i].iter().collect::<String>().parse().ok();
            }
            // Parse precision
            let mut precision: Option<usize> = None;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                let p_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
                precision = Some(chars[p_start..i].iter().collect::<String>().parse().unwrap_or(6));
            }
            if i >= chars.len() { break; }
            match chars[i] {
                's' => {
                    let val = args.get(arg_idx).map(|v| v.to_display()).unwrap_or_default();
                    if let Some(w) = width {
                        if flags.contains('-') {
                            result.push_str(&format!("{:<width$}", val, width = w));
                        } else {
                            result.push_str(&format!("{:>width$}", val, width = w));
                        }
                    } else {
                        result.push_str(&val);
                    }
                    arg_idx += 1;
                }
                'd' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    if let Some(w) = width {
                        if flags.contains('0') {
                            result.push_str(&format!("{:0>width$}", val, width = w));
                        } else {
                            result.push_str(&format!("{:width$}", val, width = w));
                        }
                    } else {
                        result.push_str(&val.to_string());
                    }
                    arg_idx += 1;
                }
                'f' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    result.push_str(&format!("{:.prec$}", val, prec = prec));
                    arg_idx += 1;
                }
                'b' => {
                    let val = args.get(arg_idx).map(|v| v.is_truthy()).unwrap_or(false);
                    result.push_str(&val.to_string());
                    arg_idx += 1;
                }
                'c' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    if let Some(c) = char::from_u32(val as u32) { result.push(c); }
                    arg_idx += 1;
                }
                'x' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    result.push_str(&format!("{:x}", val));
                    arg_idx += 1;
                }
                'o' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    result.push_str(&format!("{:o}", val));
                    arg_idx += 1;
                }
                'n' => result.push('\n'),
                '%' => result.push('%'),
                _ => { result.push('%'); result.push(chars[i]); }
            }
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }
    result
}

/// Simple pseudo-random f64 in [0.0, 1.0).
fn rand_f64() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::cell::Cell;
    thread_local! {
        static SEED: Cell<u64> = Cell::new(0);
    }
    SEED.with(|s| {
        let mut val = s.get();
        if val == 0 {
            let mut h = DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut h);
            val = h.finish();
        }
        // xorshift64
        val ^= val << 13;
        val ^= val >> 7;
        val ^= val << 17;
        s.set(val);
        (val as f64) / (u64::MAX as f64)
    })
}

/// Simple regex-like match for String.matches().
/// Supports basic patterns; falls back to exact match for complex regex.
fn regex_lite_match(pattern: &str, s: &str) -> bool {
    // Very basic: if pattern is a simple literal, do exact match
    // For common patterns like ".*", "[a-z]+", etc. — approximate
    if pattern == ".*" { return true; }
    if pattern.starts_with("^") && pattern.ends_with("$") {
        let inner = &pattern[1..pattern.len()-1];
        return s == inner;
    }
    s == pattern
}
