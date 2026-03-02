//! Object header with precise mark word bit layout (§27.1).

/// Mark word bit positions within the 8-byte mark word.
pub mod mark_bits {
    /// Bits [63:62] — lock state.
    pub const LOCK_MASK: u64 = 0b11 << 62;
    pub const LOCK_UNLOCKED: u64 = 0b00 << 62;
    pub const LOCK_BIASED: u64 = 0b01 << 62;
    pub const LOCK_LIGHTWEIGHT: u64 = 0b10 << 62;
    pub const LOCK_HEAVYWEIGHT: u64 = 0b11 << 62;

    /// Bit [61] — GC tri-color mark bit.
    pub const GC_MARK: u64 = 1 << 61;

    /// Bit [60] — forwarding pointer flag (set when GC moves the object).
    pub const FORWARDING: u64 = 1 << 60;

    /// Bit [59] — origin tag: 0 = AOT object, 1 = MicroRT object.
    pub const MICRORT_ORIGIN: u64 = 1 << 59;

    /// Bits [58:32] — identity hashcode (27 bits).
    pub const HASHCODE_SHIFT: u64 = 32;
    pub const HASHCODE_MASK: u64 = 0x07FF_FFFF << 32;
}

/// The 8-byte mark word embedded in every object header.
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct MarkWord(pub u64);

impl MarkWord {
    /// Returns true if this object was allocated by MicroRT (not AOT code).
    #[inline]
    pub fn is_micrort(&self) -> bool {
        self.0 & mark_bits::MICRORT_ORIGIN != 0
    }

    /// Returns true if the GC has marked this object (tri-color: grey or black).
    #[inline]
    pub fn is_marked(&self) -> bool {
        self.0 & mark_bits::GC_MARK != 0
    }

    /// Returns true if this object has been moved by the GC (forwarding pointer set).
    #[inline]
    pub fn is_forwarded(&self) -> bool {
        self.0 & mark_bits::FORWARDING != 0
    }

    pub fn set_marked(&mut self) {
        self.0 |= mark_bits::GC_MARK;
    }

    pub fn clear_marked(&mut self) {
        self.0 &= !mark_bits::GC_MARK;
    }

    pub fn set_micrort_origin(&mut self) {
        self.0 |= mark_bits::MICRORT_ORIGIN;
    }
}

/// Every heap object begins with this 16-byte header.
#[repr(C)]
pub struct ObjectHeader {
    /// Mark word — GC state, lock state, origin tag, hashcode.
    pub mark: MarkWord,
    /// Pointer to the KlassDescriptor for this object's type.
    /// AOT objects:    points to an `AotKlass`
    /// MicroRT objects: points to a `MicroRtKlass`
    pub klass_ptr: *const (),
}

// SAFETY: ObjectHeader is only accessed under heap lock or during GC stop-the-world.
unsafe impl Send for ObjectHeader {}
unsafe impl Sync for ObjectHeader {}

/// A reference to a heap-allocated object.
///
/// This is the runtime representation of a Java reference (`jobject`).
/// It is an index/offset into the unified heap — not a raw pointer —
/// so the GC can move objects without invalidating references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapRef(pub usize);

impl HeapRef {
    pub const NULL: HeapRef = HeapRef(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_word_origin_tag() {
        let mut mw = MarkWord::default();
        assert!(!mw.is_micrort());
        mw.set_micrort_origin();
        assert!(mw.is_micrort());
    }

    #[test]
    fn mark_word_gc_mark() {
        let mut mw = MarkWord::default();
        assert!(!mw.is_marked());
        mw.set_marked();
        assert!(mw.is_marked());
        mw.clear_marked();
        assert!(!mw.is_marked());
    }
}
