//! GC strategy extension point — §27.2.

use rava_common::error::Result;

/// A pluggable garbage collection algorithm.
///
/// Phase 2: `NoopGc` (arena-only, no collection — sufficient for short-lived `rava run`)
/// Phase 3: `StopTheWorldGc` (tri-color mark + sweep, card table)
/// Phase 5: `ConcurrentGc` (reduce stop-the-world pauses)
pub trait GcStrategy: Send + Sync {
    /// Run a collection cycle. Returns the number of bytes freed.
    fn collect(&mut self) -> Result<usize>;
    fn used_bytes(&self) -> usize;
    fn capacity_bytes(&self) -> usize;
}

/// No-op GC — never collects. Used in Phase 1/2 where programs are short-lived.
/// Allocation simply fails when the heap is full.
pub struct NoopGc;

impl GcStrategy for NoopGc {
    fn collect(&mut self) -> Result<usize> {
        Ok(0) // nothing freed
    }
    fn used_bytes(&self) -> usize {
        0
    }
    fn capacity_bytes(&self) -> usize {
        0
    }
}
