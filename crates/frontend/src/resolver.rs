//! Import resolver — resolves `import` statements to source files and compiles on-demand.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use rava_common::error::{RavaError, Result};
use rava_rir::Module;
use crate::compiler::Compiler;

/// Resolves imports and compiles referenced files on-demand.
pub struct ImportResolver {
    /// Root source directories to search (typically ["src"])
    source_roots: Vec<PathBuf>,
    /// Rava stdlib directory (stdlib/)
    stdlib_root: Option<PathBuf>,
    /// Already-compiled files (canonical path → module)
    compiled: HashMap<PathBuf, Module>,
    /// Files currently being compiled (cycle detection)
    in_progress: HashSet<PathBuf>,
}

impl ImportResolver {
    pub fn new(source_roots: Vec<PathBuf>) -> Self {
        let stdlib_root = find_stdlib();
        Self {
            source_roots,
            stdlib_root,
            compiled: HashMap::new(),
            in_progress: HashSet::new(),
        }
    }

    /// Compile a file and all its transitive imports.
    pub fn compile_file(&mut self, file: &Path, compiler: &Compiler) -> Result<()> {
        let canonical = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        if self.compiled.contains_key(&canonical) || self.in_progress.contains(&canonical) {
            return Ok(()); // already done or cycle
        }

        self.in_progress.insert(canonical.clone());

        // Lex + parse to get imports
        let source = std::fs::read_to_string(file).map_err(|e| {
            RavaError::Other(format!("cannot read {}: {e}", file.display()))
        })?;
        let tokens = crate::lexer::Lexer::new(&source).tokenize()?;
        let ast = crate::parser::Parser::new(tokens).parse_file()?;

        // Recursively compile imports first
        for import in &ast.imports {
            let import_files = self.resolve_import(import);
            for f in import_files {
                self.compile_file(&f, compiler)?;
            }
        }

        // Compile this file
        let module_name = file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        let module = crate::lowerer::Lowerer::new(module_name).lower_file(&ast)?;

        self.in_progress.remove(&canonical);
        self.compiled.insert(canonical, module);
        Ok(())
    }

    /// Resolve an import string to file paths.
    ///
    /// Search order:
    /// 1. User source roots (src/)
    /// 2. Rava stdlib (stdlib/) — for java.*, javax.*, and any stdlib class
    /// 3. Flat fallback (just the class name)
    /// 4. Not found → empty (might be handled by interpreter builtins)
    fn resolve_import(&self, import: &str) -> Vec<PathBuf> {
        if import.ends_with(".*") {
            return self.resolve_wildcard(import);
        }

        // Convert dots to path: java.util.Arrays → java/util/Arrays.java
        let rel_path = import.replace('.', "/") + ".java";

        // 1. Search user source roots
        for root in &self.source_roots {
            let full = root.join(&rel_path);
            if full.exists() {
                return vec![full];
            }
        }

        // 2. Search stdlib
        if let Some(ref stdlib) = self.stdlib_root {
            let full = stdlib.join(&rel_path);
            if full.exists() {
                return vec![full];
            }
        }

        // 3. Flat fallback: just the class name
        let class_name = import.rsplit('.').next().unwrap_or(import);
        let flat_name = format!("{class_name}.java");
        for root in &self.source_roots {
            let flat = root.join(&flat_name);
            if flat.exists() {
                return vec![flat];
            }
        }

        Vec::new() // not found — might be a builtin
    }

    /// Resolve wildcard import: `com.example.*` → all .java files in that directory.
    fn resolve_wildcard(&self, import: &str) -> Vec<PathBuf> {
        let package = import.trim_end_matches(".*");
        let rel_dir = package.replace('.', "/");

        let mut files = Vec::new();

        // Search user source roots
        for root in &self.source_roots {
            let dir = root.join(&rel_dir);
            Self::collect_java_in_dir(&dir, &mut files);
        }

        // Search stdlib
        if let Some(ref stdlib) = self.stdlib_root {
            let dir = stdlib.join(&rel_dir);
            Self::collect_java_in_dir(&dir, &mut files);
        }

        files
    }

    fn collect_java_in_dir(dir: &Path, files: &mut Vec<PathBuf>) {
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "java").unwrap_or(false) {
                        files.push(path);
                    }
                }
            }
        }
    }

    /// Consume the resolver and return the merged module.
    pub fn into_module(self) -> Module {
        let mut merged = Module::new("project");
        for (_, module) in self.compiled {
            merged.merge(module);
        }
        merged
    }
}

/// Find the Rava stdlib directory.
fn find_stdlib() -> Option<PathBuf> {
    let exe = std::env::current_exe().unwrap_or_default();
    let candidates = [
        // Development: relative to cwd
        PathBuf::from("stdlib"),
        // Relative to binary
        exe.parent().unwrap_or(Path::new(".")).join("../../stdlib"),
        exe.parent().unwrap_or(Path::new(".")).join("../share/rava/stdlib"),
    ];
    for c in &candidates {
        if c.is_dir() {
            return Some(c.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jdk_imports_resolve_to_stdlib() {
        // If stdlib exists, java.util.Arrays should resolve
        let resolver = ImportResolver::new(vec![PathBuf::from("src")]);
        if resolver.stdlib_root.is_some() {
            let files = resolver.resolve_import("java.util.Arrays");
            assert!(!files.is_empty(), "java.util.Arrays should resolve to stdlib");
        }
    }

    #[test]
    fn unknown_jdk_import_returns_empty() {
        let resolver = ImportResolver::new(vec![PathBuf::from("src")]);
        let files = resolver.resolve_import("java.util.NonExistent");
        assert!(files.is_empty());
    }
}
