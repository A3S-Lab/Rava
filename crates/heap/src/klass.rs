//! KlassDescriptor trait — the unified type descriptor for AOT and MicroRT objects (§27.1).

/// Unified type descriptor implemented by both AOT and MicroRT class representations.
///
/// When AOT code calls a virtual method on any object, it reads the klass pointer
/// from the object header and calls `vtable()[slot]`. The same dispatch path works
/// for both AOT objects (direct function pointers) and MicroRT objects (interpreter
/// stub functions) — the caller never needs to know which kind it is.
pub trait KlassDescriptor: Send + Sync {
    /// The fully-qualified class name (e.g. `"com/example/User"`).
    fn name(&self) -> &str;

    /// Virtual method table.
    /// - AOT objects:    array of direct function pointers to AOT-compiled methods
    /// - MicroRT objects: array of interpreter stub functions that enter the bytecode engine
    fn vtable(&self) -> &[fn()];

    /// Size of an instance of this class in bytes (excluding the object header).
    fn instance_size(&self) -> usize;

    /// Byte offsets within the object body that contain heap references.
    /// Used by the GC to find all references during the mark phase.
    fn ref_offsets(&self) -> &[usize];
}
