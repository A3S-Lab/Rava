//! UnifiedHeap implementation — §27.2.

use crate::gc::GcStrategy;
use crate::object::HeapRef;
use rava_common::error::{RavaError, Result};

/// Card table constant: one card covers 512 bytes of heap.
const CARD_SIZE_BYTES: usize = 512;
/// Dirty card marker written by the write barrier.
const DIRTY_CARD: u8 = 1;

/// The unified heap shared by AOT-compiled code and MicroRT.
///
/// Allocation fast path: TLAB (Thread-Local Allocation Buffer) bump pointer.
/// Slow path: request a new TLAB or trigger Minor GC.
pub struct UnifiedHeap {
    /// Raw heap storage.
    storage: Vec<u8>,
    /// Current allocation cursor (bump pointer into `storage`).
    cursor: usize,
    /// Card table: 1 byte per `CARD_SIZE_BYTES` of heap.
    /// Written by the write barrier after every reference field store.
    card_table: Vec<u8>,
    /// Pluggable GC strategy.
    gc: Box<dyn GcStrategy>,
}

impl UnifiedHeap {
    pub fn new(capacity: usize, gc: Box<dyn GcStrategy>) -> Self {
        let card_count = capacity.div_ceil(CARD_SIZE_BYTES);
        Self {
            storage: vec![0u8; capacity],
            cursor: 1, // 0 is reserved for HeapRef::NULL
            card_table: vec![0u8; card_count],
            gc,
        }
    }

    /// Allocate `size` bytes on the heap. Returns a `HeapRef` (offset into storage).
    ///
    /// This is the slow path — the fast path is a TLAB bump pointer in the thread.
    /// TODO(phase-2): implement per-thread TLABs.
    pub fn alloc(&mut self, size: usize) -> Result<HeapRef> {
        // 8-byte align
        let aligned_size = (size + 7) & !7;
        if self.cursor + aligned_size > self.storage.len() {
            // Try GC before giving up
            self.gc.collect()?;
            if self.cursor + aligned_size > self.storage.len() {
                return Err(RavaError::Other(format!(
                    "heap out of memory: need {aligned_size} bytes, \
                     used {}/{} bytes",
                    self.cursor,
                    self.storage.len()
                )));
            }
        }
        let offset = self.cursor;
        self.cursor += aligned_size;
        Ok(HeapRef(offset))
    }

    /// Write barrier — must be called after every store to a reference field.
    ///
    /// Marks the card containing `obj_offset` as dirty so the GC knows to
    /// re-scan it during the next minor collection.
    #[inline]
    pub fn write_barrier(&mut self, obj_offset: usize) {
        let card_idx = obj_offset / CARD_SIZE_BYTES;
        if card_idx < self.card_table.len() {
            self.card_table[card_idx] = DIRTY_CARD;
        }
    }

    pub fn used_bytes(&self) -> usize {
        self.cursor
    }
    pub fn capacity_bytes(&self) -> usize {
        self.storage.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::NoopGc;

    #[test]
    fn alloc_advances_cursor() {
        let mut heap = UnifiedHeap::new(4096, Box::new(NoopGc));
        let r1 = heap.alloc(16).unwrap();
        let r2 = heap.alloc(16).unwrap();
        assert_ne!(r1, r2);
        assert_eq!(heap.used_bytes(), 33); // 1 (null sentinel) + 16 + 16
    }

    #[test]
    fn alloc_aligns_to_8_bytes() {
        let mut heap = UnifiedHeap::new(4096, Box::new(NoopGc));
        let _ = heap.alloc(3).unwrap();
        // cursor should be at 1 + 8 = 9 (3 rounded up to 8)
        assert_eq!(heap.used_bytes(), 9);
    }

    #[test]
    fn alloc_fails_when_full() {
        let mut heap = UnifiedHeap::new(16, Box::new(NoopGc));
        // 1 byte used for null sentinel; only 15 bytes left
        assert!(heap.alloc(16).is_err());
    }

    #[test]
    fn write_barrier_marks_card() {
        let mut heap = UnifiedHeap::new(4096, Box::new(NoopGc));
        heap.write_barrier(0);
        assert_eq!(heap.card_table[0], DIRTY_CARD);
    }
}
