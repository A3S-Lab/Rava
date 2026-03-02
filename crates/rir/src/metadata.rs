//! Reflection metadata structures for Phase 2.
//!
//! These structures represent the compile-time metadata table that enables
//! fast-path reflection without requiring a full bytecode interpreter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete metadata table for all classes in the program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTable {
    /// Maps fully qualified class name → class metadata
    pub classes: HashMap<String, ClassMetadata>,
}

impl MetadataTable {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    pub fn add_class(&mut self, name: String, metadata: ClassMetadata) {
        self.classes.insert(name, metadata);
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassMetadata> {
        self.classes.get(name)
    }
}

impl Default for MetadataTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata for a single class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMetadata {
    /// Fully qualified class name (e.g., "com.example.User")
    pub name: String,
    /// Superclass name (None for java.lang.Object)
    pub superclass: Option<String>,
    /// Implemented interfaces
    pub interfaces: Vec<String>,
    /// Field metadata
    pub fields: Vec<FieldMetadata>,
    /// Method metadata
    pub methods: Vec<MethodMetadata>,
    /// Constructor metadata
    pub constructors: Vec<ConstructorMetadata>,
    /// Class modifiers (public, abstract, final, etc.)
    pub modifiers: Vec<String>,
}

/// Metadata for a field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
    /// Field name
    pub name: String,
    /// Field type (Java type descriptor, e.g., "J" for long, "Ljava/lang/String;" for String)
    pub type_descriptor: String,
    /// Field offset in object layout (for instance fields)
    pub offset: Option<usize>,
    /// Function pointer for getter (if AOT-compiled)
    pub getter_ptr: Option<u64>,
    /// Function pointer for setter (if AOT-compiled)
    pub setter_ptr: Option<u64>,
    /// Field modifiers (public, static, final, etc.)
    pub modifiers: Vec<String>,
}

/// Metadata for a method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodMetadata {
    /// Method name
    pub name: String,
    /// Method signature (e.g., "(Ljava/lang/String;)V")
    pub signature: String,
    /// Function pointer (if AOT-compiled)
    pub function_ptr: Option<u64>,
    /// Method modifiers (public, static, abstract, etc.)
    pub modifiers: Vec<String>,
}

/// Metadata for a constructor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorMetadata {
    /// Constructor signature (e.g., "(Ljava/lang/String;I)V")
    pub signature: String,
    /// Function pointer (if AOT-compiled)
    pub function_ptr: Option<u64>,
    /// Constructor modifiers (public, private, etc.)
    pub modifiers: Vec<String>,
}
