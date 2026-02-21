//! Java type system primitives.

use serde::{Deserialize, Serialize};

/// A Java type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JavaType {
    // Primitives
    Void,
    Boolean,
    Byte,
    Short,
    Char,
    Int,
    Long,
    Float,
    Double,
    // Reference types
    Class(String),
    Array(Box<JavaType>),
    Null,
}

impl std::fmt::Display for JavaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaType::Void    => write!(f, "void"),
            JavaType::Boolean => write!(f, "boolean"),
            JavaType::Byte    => write!(f, "byte"),
            JavaType::Short   => write!(f, "short"),
            JavaType::Char    => write!(f, "char"),
            JavaType::Int     => write!(f, "int"),
            JavaType::Long    => write!(f, "long"),
            JavaType::Float   => write!(f, "float"),
            JavaType::Double  => write!(f, "double"),
            JavaType::Class(name) => write!(f, "{name}"),
            JavaType::Array(inner) => write!(f, "{inner}[]"),
            JavaType::Null    => write!(f, "null"),
        }
    }
}
