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

/// Thread-local output sink — set during tests to capture println output.
thread_local! {
    pub(crate) static OUTPUT: RefCell<Option<Vec<u8>>> = RefCell::new(None);
    /// Carries the original thrown object across function call boundaries.
    pub(crate) static THROWN_OBJ: RefCell<Option<RVal>> = RefCell::new(None);
    /// Lambda capture table: lambda_name → captured variable values.
    pub(crate) static LAMBDA_CAPTURES: RefCell<HashMap<String, HashMap<String, RVal>>> =
        RefCell::new(HashMap::new());
    /// Annotation registry: "ClassName" or "ClassName::memberName" → list of annotation names.
    pub(crate) static ANNOTATION_REGISTRY: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
}

/// Write a line to the current output sink (or real stdout if none set).
pub(crate) fn write_output(s: &str) {
    OUTPUT.with(|o| {
        if let Some(ref mut buf) = *o.borrow_mut() {
            buf.extend_from_slice(s.as_bytes());
            buf.push(b'\n');
        } else {
            println!("{}", s);
        }
    });
}

/// Write without newline.
pub(crate) fn write_output_no_nl(s: &str) {
    OUTPUT.with(|o| {
        if let Some(ref mut buf) = *o.borrow_mut() {
            buf.extend_from_slice(s.as_bytes());
        } else {
            print!("{}", s);
        }
    });
}

/// All known instance method names — used to reverse-lookup __method__<name> calls.
const KNOWN_METHODS: &[&str] = &[
    // String
    "length", "isEmpty", "toUpperCase", "toLowerCase", "trim",
    "charAt", "substring", "contains", "startsWith", "endsWith",
    "equals", "equalsIgnoreCase", "replace", "replaceAll", "replaceFirst",
    "indexOf", "split", "matches",
    "toString", "hashCode", "compareTo", "toCharArray", "valueOf",
    "format", "join", "strip", "stripLeading", "stripTrailing",
    "repeat", "chars", "codePointAt", "lastIndexOf",
    // ArrayList / array
    "size", "add", "get", "set", "remove", "clear",
    "addAll", "removeAll", "iterator", "toArray", "sort",
    "subList", "indexOf",
    // HashMap
    "put", "get", "getOrDefault", "containsKey", "containsValue",
    "keySet", "values", "entrySet", "remove", "replace", "putIfAbsent",
    "computeIfAbsent", "merge", "compute", "putAll", "forEach",
    // PriorityQueue / Queue / Deque
    "offer", "poll", "peek", "push", "pop",
    "addFirst", "addLast", "removeFirst", "removeLast",
    "getFirst", "getLast", "offerFirst", "offerLast",
    "pollFirst", "pollLast", "peekFirst", "peekLast",
    // StringBuilder
    "append", "insert", "reverse", "deleteCharAt",
    // Object
    "getClass", "notify", "wait", "getMessage",
    // Map.Entry
    "getKey", "getValue",
    // Stream
    "stream", "map", "flatMap", "filter", "forEach", "collect", "reduce",
    "sorted", "count", "toList", "distinct", "limit", "skip",
    "findFirst", "anyMatch", "allMatch", "noneMatch",
    "mapToInt", "mapToLong", "mapToDouble", "sum", "average", "min", "max",
    // Iterable / Iterator
    "of", "iterator", "hasNext", "next",
    // Optional
    "isPresent", "isEmpty", "get", "orElse", "orElseGet", "orElseThrow", "ifPresent",
    // Set / HashSet / TreeSet
    "contains", "first", "last", "headSet", "tailSet", "subSet",
    // java.time
    "getYear", "getMonthValue", "getDayOfMonth", "getHour", "getMinute", "getSecond",
    "getNano", "getDayOfWeek", "getDayOfYear", "getMonth",
    "plusDays", "plusMonths", "plusYears", "plusHours", "plusMinutes", "plusSeconds",
    "minusDays", "minusMonths", "minusYears", "minusHours", "minusMinutes", "minusSeconds",
    "isBefore", "isAfter", "isEqual", "withDayOfMonth", "withMonth", "withYear",
    "toLocalDate", "toLocalTime", "toLocalDateTime", "toEpochSecond", "toEpochMilli",
    "atStartOfDay", "atTime", "atDate",
    "toDays", "toHours", "toMinutes", "toSeconds", "toNanos", "toMillis",
    "getSeconds", "getNano",
    "matches",
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

    /// Run main(), capturing all System.out output into `buf`.
    pub fn run_main_with_output(&self, buf: &mut Vec<u8>) -> rava_common::error::Result<()> {
        OUTPUT.with(|o| { *o.borrow_mut() = Some(Vec::new()); });
        let result = self.run_main();
        OUTPUT.with(|o| {
            if let Some(captured) = o.borrow_mut().take() {
                buf.extend_from_slice(&captured);
            }
        });
        result
    }
}

