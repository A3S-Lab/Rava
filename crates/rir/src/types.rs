//! RIR type system — mirrors the Java type system at the IR level.

use serde::{Deserialize, Serialize};
use crate::ClassId;

/// A type in the RIR type system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RirType {
    // Primitive types (matching JVM primitives)
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Void,
    // Reference types
    Ref(ClassId),           // pointer to a heap object
    Array(Box<RirType>),    // Java array
    // Special types
    RawPtr,                 // for MicroRT interop layer
}

impl std::fmt::Display for RirType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RirType::I8      => write!(f, "i8"),
            RirType::I16     => write!(f, "i16"),
            RirType::I32     => write!(f, "i32"),
            RirType::I64     => write!(f, "i64"),
            RirType::F32     => write!(f, "f32"),
            RirType::F64     => write!(f, "f64"),
            RirType::Bool    => write!(f, "bool"),
            RirType::Void    => write!(f, "void"),
            RirType::Ref(id) => write!(f, "ref({})", id.0),
            RirType::Array(t) => write!(f, "{t}[]"),
            RirType::RawPtr  => write!(f, "rawptr"),
        }
    }
}
