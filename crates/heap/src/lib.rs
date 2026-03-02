//! Unified heap — §27 of the architecture spec.
//!
//! AOT-compiled objects and MicroRT-interpreted objects share this heap
//! and are managed by the same GC. This is the foundation for transparent
//! AOT ↔ MicroRT interoperability.
//!
//! # Object header layout (16 bytes, §27.1)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │             Unified Object Header (16 bytes)          │
//! │  ┌────────────────────────────────────────────────┐  │
//! │  │              Mark Word (8 bytes)               │  │
//! │  │  [63:62] lock state                            │  │
//! │  │  [61]    GC mark bit (tri-color)               │  │
//! │  │  [60]    forwarding pointer flag               │  │
//! │  │  [59]    origin tag: 0=AOT  1=MicroRT          │  │
//! │  │  [58:32] identity hashcode (27 bits)           │  │
//! │  │  [31:0]  GC generation + flags                 │  │
//! │  └────────────────────────────────────────────────┘  │
//! │  ┌────────────────────────────────────────────────┐  │
//! │  │              Klass Pointer (8 bytes)           │  │
//! │  │  → KlassDescriptor (AOT or MicroRT)            │  │
//! │  └────────────────────────────────────────────────┘  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! Extension points:
//!   - [`GcStrategy`] — pluggable GC algorithm

pub mod gc;
pub mod heap;
pub mod klass;
pub mod object;

pub use gc::{GcStrategy, NoopGc};
pub use heap::UnifiedHeap;
pub use klass::KlassDescriptor;
pub use object::{HeapRef, MarkWord, ObjectHeader};
