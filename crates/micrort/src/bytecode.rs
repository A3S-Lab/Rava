//! Lower a parsed `.class` method's stack-based JVM bytecode to RIR, so the existing
//! RIR interpreter can execute **compiled** Java (instead of a separate bytecode VM).
//!
//! Covers the integer subset: constants, locals (`iload`/`istore`/`iinc`), arithmetic,
//! `ineg`, conditional/unconditional branches, and returns — enough for control flow and
//! loops. Unsupported opcodes (calls, objects, fields, longs/floats, …) make the method
//! "not runnable yet" (an error) rather than mis-executed.
//!
//! Locals are modelled as mutable, named env slots (`l0`, `l1`, …) rather than SSA values —
//! the RIR interpreter is a mutable-environment tree-walker, so loops work via block
//! re-execution without needing phi / block parameters.

use crate::classfile::{ClassFile, Method};
use rava_common::error::{RavaError, Result};
use rava_rir::module::{BasicBlock, FuncFlags, RirFunction};
use rava_rir::{BinOp, BlockId, ClassId, FuncId, RirInstr, RirType, UnaryOp, Value};
use std::collections::BTreeSet;

/// Lower one method to an RIR function named `Class.method`.
pub fn lower_method(class: &ClassFile, method: &Method, func_id: u32) -> Result<RirFunction> {
    let code = method
        .code
        .as_ref()
        .ok_or_else(|| RavaError::Other(format!("method {} has no Code attribute", method.name)))?;

    let mut params = Vec::new();
    let mut local = 0u16;
    if !method.is_static {
        params.push((Value(format!("l{local}")), RirType::Ref(ClassId(0))));
        local += 1;
    }
    for ty in parse_arg_types(&method.descriptor)? {
        let s = slots(&ty);
        params.push((Value(format!("l{local}")), ty));
        local += s;
    }

    Ok(RirFunction {
        id: FuncId(func_id),
        name: format!("{}.{}", class.name, method.name),
        params,
        return_type: parse_return_type(&method.descriptor)?,
        basic_blocks: lower_blocks(code)?,
        flags: FuncFlags::default(),
    })
}

// ── Control-flow graph construction ───────────────────────────────────────────

fn lower_blocks(code: &[u8]) -> Result<Vec<BasicBlock>> {
    let leaders = find_leaders(code)?;
    let starts: Vec<usize> = leaders.iter().copied().filter(|&l| l < code.len()).collect();

    let mut tmp = 0u32;
    let mut blocks = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(code.len());
        blocks.push(translate_block(code, start, end, &mut tmp)?);
    }
    Ok(blocks)
}

/// Block leaders: offset 0, every branch target, and the instruction after every
/// branch / goto / return.
fn find_leaders(code: &[u8]) -> Result<BTreeSet<usize>> {
    let mut leaders = BTreeSet::new();
    leaders.insert(0);
    let mut pc = 0;
    while pc < code.len() {
        let op = code[pc];
        let len = instr_len(op)
            .ok_or_else(|| unsupported(op))?;
        if is_branch(op) {
            let target = (pc as i64 + read_i16(code, pc + 1)? as i64) as usize;
            leaders.insert(target);
            leaders.insert(pc + len);
        } else if is_return(op) {
            leaders.insert(pc + len);
        }
        pc += len;
    }
    Ok(leaders)
}

fn translate_block(code: &[u8], start: usize, end: usize, tmp: &mut u32) -> Result<BasicBlock> {
    let mut instrs = Vec::new();
    let mut stack: Vec<Value> = Vec::new();
    let mut fresh = || {
        let v = Value(format!("t{tmp}"));
        *tmp += 1;
        v
    };
    let mut terminated = false;

    let mut pc = start;
    while pc < end {
        let op = code[pc];
        let len = instr_len(op).ok_or_else(|| unsupported(op))?;
        match op {
            0x1a..=0x1d => stack.push(Value(format!("l{}", op - 0x1a))), // iload_0..3
            0x15 => stack.push(Value(format!("l{}", code[pc + 1]))),     // iload <idx>
            0x02..=0x08 => {
                // iconst_m1..iconst_5
                let r = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: r.clone(),
                    value: op as i64 - 0x03,
                });
                stack.push(r);
            }
            0x10 => {
                let r = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: r.clone(),
                    value: code[pc + 1] as i8 as i64,
                });
                stack.push(r);
            }
            0x11 => {
                let r = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: r.clone(),
                    value: read_i16(code, pc + 1)? as i64,
                });
                stack.push(r);
            }
            0x3b..=0x3e => store_local(&mut instrs, &mut stack, (op - 0x3b) as u16)?, // istore_0..3
            0x36 => store_local(&mut instrs, &mut stack, code[pc + 1] as u16)?,       // istore <idx>
            0x84 => {
                // iinc <idx> <const>: local += const
                let local = Value(format!("l{}", code[pc + 1]));
                let by = code[pc + 2] as i8 as i64;
                let c = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: c.clone(),
                    value: by,
                });
                let t = fresh();
                instrs.push(RirInstr::BinOp {
                    op: BinOp::Add,
                    lhs: local.clone(),
                    rhs: c,
                    ret: t.clone(),
                });
                instrs.push(copy(t, local));
            }
            0x60 | 0x64 | 0x68 | 0x6c | 0x70 => {
                let rhs = pop(&mut stack)?;
                let lhs = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::BinOp {
                    op: arith_op(op),
                    lhs,
                    rhs,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0x74 => {
                // ineg
                let v = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: v,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0x99..=0x9e => {
                // if<cond> — compare top-of-stack against 0
                let val = pop(&mut stack)?;
                let zero = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: zero.clone(),
                    value: 0,
                });
                let cond = fresh();
                instrs.push(RirInstr::BinOp {
                    op: cmp_zero_op(op),
                    lhs: val,
                    rhs: zero,
                    ret: cond.clone(),
                });
                push_branch(&mut instrs, cond, code, pc, len)?;
                terminated = true;
            }
            0x9f..=0xa4 => {
                // if_icmp<cond> — compare top two
                let b = pop(&mut stack)?;
                let a = pop(&mut stack)?;
                let cond = fresh();
                instrs.push(RirInstr::BinOp {
                    op: cmp_op(op),
                    lhs: a,
                    rhs: b,
                    ret: cond.clone(),
                });
                push_branch(&mut instrs, cond, code, pc, len)?;
                terminated = true;
            }
            0xa7 => {
                let target = (pc as i64 + read_i16(code, pc + 1)? as i64) as usize;
                instrs.push(RirInstr::Jump(BlockId(target as u32)));
                terminated = true;
            }
            0xac => {
                let v = pop(&mut stack)?;
                instrs.push(RirInstr::Return(Some(v)));
                terminated = true;
            }
            0xb1 => {
                instrs.push(RirInstr::Return(None));
                terminated = true;
            }
            0x00 => {} // nop
            other => return Err(unsupported(other)),
        }
        pc += len;
    }

    if !terminated {
        instrs.push(RirInstr::Jump(BlockId(end as u32)));
    }
    Ok(BasicBlock {
        id: BlockId(start as u32),
        params: Vec::new(),
        instrs,
    })
}

// ── Instruction helpers ───────────────────────────────────────────────────────

/// Identity int copy `dst = src` (the RIR interpreter treats I32→I32 Convert as a move).
fn copy(src: Value, dst: Value) -> RirInstr {
    RirInstr::Convert {
        val: src,
        from: RirType::I32,
        to: RirType::I32,
        ret: dst,
    }
}

fn store_local(instrs: &mut Vec<RirInstr>, stack: &mut Vec<Value>, index: u16) -> Result<()> {
    let v = pop(stack)?;
    instrs.push(copy(v, Value(format!("l{index}"))));
    Ok(())
}

fn push_branch(
    instrs: &mut Vec<RirInstr>,
    cond: Value,
    code: &[u8],
    pc: usize,
    len: usize,
) -> Result<()> {
    let target = (pc as i64 + read_i16(code, pc + 1)? as i64) as usize;
    instrs.push(RirInstr::Branch {
        cond,
        then_bb: BlockId(target as u32),
        else_bb: BlockId((pc + len) as u32),
    });
    Ok(())
}

fn pop(stack: &mut Vec<Value>) -> Result<Value> {
    stack
        .pop()
        .ok_or_else(|| RavaError::Other("bytecode operand stack underflow".into()))
}

fn arith_op(op: u8) -> BinOp {
    match op {
        0x60 => BinOp::Add,
        0x64 => BinOp::Sub,
        0x68 => BinOp::Mul,
        0x6c => BinOp::Div,
        _ => BinOp::Rem, // 0x70
    }
}

/// `if<cond>` (vs 0): RIR cond is true when the *branch is taken*.
fn cmp_zero_op(op: u8) -> BinOp {
    match op {
        0x99 => BinOp::Eq, // ifeq
        0x9a => BinOp::Ne, // ifne
        0x9b => BinOp::Lt, // iflt
        0x9c => BinOp::Ge, // ifge
        0x9d => BinOp::Gt, // ifgt
        _ => BinOp::Le,    // 0x9e ifle
    }
}

fn cmp_op(op: u8) -> BinOp {
    match op {
        0x9f => BinOp::Eq, // if_icmpeq
        0xa0 => BinOp::Ne, // if_icmpne
        0xa1 => BinOp::Lt, // if_icmplt
        0xa2 => BinOp::Ge, // if_icmpge
        0xa3 => BinOp::Gt, // if_icmpgt
        _ => BinOp::Le,    // 0xa4 if_icmple
    }
}

fn is_branch(op: u8) -> bool {
    (0x99..=0xa4).contains(&op) || op == 0xa7
}

fn is_return(op: u8) -> bool {
    op == 0xac || op == 0xb1
}

/// Total byte length of a supported instruction (incl. opcode); `None` if unsupported.
fn instr_len(op: u8) -> Option<usize> {
    Some(match op {
        0x10 | 0x15 | 0x36 => 2,                      // bipush, iload, istore (1 operand)
        0x11 | 0x84 | 0xa7 => 3,                      // sipush, iinc, goto
        0x99..=0xa4 => 3,                             // if<cond> / if_icmp<cond>
        0x00 => 1,                                    // nop
        0x02..=0x08 => 1,                             // iconst_m1..5
        0x1a..=0x1d => 1,                             // iload_0..3
        0x3b..=0x3e => 1,                             // istore_0..3
        0x60 | 0x64 | 0x68 | 0x6c | 0x70 => 1,        // iadd isub imul idiv irem
        0x74 => 1,                                    // ineg
        0xac | 0xb1 => 1,                             // ireturn, return
        _ => return None,
    })
}

fn unsupported(op: u8) -> RavaError {
    RavaError::Other(format!(
        "unsupported JVM bytecode opcode 0x{op:02x} (bytecode→RIR covers the integer subset \
         with control flow so far)"
    ))
}

fn read_i16(code: &[u8], at: usize) -> Result<i16> {
    let hi = *code
        .get(at)
        .ok_or_else(|| RavaError::Other("truncated bytecode operand".into()))? as i16;
    let lo = *code
        .get(at + 1)
        .ok_or_else(|| RavaError::Other("truncated bytecode operand".into()))? as i16;
    Ok((hi << 8) | (lo & 0xff))
}

// ── Method descriptor parsing ─────────────────────────────────────────────────

fn slots(ty: &RirType) -> u16 {
    matches!(ty, RirType::I64 | RirType::F64) as u16 + 1
}

fn parse_arg_types(descriptor: &str) -> Result<Vec<RirType>> {
    let args = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split(')').next())
        .ok_or_else(|| RavaError::Other(format!("bad method descriptor {descriptor:?}")))?;
    let mut chars = args.chars().peekable();
    let mut out = Vec::new();
    while chars.peek().is_some() {
        out.push(parse_field_type(&mut chars)?);
    }
    Ok(out)
}

fn parse_return_type(descriptor: &str) -> Result<RirType> {
    let ret = descriptor
        .split(')')
        .nth(1)
        .ok_or_else(|| RavaError::Other(format!("bad method descriptor {descriptor:?}")))?;
    if ret == "V" {
        return Ok(RirType::Void);
    }
    parse_field_type(&mut ret.chars().peekable())
}

fn parse_field_type(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<RirType> {
    let c = chars
        .next()
        .ok_or_else(|| RavaError::Other("truncated type descriptor".into()))?;
    Ok(match c {
        'B' => RirType::I8,
        'C' | 'S' => RirType::I16,
        'I' => RirType::I32,
        'J' => RirType::I64,
        'F' => RirType::F32,
        'D' => RirType::F64,
        'Z' => RirType::Bool,
        '[' => RirType::Array(Box::new(parse_field_type(chars)?)),
        'L' => {
            for ch in chars.by_ref() {
                if ch == ';' {
                    break;
                }
            }
            RirType::Ref(ClassId(0))
        }
        other => return Err(RavaError::Other(format!("unknown type descriptor '{other}'"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classfile;
    use crate::rir_interp::{RVal, RirInterpreter};
    use rava_rir::module::RirModule;

    fn run(fixture: &[u8], method: &str, args: Vec<RVal>) -> i64 {
        let cf = classfile::parse(fixture).unwrap();
        let m = cf.methods.iter().find(|m| m.name == method).unwrap();
        let func = lower_method(&cf, m, 0).unwrap();
        let name = func.name.clone();
        let mut module = RirModule::new("M");
        module.functions.push(func);
        let r = RirInterpreter::new(module).call(&name, args).unwrap();
        r.to_display().parse().unwrap()
    }

    const ADD: &[u8] = include_bytes!("fixtures/Add.class");
    const CALC: &[u8] = include_bytes!("fixtures/Calc.class");

    #[test]
    fn arithmetic() {
        assert_eq!(run(ADD, "add", vec![RVal::Int(2), RVal::Int(3)]), 5);
        assert_eq!(run(ADD, "triple", vec![RVal::Null, RVal::Int(7)]), 21);
    }

    #[test]
    fn conditionals() {
        assert_eq!(run(CALC, "max", vec![RVal::Int(7), RVal::Int(3)]), 7);
        assert_eq!(run(CALC, "max", vec![RVal::Int(3), RVal::Int(7)]), 7);
        assert_eq!(run(CALC, "absv", vec![RVal::Int(-4)]), 4);
        assert_eq!(run(CALC, "absv", vec![RVal::Int(4)]), 4);
    }

    #[test]
    fn loops() {
        // sumTo(n) = 1 + 2 + ... + n  (exercises istore / iinc / if_icmp / goto back-edge)
        assert_eq!(run(CALC, "sumTo", vec![RVal::Int(5)]), 15);
        assert_eq!(run(CALC, "sumTo", vec![RVal::Int(10)]), 55);
        assert_eq!(run(CALC, "sumTo", vec![RVal::Int(0)]), 0);
    }
}
