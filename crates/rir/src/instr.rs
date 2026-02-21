//! RIR instruction set — matches the spec in docs/rava.md §24.2.

use serde::{Deserialize, Serialize};
use crate::{BlockId, ClassId, FieldId, FuncId, MethodId, RirType, Value};

/// A single RIR instruction (SSA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RirInstr {
    // ── Control flow ──────────────────────────────────────────────────────────
    Branch    { cond: Value, then_bb: BlockId, else_bb: BlockId },
    Jump      (BlockId),
    Return    (Option<Value>),
    Unreachable,

    // ── Calls ─────────────────────────────────────────────────────────────────
    /// Static / direct call.
    Call          { func: FuncId,      args: Vec<Value>, ret: Option<Value> },
    /// Virtual dispatch (class hierarchy).
    CallVirtual   { receiver: Value,   method: MethodId, args: Vec<Value>, ret: Option<Value> },
    /// Interface dispatch.
    CallInterface { receiver: Value,   method: MethodId, args: Vec<Value>, ret: Option<Value> },

    // ── Object operations ─────────────────────────────────────────────────────
    New        { class: ClassId,                           ret: Value },
    GetField   { obj: Value,   field: FieldId,             ret: Value },
    SetField   { obj: Value,   field: FieldId, val: Value             },
    GetStatic  { field: FieldId,                           ret: Value },
    SetStatic  { field: FieldId,               val: Value             },
    Instanceof { obj: Value,   class: ClassId,             ret: Value },
    /// Throws `ClassCastException` if the cast fails.
    Checkcast  { obj: Value,   class: ClassId                         },

    // ── Array operations ──────────────────────────────────────────────────────
    NewArray   { elem_type: RirType, len: Value,           ret: Value },
    ArrayLoad  { arr: Value, idx: Value,                   ret: Value },
    ArrayStore { arr: Value, idx: Value, val: Value                   },
    ArrayLen   { arr: Value,                               ret: Value },

    // ── Arithmetic / bitwise / comparison ─────────────────────────────────────
    BinOp   { op: BinOp,   lhs: Value, rhs: Value, ret: Value },
    UnaryOp { op: UnaryOp, operand: Value,          ret: Value },

    // ── Type conversion ───────────────────────────────────────────────────────
    Convert { val: Value, from: RirType, to: RirType, ret: Value },

    // ── Constants ─────────────────────────────────────────────────────────────
    ConstInt    { ret: Value, value: i64 },
    ConstFloat  { ret: Value, value: f64 },
    ConstStr    { ret: Value, value: String },
    ConstNull   { ret: Value },

    // ── Exceptions ────────────────────────────────────────────────────────────
    Throw(Value),

    // ── Synchronization ───────────────────────────────────────────────────────
    MonitorEnter(Value),
    MonitorExit(Value),

    // ── MicroRT interop (inserted by Analysis Passes) ─────────────────────────
    /// Dynamic `Class.forName(expr)` — unresolvable at compile time.
    MicroRtReflect   { class_name: Value,                       ret: Value },
    /// Dynamic `Proxy.newProxyInstance` with runtime interface list.
    MicroRtProxy     { interfaces: Vec<Value>, handler: Value,  ret: Value },
    /// Dynamic class loading via `ServiceLoader` or `ClassLoader.loadClass`.
    MicroRtClassLoad { class_name: Value,                       ret: Value },
}

/// Binary operation kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    And, Or, Xor,
    BitAnd, BitOr,
    Shl, Shr, UShr,
    Eq, Ne, Lt, Le, Gt, Ge,
}

/// Unary operation kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// A value on the MicroRT operand stack.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StackValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// Reference to a heap object (index into the heap).
    Ref(usize),
    Null,
}
