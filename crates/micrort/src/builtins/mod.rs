//! Built-in Java method dispatch for the RIR interpreter.
//!
//! Organized by domain:
//!   format      — format_java_string, fnv hash, rand_f64
//!   math        — java.lang.Math
//!   numbers     — Integer, Long, Double, Float, Byte, Short, Boolean, Character
//!   string      — String instance + static methods
//!   system      — System.out, System.exit, System.arraycopy, etc.
//!   collections — ArrayList, Collections, Arrays, List/Set/Map factories
//!   time        — java.time (LocalDate, LocalTime, LocalDateTime, Instant, Duration, Period)
//!   io          — java.io / java.nio.file
//!   concurrent  — Thread, Atomic*, Lock stubs
//!   reflect     — Class.forName, reflection stubs
//!   network     — java.net stubs
//!   regex       — java.util.regex.Pattern and Matcher
//!   scanner     — java.util.Scanner

pub mod collections;
pub mod concurrent;
pub mod format;
pub mod io;
pub mod math;
pub mod network;
pub mod numbers;
pub mod reflect;
pub mod regex;
pub mod scanner;
pub mod string;
pub mod system;
pub mod time;

use crate::rir_interp::RVal;
use rava_common::error::Result;
use std::cell::Cell;
use std::rc::Rc;

pub use collections::rval_cmp;
pub use format::format_java_string;

/// Dispatch a static/free builtin call by hashed func_id.
pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    system::dispatch(func_id, args)
        .or_else(|| math::dispatch(func_id, args))
        .or_else(|| numbers::dispatch(func_id, args))
        .or_else(|| string::dispatch_static(func_id, args))
        .or_else(|| collections::dispatch(func_id, args))
        .or_else(|| time::dispatch(func_id, args))
        .or_else(|| io::dispatch(func_id, args))
        .or_else(|| concurrent::dispatch(func_id, args))
        .or_else(|| reflect::dispatch(func_id, args))
        .or_else(|| network::dispatch(func_id, args))
        .or_else(|| regex::dispatch(func_id, args))
        .or_else(|| scanner::dispatch(func_id, args))
}

/// Dispatch an instance method call on a receiver value (unnamed).
pub fn dispatch_method(receiver: &RVal, args: &[RVal]) -> Option<Result<RVal>> {
    let _ = (receiver, args);
    None
}

/// Dispatch a named instance method on a receiver value.
pub fn dispatch_named_method(receiver: &RVal, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match receiver {
        RVal::Str(s) => {
            // Class objects are encoded as "__class__ClassName" strings
            if s.starts_with("__class__") {
                let class_name = s.strip_prefix("__class__").unwrap_or(s);
                return match method {
                    "getName" | "getCanonicalName" => Some(Ok(RVal::Str(class_name.to_string()))),
                    "getSimpleName" => {
                        let simple = class_name.rsplit('.').next().unwrap_or(class_name);
                        Some(Ok(RVal::Str(simple.to_string())))
                    }
                    "getDeclaredMethods" | "getMethods" | "getDeclaredFields" | "getFields" => {
                        Some(Ok(RVal::Array(Rc::new(std::cell::RefCell::new(vec![])))))
                    }
                    "isInterface" | "isEnum" | "isArray" | "isPrimitive" => {
                        Some(Ok(RVal::Bool(false)))
                    }
                    _ => None,
                };
            }
            // java.time objects
            if s.starts_with("__date__")
                || s.starts_with("__time__")
                || s.starts_with("__datetime__")
                || s.starts_with("__instant__")
                || s.starts_with("__duration__")
                || s.starts_with("__period__")
            {
                return time::dispatch_named(s, method, args);
            }
            // Pattern objects
            if s.starts_with("__pattern__") {
                return regex::dispatch_pattern(s, method, args);
            }
            // Matcher objects
            if s.starts_with("__matcher__") {
                return regex::dispatch_matcher(s, method, args);
            }
            // Scanner objects
            if s.starts_with("__scanner__") {
                return scanner::dispatch_scanner(s, method, args);
            }
            string::dispatch_named(s, method, args)
        }
        RVal::Array(arr) => collections::dispatch_array_named(arr, method, args),
        RVal::ArrayIter(arr, idx) => dispatch_array_iter(arr, idx, method),
        RVal::Int(n) => match method {
            "compareTo" => {
                let other = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(RVal::Int(n.cmp(&other) as i64)))
            }
            "intValue" | "longValue" | "shortValue" | "byteValue" => Some(Ok(RVal::Int(*n))),
            "doubleValue" | "floatValue" => Some(Ok(RVal::Float(*n as f64))),
            "booleanValue" => Some(Ok(RVal::Bool(*n != 0))),
            "toString" => Some(Ok(RVal::Str(n.to_string()))),
            _ => None,
        },
        RVal::Float(f) => match method {
            "compareTo" => {
                let other = args.first().map(|v| v.as_float()).unwrap_or(0.0);
                Some(Ok(RVal::Int(
                    f.partial_cmp(&other).map(|o| o as i64).unwrap_or(0),
                )))
            }
            "doubleValue" | "floatValue" => Some(Ok(RVal::Float(*f))),
            "intValue" | "longValue" | "shortValue" | "byteValue" => Some(Ok(RVal::Int(*f as i64))),
            "toString" => Some(Ok(RVal::Str(f.to_string()))),
            _ => None,
        },
        RVal::Object(_) => concurrent::dispatch_named(method, args)
            .or_else(|| reflect::dispatch_named(method, args)),
        _ => None,
    }
}

fn dispatch_array_iter(
    arr: &Rc<std::cell::RefCell<Vec<RVal>>>,
    idx: &Rc<Cell<usize>>,
    method: &str,
) -> Option<Result<RVal>> {
    match method {
        "hasNext" => Some(Ok(RVal::Bool(idx.get() < arr.borrow().len()))),
        "next" => {
            let i = idx.get();
            let val = arr.borrow().get(i).cloned().unwrap_or(RVal::Null);
            idx.set(i + 1);
            Some(Ok(val))
        }
        "remove" => {
            // Remove the last element returned by next()
            let i = idx.get();
            if i > 0 && i <= arr.borrow().len() {
                arr.borrow_mut().remove(i - 1);
                idx.set(i - 1);
            }
            Some(Ok(RVal::Void))
        }
        _ => None,
    }
}
