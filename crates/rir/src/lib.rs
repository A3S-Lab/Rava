//! Rava Intermediate Representation (RIR).
//!
//! RIR is the stable contract between the frontend and all backends.
//! It is SSA-form and sufficiently lowered — Java's high-level abstractions
//! are gone, but the semantics are preserved.
//!
//! Structure:
//!   [`RirModule`] → [`RirFunction`]* → [`BasicBlock`]* → [`RirInstr`]*

pub mod instr;
pub mod metadata;
pub mod module;
pub mod types;

pub use instr::{BinOp, RirInstr, StackValue, UnaryOp};
pub use metadata::{
    ClassMetadata, ConstructorMetadata, FieldMetadata, MetadataTable, MethodMetadata,
};
pub use module::{BasicBlock, FuncFlags, RirFunction, RirModule};
pub use types::RirType;

// Stable ID types — newtype wrappers prevent mixing up IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FuncId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ClassId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FieldId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MethodId(pub u32);
/// An SSA value (named register).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Value(pub String);

// Keep the old `Module` name as an alias so existing code compiles.
pub use module::RirModule as Module;
