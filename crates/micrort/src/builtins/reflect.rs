//! Java reflection API stubs.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use super::format::fnv;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("Class.forName") => {
            let name = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("__class__{}", name))))
        }
        id if id == fnv("Class.getName") || id == fnv("Class.getSimpleName") => {
            let cls = args.first().map(|v| v.to_display()).unwrap_or_default();
            let name = cls.strip_prefix("__class__").unwrap_or(&cls);
            Some(Ok(RVal::Str(name.to_string())))
        }
        id if id == fnv("Class.newInstance") || id == fnv("Constructor.newInstance") => {
            Some(Ok(RVal::Null))
        }
        id if id == fnv("Class.isInstance") => {
            Some(Ok(RVal::Bool(true))) // stub
        }
        id if id == fnv("Class.isAssignableFrom") => {
            Some(Ok(RVal::Bool(true))) // stub
        }
        _ => None,
    }
}

/// Instance methods on Class objects (represented as __class__Name strings).
pub fn dispatch_named(method: &str, _args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        "getName" | "getSimpleName" | "getCanonicalName" => Some(Ok(RVal::Str(String::new()))),
        "getDeclaredMethods" | "getMethods" | "getDeclaredFields" | "getFields" => {
            use std::cell::RefCell;
            use std::rc::Rc;
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vec![])))))
        }
        "newInstance" => Some(Ok(RVal::Null)),
        "isInterface" | "isEnum" | "isRecord" | "isArray" | "isPrimitive" => Some(Ok(RVal::Bool(false))),
        "getModifiers" => Some(Ok(RVal::Int(1))), // PUBLIC
        _ => None,
    }
}
