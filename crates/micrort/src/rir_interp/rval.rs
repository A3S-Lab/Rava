//! Runtime value types: ObjId, JavaObject, RVal.

use std::collections::HashMap;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type ObjId = u64;

/// A heap-allocated Java object.
#[derive(Debug, Clone)]
pub struct JavaObject {
    pub class_name: String,
    pub fields:     HashMap<String, RVal>,
}

/// A runtime value.
#[derive(Debug, Clone)]
pub enum RVal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Object(ObjId),
    Array(Rc<RefCell<Vec<RVal>>>),
    /// Array iterator: (backing array, current index)
    ArrayIter(Rc<RefCell<Vec<RVal>>>, Rc<Cell<usize>>),
    Null,
    Void,
}

impl RVal {
    pub(crate) fn as_int(&self) -> i64 {
        match self {
            RVal::Int(n)   => *n,
            RVal::Float(f) => *f as i64,
            RVal::Bool(b)  => if *b { 1 } else { 0 },
            RVal::Str(s)   => s.parse::<i64>().unwrap_or(0),
            _              => 0,
        }
    }

    pub(crate) fn as_float(&self) -> f64 {
        match self {
            RVal::Float(f) => *f,
            RVal::Int(n)   => *n as f64,
            RVal::Bool(b)  => if *b { 1.0 } else { 0.0 },
            RVal::Str(s)   => s.parse::<f64>().unwrap_or(0.0),
            _              => 0.0,
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            RVal::Bool(b)       => *b,
            RVal::Int(n)        => *n != 0,
            RVal::Float(f)      => *f != 0.0,
            RVal::Null          => false,
            RVal::Void          => false,
            RVal::Str(s)        => !s.is_empty(),
            RVal::Object(_)     => true,
            RVal::Array(_)      => true,
            RVal::ArrayIter(..) => true,
        }
    }

    pub(crate) fn is_float(&self) -> bool {
        matches!(self, RVal::Float(_))
    }

    pub fn to_display(&self) -> String {
        match self {
            RVal::Int(n)   => n.to_string(),
            RVal::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{:.1}", f) }
                else { f.to_string() }
            }
            RVal::Str(s)        => s.clone(),
            RVal::Bool(b)       => b.to_string(),
            RVal::Null          => "null".into(),
            RVal::Void          => "".into(),
            RVal::Object(id)    => format!("Object@{id}"),
            RVal::Array(arr)    => {
                let v = arr.borrow();
                let items: Vec<_> = v.iter().map(|x| x.to_display()).collect();
                format!("[{}]", items.join(", "))
            }
            RVal::ArrayIter(..) => "ArrayIterator".into(),
        }
    }
}
