//! Java reflection API stubs.

use rava_common::error::Result;
use crate::rir_interp::{RVal, ANNOTATION_REGISTRY};
use super::format::fnv;
use std::cell::RefCell;
use std::rc::Rc;

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

/// Register annotations for a class or member (called by the lowerer/interpreter setup).
pub fn register_annotations(key: &str, annotations: Vec<String>) {
    ANNOTATION_REGISTRY.with(|r| {
        r.borrow_mut().insert(key.to_string(), annotations);
    });
}

/// Instance methods on Class objects (represented as __class__Name strings).
pub fn dispatch_named(method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        "getName" | "getSimpleName" | "getCanonicalName" => {
            let cls = args.first().map(|v| v.to_display()).unwrap_or_default();
            let name = cls.strip_prefix("__class__").unwrap_or(&cls);
            Some(Ok(RVal::Str(name.to_string())))
        }
        "getDeclaredMethods" | "getMethods" | "getDeclaredFields" | "getFields" => {
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vec![])))))
        }
        "newInstance" => Some(Ok(RVal::Null)),
        "isInterface" | "isEnum" | "isRecord" | "isArray" | "isPrimitive" => Some(Ok(RVal::Bool(false))),
        "getModifiers" => Some(Ok(RVal::Int(1))), // PUBLIC
        "getAnnotations" | "getDeclaredAnnotations" => {
            let cls = args.first().map(|v| v.to_display()).unwrap_or_default();
            let key = cls.strip_prefix("__class__").unwrap_or(&cls);
            let annotations = ANNOTATION_REGISTRY.with(|r| {
                r.borrow().get(key).cloned().unwrap_or_default()
            });
            let vals: Vec<RVal> = annotations.into_iter()
                .map(|name| RVal::Str(format!("@{}", name)))
                .collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(vals)))))
        }
        "getAnnotation" | "getDeclaredAnnotation" => {
            let cls = args.first().map(|v| v.to_display()).unwrap_or_default();
            let ann_type = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            let key = cls.strip_prefix("__class__").unwrap_or(&cls);
            let ann_name = ann_type.strip_prefix("__class__").unwrap_or(&ann_type);
            let found = ANNOTATION_REGISTRY.with(|r| {
                r.borrow().get(key)
                    .map(|anns| anns.iter().any(|a| a == ann_name))
                    .unwrap_or(false)
            });
            if found {
                Some(Ok(RVal::Str(format!("@{}", ann_name))))
            } else {
                Some(Ok(RVal::Null))
            }
        }
        "isAnnotationPresent" => {
            let cls = args.first().map(|v| v.to_display()).unwrap_or_default();
            let ann_type = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            let key = cls.strip_prefix("__class__").unwrap_or(&cls);
            let ann_name = ann_type.strip_prefix("__class__").unwrap_or(&ann_type);
            let found = ANNOTATION_REGISTRY.with(|r| {
                r.borrow().get(key)
                    .map(|anns| anns.iter().any(|a| a == ann_name))
                    .unwrap_or(false)
            });
            Some(Ok(RVal::Bool(found)))
        }
        _ => None,
    }
}

