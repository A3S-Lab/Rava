//! RIR interpreter — executes RIR directly with full Java semantics.
//!
//! Supports: all arithmetic (int + float), string operations, object fields,
//! arrays, static fields, user-defined methods, break/continue (via control flow),
//! ternary (via branching), type conversion, instanceof, try/catch (simplified).

use std::collections::HashMap;
use std::cell::RefCell;
use rava_rir::RirModule;

pub mod rval;
mod interp;
mod helpers;
mod objects;

pub use rval::{ObjId, JavaObject, RVal};

/// All known instance method names — used to reverse-lookup __method__<name> calls.
const KNOWN_METHODS: &[&str] = &[
    // String
    "length", "isEmpty", "toUpperCase", "toLowerCase", "trim",
    "charAt", "substring", "contains", "startsWith", "endsWith",
    "equals", "equalsIgnoreCase", "replace", "indexOf", "split",
    "toString", "hashCode", "compareTo", "toCharArray", "valueOf",
    "format", "join", "strip", "stripLeading", "stripTrailing",
    "repeat", "chars", "codePointAt", "lastIndexOf",
    // ArrayList / array
    "size", "add", "get", "set", "remove", "clear",
    "addAll", "removeAll", "iterator", "toArray", "sort",
    "subList", "indexOf",
    // HashMap
    "put", "getOrDefault", "containsKey", "containsValue",
    "keySet", "values", "entrySet",
    // StringBuilder
    "append", "insert", "reverse", "deleteCharAt",
    // Object
    "getClass", "notify", "wait", "getMessage",
    // Map.Entry
    "getKey", "getValue",
    // Stream
    "stream", "map", "filter", "forEach", "collect", "reduce",
    "sorted", "count", "toList", "distinct", "limit", "skip",
    "findFirst", "anyMatch", "allMatch", "noneMatch",
    // Iterable / Iterator
    "of", "iterator", "hasNext", "next",
];

pub struct RirInterpreter {
    pub(super) module:        RirModule,
    pub(super) heap:          RefCell<HashMap<ObjId, JavaObject>>,
    pub(super) next_id:       RefCell<ObjId>,
    pub(super) static_fields: RefCell<HashMap<String, RVal>>,
}

impl RirInterpreter {
    pub fn new(module: RirModule) -> Self {
        Self {
            module,
            heap:          RefCell::new(HashMap::new()),
            next_id:       RefCell::new(1),
            static_fields: RefCell::new(HashMap::new()),
        }
    }
}
