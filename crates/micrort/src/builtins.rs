//! Built-in Java method dispatch for the RIR interpreter.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::RefCell;
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

    // ArrayList constructor
    if func_id == fnv("ArrayList") || func_id == fnv("ArrayList.<init>") {
        let arr = Rc::new(RefCell::new(Vec::<RVal>::new()));
        return Some(Ok(RVal::Array(arr)));
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
            while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#') {
                i += 1;
            }
            // Skip width
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // Skip precision
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i >= chars.len() { break; }
            match chars[i] {
                's' => {
                    let val = args.get(arg_idx).map(|v| v.to_display()).unwrap_or_default();
                    result.push_str(&val);
                    arg_idx += 1;
                }
                'd' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    result.push_str(&val.to_string());
                    arg_idx += 1;
                }
                'f' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    result.push_str(&format!("{:.6}", val));
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
