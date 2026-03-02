//! Reflection engine — runtime metadata queries via AOT metadata table.
//!
//! For AOT-known classes, queries the compile-time `MetadataTable` embedded
//! in the binary (fast path). The class registry for dynamically loaded classes
//! is a Phase 5 concern.

use rava_common::error::Result;
use rava_rir::{ClassMetadata, MetadataTable};

/// Queries class/method/field metadata at runtime.
///
/// For AOT-known classes, this queries the compile-time metadata table embedded
/// in the binary (fast path). For dynamically loaded classes, it queries the
/// MicroRT class registry (slow path, cached after first lookup).
pub struct ReflectionEngine {
    table: MetadataTable,
}

impl ReflectionEngine {
    pub fn new() -> Self {
        Self {
            table: MetadataTable::new(),
        }
    }

    /// Create a ReflectionEngine backed by an AOT-generated metadata table.
    pub fn with_table(table: MetadataTable) -> Self {
        Self { table }
    }

    /// Look up a class by its binary name (e.g. `"com/example/Foo"` or `"Foo"`).
    ///
    /// Returns `Some(&ClassMetadata)` for AOT-known classes, `None` otherwise.
    pub fn find_class(&self, binary_name: &str) -> Result<Option<&ClassMetadata>> {
        // Normalise: class files use '/' as separator, metadata uses '.'
        let normalized = binary_name.replace('/', ".");
        Ok(self.table.get_class(&normalized))
    }

    /// Return the names of all fields declared on `class_name`.
    pub fn field_names(&self, class_name: &str) -> Vec<String> {
        match self.table.get_class(class_name) {
            Some(meta) => meta.fields.iter().map(|f| f.name.clone()).collect(),
            None => Vec::new(),
        }
    }

    /// Return the names of all methods declared on `class_name`.
    pub fn method_names(&self, class_name: &str) -> Vec<String> {
        match self.table.get_class(class_name) {
            Some(meta) => meta.methods.iter().map(|m| m.name.clone()).collect(),
            None => Vec::new(),
        }
    }

    /// Return true if `class_name` is registered in the AOT metadata table.
    pub fn has_class(&self, class_name: &str) -> bool {
        self.table.get_class(class_name).is_some()
    }
}

impl Default for ReflectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rava_rir::{ClassMetadata, FieldMetadata, MethodMetadata};

    fn make_table() -> MetadataTable {
        let mut t = MetadataTable::new();
        t.add_class(
            "com.example.User".into(),
            ClassMetadata {
                name: "com.example.User".into(),
                superclass: Some("java.lang.Object".into()),
                interfaces: vec![],
                fields: vec![
                    FieldMetadata {
                        name: "id".into(),
                        type_descriptor: "I".into(),
                        offset: None,
                        getter_ptr: None,
                        setter_ptr: None,
                        modifiers: vec!["private".into()],
                    },
                    FieldMetadata {
                        name: "name".into(),
                        type_descriptor: "Ljava/lang/String;".into(),
                        offset: None,
                        getter_ptr: None,
                        setter_ptr: None,
                        modifiers: vec!["private".into()],
                    },
                ],
                methods: vec![MethodMetadata {
                    name: "getName".into(),
                    signature: "()Ljava/lang/String;".into(),
                    function_ptr: None,
                    modifiers: vec!["public".into()],
                }],
                constructors: vec![],
                modifiers: vec!["public".into()],
            },
        );
        t
    }

    #[test]
    fn find_class_by_dot_name() {
        let engine = ReflectionEngine::with_table(make_table());
        let meta = engine.find_class("com.example.User").unwrap();
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().name, "com.example.User");
    }

    #[test]
    fn find_class_by_slash_name() {
        let engine = ReflectionEngine::with_table(make_table());
        let meta = engine.find_class("com/example/User").unwrap();
        assert!(meta.is_some());
    }

    #[test]
    fn find_class_unknown_returns_none() {
        let engine = ReflectionEngine::with_table(make_table());
        let meta = engine.find_class("com.example.Ghost").unwrap();
        assert!(meta.is_none());
    }

    #[test]
    fn field_names_returns_declared_fields() {
        let engine = ReflectionEngine::with_table(make_table());
        let fields = engine.field_names("com.example.User");
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"name".to_string()));
    }

    #[test]
    fn method_names_returns_declared_methods() {
        let engine = ReflectionEngine::with_table(make_table());
        let methods = engine.method_names("com.example.User");
        assert!(methods.contains(&"getName".to_string()));
    }

    #[test]
    fn has_class() {
        let engine = ReflectionEngine::with_table(make_table());
        assert!(engine.has_class("com.example.User"));
        assert!(!engine.has_class("com.example.Ghost"));
    }
}
