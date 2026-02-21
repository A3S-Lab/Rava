//! Three-tier class loader: Bootstrap → Platform → Application.

use rava_common::error::Result;

/// Loads Java class bytecode by fully-qualified class name.
///
/// Follows the standard Java three-tier delegation model:
/// 1. Bootstrap — loads `java.*`, `javax.*`, `sun.*` from the embedded stdlib
/// 2. Platform   — loads platform extension classes
/// 3. Application — loads user classes from the classpath
pub trait ClassLoader: Send + Sync {
    fn name(&self) -> &'static str;
    /// Load bytecode for a class by its binary name (e.g. `"com/example/Foo"`).
    fn load(&self, binary_name: &str) -> Result<Vec<u8>>;
    /// Returns true if this loader can handle the given class name.
    fn can_load(&self, binary_name: &str) -> bool;
}
