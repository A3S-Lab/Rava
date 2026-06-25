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

use crate::classfile::{ClassFile, Constant, ExceptionEntry, Method};
use rava_common::error::{RavaError, Result};
use rava_rir::module::{BasicBlock, FuncFlags, RirFunction, RirModule};
use rava_rir::{
    BinOp, BlockId, ClassId, FieldId, FuncId, MethodId, RirInstr, RirType, UnaryOp, Value,
};
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
        basic_blocks: lower_blocks(code, class, &method.exceptions)?,
        flags: FuncFlags::default(),
    })
}

/// Parse one `.class` file and lower its (lowerable) methods into a runnable RIR module.
pub fn load_class_module(bytes: &[u8]) -> Result<RirModule> {
    load_classes_module(std::slice::from_ref(&bytes))
}

/// Lower several `.class` files into one module so cross-class calls resolve — the core
/// of running a multi-class program or a JAR. Methods using opcodes not yet supported are
/// skipped (so one advanced method doesn't block running the rest); field names are
/// registered so `getfield`/`putfield` resolve.
pub fn load_classes_module(classes: &[&[u8]]) -> Result<RirModule> {
    let mut module = RirModule::new("app");
    let mut func_id = 0u32;
    for bytes in classes {
        let cf = crate::classfile::parse(bytes)?;
        for f in &cf.fields {
            module
                .field_names
                .insert(crate::lowerer_hash::encode_builtin(f), f.clone());
        }
        for m in &cf.methods {
            if let Ok(func) = lower_method(&cf, m, func_id) {
                module.functions.push(func);
                func_id += 1;
            }
        }
    }
    Ok(module)
}

/// Extract every `.class` entry's bytes from a JAR (a ZIP archive).
fn jar_class_entries(jar_bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(jar_bytes))
        .map_err(|e| RavaError::Other(format!("not a valid JAR/ZIP archive: {e}")))?;
    let mut classes: Vec<Vec<u8>> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| RavaError::Other(format!("reading JAR entry {i}: {e}")))?;
        if entry.name().ends_with(".class") {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| RavaError::Other(format!("reading JAR entry: {e}")))?;
            classes.push(buf);
        }
    }
    Ok(classes)
}

/// Load every `.class` from one JAR into a runnable module.
pub fn load_jar(jar_bytes: &[u8]) -> Result<RirModule> {
    load_jars(std::slice::from_ref(&jar_bytes))
}

/// Load several JARs (a main JAR plus its dependency JARs / classpath) into one module
/// so cross-JAR calls link — the entry point for running a JAR with its dependencies.
pub fn load_jars(jars: &[&[u8]]) -> Result<RirModule> {
    let mut all: Vec<Vec<u8>> = Vec::new();
    for jar in jars {
        all.extend(jar_class_entries(jar)?);
    }
    let refs: Vec<&[u8]> = all.iter().map(|v| v.as_slice()).collect();
    load_classes_module(&refs)
}

// ── Control-flow graph construction ───────────────────────────────────────────

fn lower_blocks(
    code: &[u8],
    class: &ClassFile,
    exceptions: &[ExceptionEntry],
) -> Result<Vec<BasicBlock>> {
    let mut leaders = find_leaders(code)?;
    // try-region start and handler are block leaders. (Not `end`: it can fall in the
    // middle of a straight-line sequence and splitting there would break the operand
    // stack; the handler is registered per-block from the try start, which is enough.)
    for e in exceptions {
        leaders.insert(e.start as usize);
        leaders.insert(e.handler as usize);
    }
    let starts: Vec<usize> = leaders.iter().copied().filter(|&l| l < code.len()).collect();

    let mut tmp = 0u32;
    let mut synth_id = 0u32;
    let mut blocks = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(code.len());
        blocks.extend(translate_block(
            code,
            start,
            end,
            &mut tmp,
            &mut synth_id,
            class,
            exceptions,
        )?);
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
        if op == 0xaa || op == 0xab {
            let (len, cases, default) = parse_switch(code, pc)?;
            for (_, t) in &cases {
                leaders.insert(*t);
            }
            leaders.insert(default);
            leaders.insert(pc + len);
            pc += len;
            continue;
        }
        let len = instr_len(op).ok_or_else(|| unsupported(op))?;
        if is_branch(op) {
            let target = (pc as i64 + read_i16(code, pc + 1)? as i64) as usize;
            leaders.insert(target);
            leaders.insert(pc + len);
        } else if is_return(op) || op == 0xbf {
            // return / athrow end a block; the following instruction starts a new one.
            leaders.insert(pc + len);
        }
        pc += len;
    }
    Ok(leaders)
}

fn translate_block(
    code: &[u8],
    start: usize,
    end: usize,
    tmp: &mut u32,
    synth_id: &mut u32,
    class: &ClassFile,
    exceptions: &[ExceptionEntry],
) -> Result<Vec<BasicBlock>> {
    let mut instrs = Vec::new();
    let mut stack: Vec<Value> = Vec::new();
    // Extra synthetic blocks emitted by this block (e.g. a switch's comparison chain).
    let mut extra_blocks: Vec<BasicBlock> = Vec::new();
    let mut fresh = || {
        let v = Value(format!("t{tmp}"));
        *tmp += 1;
        v
    };
    let mut terminated = false;

    // A catch/handler block receives the caught exception on the operand stack.
    if exceptions.iter().any(|e| e.handler as usize == start) {
        stack.push(Value("__exception__".into()));
    }
    // Entering a try region: register its catch handlers via the interpreter's
    // __try_catch__ marker (handler block id : caught type; empty = catch-all).
    for e in exceptions {
        if e.start as usize == start {
            let ret = fresh();
            let ty = e.catch_type.clone().unwrap_or_default();
            instrs.push(RirInstr::ConstStr {
                ret,
                value: format!("__try_catch__{}:{}", e.handler, ty),
            });
        }
    }

    let mut pc = start;
    while pc < end {
        let op = code[pc];
        if op == 0xaa || op == 0xab {
            // tableswitch / lookupswitch → a chain of `key == case` comparisons (RIR has no
            // jump table). The key is held in a named local so each comparison block reads it.
            let (len, cases, default) = parse_switch(code, pc)?;
            let key = pop(&mut stack)?;
            let key_slot = Value(format!("__sw{start}_key"));
            instrs.push(copy(key, key_slot.clone()));
            let n = cases.len();
            let base = *synth_id;
            *synth_id += n as u32;
            let default_bb = BlockId(default as u32);
            for (i, (m, target)) in cases.iter().enumerate() {
                let else_bb = if i + 1 < n {
                    BlockId(SYNTH_FLAG | (base + i as u32 + 1))
                } else {
                    default_bb
                };
                let c = fresh();
                let cond = fresh();
                let cmp = vec![
                    RirInstr::ConstInt {
                        ret: c.clone(),
                        value: *m,
                    },
                    RirInstr::BinOp {
                        op: BinOp::Eq,
                        lhs: key_slot.clone(),
                        rhs: c,
                        ret: cond.clone(),
                    },
                    RirInstr::Branch {
                        cond,
                        then_bb: BlockId(*target as u32),
                        else_bb,
                    },
                ];
                if i == 0 {
                    instrs.extend(cmp);
                } else {
                    extra_blocks.push(BasicBlock {
                        id: BlockId(SYNTH_FLAG | (base + i as u32)),
                        params: Vec::new(),
                        instrs: cmp,
                    });
                }
            }
            if n == 0 {
                instrs.push(RirInstr::Jump(default_bb));
            }
            terminated = true;
            pc += len;
            continue;
        }
        let len = instr_len(op).ok_or_else(|| unsupported(op))?;
        match op {
            0x1a..=0x1d => stack.push(Value(format!("l{}", op - 0x1a))), // iload_0..3
            0x1e..=0x29 => stack.push(Value(format!("l{}", (op - 0x1e) % 4))), // {l,f,d}load_0..3
            0x15 => stack.push(Value(format!("l{}", code[pc + 1]))),     // iload <idx>
            0x16..=0x18 => stack.push(Value(format!("l{}", code[pc + 1]))), // {l,f,d}load <idx>
            0x02..=0x08 => {
                // iconst_m1..iconst_5
                let r = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: r.clone(),
                    value: op as i64 - 0x03,
                });
                stack.push(r);
            }
            0x09 | 0x0a => {
                // lconst_0/1
                let r = fresh();
                instrs.push(RirInstr::ConstInt {
                    ret: r.clone(),
                    value: (op - 0x09) as i64,
                });
                stack.push(r);
            }
            0x0b..=0x0f => {
                // fconst_0/1/2 (0x0b–0x0d), dconst_0/1 (0x0e/0x0f)
                let value = if op <= 0x0d {
                    (op - 0x0b) as f64
                } else {
                    (op - 0x0e) as f64
                };
                let r = fresh();
                instrs.push(RirInstr::ConstFloat {
                    ret: r.clone(),
                    value,
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
            0x3f..=0x4a => store_local(&mut instrs, &mut stack, ((op - 0x3f) % 4) as u16)?, // {l,f,d}store_0..3
            0x36 => store_local(&mut instrs, &mut stack, code[pc + 1] as u16)?,       // istore <idx>
            0x37..=0x39 => store_local(&mut instrs, &mut stack, code[pc + 1] as u16)?, // {l,f,d}store <idx>
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
            0x60..=0x73 => {
                // {i,l,f,d}{add,sub,mul,div,rem} — the interpreter picks int vs float
                // arithmetic from the operand values at runtime.
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
            0x74..=0x77 => {
                // {i,l,f,d}neg
                let v = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: v,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0x78..=0x83 => {
                // {i,l}{shl,shr,ushr,and,or,xor} — shifts and bitwise ops
                let rhs = pop(&mut stack)?;
                let lhs = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::BinOp {
                    op: bitwise_op(op),
                    lhs,
                    rhs,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0x85..=0x93 => {
                // numeric conversions (i2l, i2d, l2i, f2i, d2i, i2b, …)
                let v = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::Convert {
                    val: v,
                    from: RirType::RawPtr,
                    to: convert_target(op),
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
            0x12 | 0x13 | 0x14 => {
                // ldc <u8> / ldc_w <u16> / ldc2_w <u16>  (string / int / long / float / double)
                let idx = if op == 0x12 {
                    code[pc + 1] as u16
                } else {
                    read_u16(code, pc + 1)?
                };
                let r = fresh();
                match class.constant(idx)? {
                    Constant::Str(s) => instrs.push(RirInstr::ConstStr { ret: r.clone(), value: s }),
                    Constant::Int(v) => instrs.push(RirInstr::ConstInt { ret: r.clone(), value: v }),
                    Constant::Float(v) => {
                        instrs.push(RirInstr::ConstFloat { ret: r.clone(), value: v })
                    }
                }
                stack.push(r);
            }
            0x2a..=0x2d => stack.push(Value(format!("l{}", op - 0x2a))), // aload_0..3
            0x19 => stack.push(Value(format!("l{}", code[pc + 1]))),     // aload <idx>
            0x4b..=0x4e => store_local(&mut instrs, &mut stack, (op - 0x4b) as u16)?, // astore_0..3
            0x3a => store_local(&mut instrs, &mut stack, code[pc + 1] as u16)?,       // astore <idx>
            0x57 => {
                // pop
                pop(&mut stack)?;
            }
            0x59 => {
                // dup
                let top = stack
                    .last()
                    .cloned()
                    .ok_or_else(|| RavaError::Other("dup on empty operand stack".into()))?;
                stack.push(top);
            }
            0x5a => {
                // dup_x1: …, v2, v1 → …, v1, v2, v1
                let v1 = pop(&mut stack)?;
                let v2 = pop(&mut stack)?;
                stack.push(v1.clone());
                stack.push(v2);
                stack.push(v1);
            }
            0x5c => {
                // dup2: …, v2, v1 → …, v2, v1, v2, v1 (two category-1 values, e.g. arrayref+index)
                let n = stack.len();
                if n < 2 {
                    return Err(RavaError::Other("dup2 on short operand stack".into()));
                }
                let a = stack[n - 2].clone();
                let b = stack[n - 1].clone();
                stack.push(a);
                stack.push(b);
            }
            0x5f => {
                // swap
                let n = stack.len();
                if n < 2 {
                    return Err(RavaError::Other("swap on short operand stack".into()));
                }
                stack.swap(n - 1, n - 2);
            }
            0xbb => {
                // new <class index>
                let cls = class.class_name_at(read_u16(code, pc + 1)?)?;
                let r = fresh();
                instrs.push(RirInstr::New {
                    class: ClassId(crate::lowerer_hash::encode_builtin(&cls)),
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0xbc => {
                // newarray <atype> — primitive array (interpreter stores Int slots)
                let len = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::NewArray {
                    elem_type: RirType::I32,
                    len,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0xbd => {
                // anewarray <class index> — reference array
                let len = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::NewArray {
                    elem_type: RirType::Ref(ClassId(0)),
                    len,
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0xbe => {
                // arraylength
                let arr = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::ArrayLen { arr, ret: r.clone() });
                stack.push(r);
            }
            0x2e..=0x35 => {
                // *aload (array load): …, arrayref, index → value
                let idx = pop(&mut stack)?;
                let arr = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::ArrayLoad { arr, idx, ret: r.clone() });
                stack.push(r);
            }
            0x4f..=0x56 => {
                // *astore (array store): …, arrayref, index, value →
                let val = pop(&mut stack)?;
                let idx = pop(&mut stack)?;
                let arr = pop(&mut stack)?;
                instrs.push(RirInstr::ArrayStore { arr, idx, val });
            }
            0xb2 => {
                // getstatic <fieldref index>
                let (cls, name, _desc) = class.field_ref(read_u16(code, pc + 1)?)?;
                let r = fresh();
                if cls == "java/lang/System" && (name == "out" || name == "err") {
                    // Sentinel receiver for System.out/err; println routing handles the call.
                    instrs.push(RirInstr::ConstStr {
                        ret: r.clone(),
                        value: format!("__system_{name}__"),
                    });
                } else {
                    instrs.push(RirInstr::GetStatic {
                        field: FieldId(crate::lowerer_hash::encode_builtin(&name)),
                        ret: r.clone(),
                    });
                }
                stack.push(r);
            }
            0xb4 => {
                // getfield <fieldref index>
                let (_cls, name, _desc) = class.field_ref(read_u16(code, pc + 1)?)?;
                let obj = pop(&mut stack)?;
                let r = fresh();
                instrs.push(RirInstr::GetField {
                    obj,
                    field: FieldId(crate::lowerer_hash::encode_builtin(&name)),
                    ret: r.clone(),
                });
                stack.push(r);
            }
            0xb5 => {
                // putfield <fieldref index>
                let (_cls, name, _desc) = class.field_ref(read_u16(code, pc + 1)?)?;
                let val = pop(&mut stack)?;
                let obj = pop(&mut stack)?;
                instrs.push(RirInstr::SetField {
                    obj,
                    field: FieldId(crate::lowerer_hash::encode_builtin(&name)),
                    val,
                });
            }
            0xb6 | 0xb7 => {
                // invokevirtual / invokespecial — receiver + args (direct call; no vtable yet)
                let (cls, name, desc) = class.method_ref(read_u16(code, pc + 1)?)?;
                let argc = parse_arg_types(&desc)?.len();
                let mut args = vec![Value(String::new()); argc + 1]; // slot 0 = receiver
                for slot in args.iter_mut().skip(1).rev() {
                    *slot = pop(&mut stack)?;
                }
                args[0] = pop(&mut stack)?;
                if cls == "java/io/PrintStream" && (name == "println" || name == "print") {
                    // System.out.println(x) → route to the builtin; drop the PrintStream receiver.
                    let call_args: Vec<Value> = args.into_iter().skip(1).collect();
                    instrs.push(RirInstr::Call {
                        func: FuncId(crate::lowerer_hash::encode_builtin(&format!("System.out.{name}"))),
                        args: call_args,
                        ret: None,
                    });
                } else if op == 0xb7 && name == "<init>" && cls.contains('/') {
                    // A library constructor (java/lang/Object.<init>, …) is a no-op: the object
                    // is already allocated by `new`.
                    // discard
                } else {
                    // Library instance methods (String.length, …) dispatch by name on the
                    // receiver via the interpreter's builtins; user methods call by Class.method.
                    let func_name = if cls.contains('/') {
                        format!("__method__{name}")
                    } else {
                        format!("{cls}.{name}")
                    };
                    let ret = if matches!(parse_return_type(&desc)?, RirType::Void) {
                        None
                    } else {
                        Some(fresh())
                    };
                    instrs.push(RirInstr::Call {
                        func: FuncId(crate::lowerer_hash::encode_builtin(&func_name)),
                        args,
                        ret: ret.clone(),
                    });
                    if let Some(r) = ret {
                        stack.push(r);
                    }
                }
            }
            0xb8 => {
                // invokestatic <methodref index> — direct call, no receiver
                let (cls, name, desc) = class.method_ref(read_u16(code, pc + 1)?)?;
                let argc = parse_arg_types(&desc)?.len();
                let mut args = vec![Value(String::new()); argc];
                for slot in args.iter_mut().rev() {
                    *slot = pop(&mut stack)?;
                }
                // Library static methods use the simple class name (Math.max, Integer.parseInt)
                // to match the interpreter's builtin keys; user methods use Class.method.
                let target = if cls.contains('/') {
                    format!("{}.{}", cls.rsplit('/').next().unwrap_or(&cls), name)
                } else {
                    format!("{cls}.{name}")
                };
                let ret = if matches!(parse_return_type(&desc)?, RirType::Void) {
                    None
                } else {
                    Some(fresh())
                };
                instrs.push(RirInstr::Call {
                    func: FuncId(crate::lowerer_hash::encode_builtin(&target)),
                    args,
                    ret: ret.clone(),
                });
                if let Some(r) = ret {
                    stack.push(r);
                }
            }
            0xa7 => {
                let target = (pc as i64 + read_i16(code, pc + 1)? as i64) as usize;
                instrs.push(RirInstr::Jump(BlockId(target as u32)));
                terminated = true;
            }
            0xac..=0xb0 => {
                // {i,l,f,d,a}return
                let v = pop(&mut stack)?;
                instrs.push(RirInstr::Return(Some(v)));
                terminated = true;
            }
            0xb1 => {
                instrs.push(RirInstr::Return(None));
                terminated = true;
            }
            0xbf => {
                // athrow — throw the top-of-stack exception (uncaught → propagates).
                let v = pop(&mut stack)?;
                instrs.push(RirInstr::Throw(v));
                terminated = true;
            }
            0xb9 => {
                // invokeinterface <methodref> <count> 0 — virtual dispatch on the receiver
                // (resolves the impl on the receiver's actual class, and invokes lambdas).
                let (_cls, name, desc) = class.method_ref(read_u16(code, pc + 1)?)?;
                let argc = parse_arg_types(&desc)?.len();
                let mut args = vec![Value(String::new()); argc];
                for slot in args.iter_mut().rev() {
                    *slot = pop(&mut stack)?;
                }
                let receiver = pop(&mut stack)?;
                let ret = if matches!(parse_return_type(&desc)?, RirType::Void) {
                    None
                } else {
                    Some(fresh())
                };
                instrs.push(RirInstr::CallVirtual {
                    receiver,
                    method: MethodId(crate::lowerer_hash::encode_builtin(&name)),
                    args,
                    ret: ret.clone(),
                });
                if let Some(r) = ret {
                    stack.push(r);
                }
            }
            0xba => {
                // invokedynamic — string concatenation or a non-capturing lambda.
                let cp_idx = read_u16(code, pc + 1)?;
                let concat = match class.indy_concat(cp_idx)? {
                    Some(c) => c,
                    None => {
                        if let Some((lcls, lmethod, ldesc)) = class.indy_lambda(cp_idx)? {
                            if !parse_arg_types(&ldesc)?.is_empty() {
                                return Err(RavaError::Other(
                                    "capturing lambdas are not supported yet (bytecode→RIR)".into(),
                                ));
                            }
                            // A non-capturing lambda → a method-ref value the interpreter invokes
                            // when the functional-interface method (apply/test/…) is called.
                            let r = fresh();
                            instrs.push(RirInstr::ConstStr {
                                ret: r.clone(),
                                value: format!("__methodref__{lcls}.{lmethod}"),
                            });
                            stack.push(r);
                            pc += len;
                            continue;
                        }
                        return Err(RavaError::Other(
                            "unsupported invokedynamic (only string concat and non-capturing lambdas)"
                                .into(),
                        ));
                    }
                };
                let argc = parse_arg_types(&concat.descriptor)?.len();
                let mut args = vec![Value(String::new()); argc];
                for slot in args.iter_mut().rev() {
                    *slot = pop(&mut stack)?;
                }
                // Accumulate into a string (starts as "" so RIR Add concatenates).
                let mut acc = fresh();
                instrs.push(RirInstr::ConstStr {
                    ret: acc.clone(),
                    value: String::new(),
                });
                macro_rules! append {
                    ($piece:expr) => {{
                        let next = fresh();
                        instrs.push(RirInstr::BinOp {
                            op: BinOp::Add,
                            lhs: acc.clone(),
                            rhs: $piece,
                            ret: next.clone(),
                        });
                        acc = next;
                    }};
                }
                macro_rules! append_lit {
                    ($s:expr) => {{
                        let lit = fresh();
                        instrs.push(RirInstr::ConstStr {
                            ret: lit.clone(),
                            value: $s,
                        });
                        append!(lit);
                    }};
                }
                match &concat.recipe {
                    None => {
                        for a in args {
                            append!(a);
                        }
                    }
                    Some(recipe) => {
                        let mut arg_iter = args.into_iter();
                        let mut const_iter = concat.const_args.iter();
                        let mut literal = String::new();
                        for ch in recipe.chars() {
                            match ch {
                                '\u{0001}' => {
                                    if !literal.is_empty() {
                                        append_lit!(std::mem::take(&mut literal));
                                    }
                                    if let Some(a) = arg_iter.next() {
                                        append!(a);
                                    }
                                }
                                '\u{0002}' => {
                                    if !literal.is_empty() {
                                        append_lit!(std::mem::take(&mut literal));
                                    }
                                    if let Some(c) = const_iter.next() {
                                        append_lit!(c.clone());
                                    }
                                }
                                _ => literal.push(ch),
                            }
                        }
                        if !literal.is_empty() {
                            append_lit!(literal);
                        }
                    }
                }
                stack.push(acc);
            }
            0x00 => {} // nop
            other => return Err(unsupported(other)),
        }
        pc += len;
    }

    if !terminated {
        instrs.push(RirInstr::Jump(BlockId(end as u32)));
    }
    let mut blocks = Vec::with_capacity(1 + extra_blocks.len());
    blocks.push(BasicBlock {
        id: BlockId(start as u32),
        params: Vec::new(),
        instrs,
    });
    blocks.append(&mut extra_blocks);
    Ok(blocks)
}

/// Block-id flag distinguishing synthetic blocks (switch comparison chains) from
/// blocks keyed by bytecode offset. Real offsets are < 2^16, so the high bit is free.
const SYNTH_FLAG: u32 = 0x8000_0000;

fn read_i32(code: &[u8], at: usize) -> Result<i32> {
    if at + 4 > code.len() {
        return Err(RavaError::Other("truncated switch operand".into()));
    }
    Ok(i32::from_be_bytes([
        code[at],
        code[at + 1],
        code[at + 2],
        code[at + 3],
    ]))
}

/// Parse a tableswitch/lookupswitch at `pc`. Returns (total length, (match, target) cases,
/// default target). Targets are absolute bytecode offsets.
fn parse_switch(code: &[u8], pc: usize) -> Result<(usize, Vec<(i64, usize)>, usize)> {
    let op = code[pc];
    let mut p = pc + 1 + (3 - (pc % 4)); // pad so operands start 4-byte aligned
    let default = (pc as i64 + read_i32(code, p)? as i64) as usize;
    p += 4;
    let mut cases = Vec::new();
    if op == 0xaa {
        let low = read_i32(code, p)?;
        p += 4;
        let high = read_i32(code, p)?;
        p += 4;
        for k in low..=high {
            let off = read_i32(code, p)?;
            p += 4;
            cases.push((k as i64, (pc as i64 + off as i64) as usize));
        }
    } else {
        let npairs = read_i32(code, p)?.max(0) as usize;
        p += 4;
        for _ in 0..npairs {
            let m = read_i32(code, p)?;
            let off = read_i32(code, p + 4)?;
            p += 8;
            cases.push((m as i64, (pc as i64 + off as i64) as usize));
        }
    }
    Ok((p - pc, cases, default))
}

// ── Instruction helpers ───────────────────────────────────────────────────────

/// Type-preserving identity copy `dst = src` for local stores. Uses a `RawPtr` Convert
/// because the interpreter passes those through unchanged (an `I32` Convert would coerce
/// the value to an int, destroying object references).
fn copy(src: Value, dst: Value) -> RirInstr {
    RirInstr::Convert {
        val: src,
        from: RirType::RawPtr,
        to: RirType::RawPtr,
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
    // {i,l,f,d}{add,sub,mul,div,rem} are laid out in groups of four from 0x60.
    match (op - 0x60) / 4 {
        0 => BinOp::Add,
        1 => BinOp::Sub,
        2 => BinOp::Mul,
        3 => BinOp::Div,
        _ => BinOp::Rem,
    }
}

fn bitwise_op(op: u8) -> BinOp {
    // {i,l}{shl,shr,ushr,and,or,xor} are laid out in pairs from 0x78.
    match (op - 0x78) / 2 {
        0 => BinOp::Shl,
        1 => BinOp::Shr,
        2 => BinOp::UShr,
        3 => BinOp::BitAnd,
        4 => BinOp::BitOr,
        _ => BinOp::Xor,
    }
}

/// Target RIR type for a numeric conversion opcode (i2l, i2d, l2i, d2i, …).
fn convert_target(op: u8) -> RirType {
    match op {
        0x85 | 0x8c | 0x8f => RirType::I64, // i2l, f2l, d2l
        0x86 | 0x89 | 0x90 => RirType::F32, // i2f, l2f, d2f
        0x87 | 0x8a | 0x8d => RirType::F64, // i2d, l2d, f2d
        0x88 | 0x8b | 0x8e => RirType::I32, // l2i, f2i, d2i
        0x91 => RirType::I8,                // i2b
        _ => RirType::I16,                  // i2c (0x92), i2s (0x93)
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
        // 2-byte (1 operand): bipush, ldc, *load <idx>, *store <idx>, newarray
        0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xbc => 2,
        // 3-byte (2 operands): sipush, ldc_w, ldc2_w, iinc, if<cond>, goto, getstatic,
        //   get/putfield, invoke{virtual,special,static}, new, anewarray
        0x11 | 0x13 | 0x14 | 0x84 | 0x99..=0xa4 | 0xa7 | 0xb2 | 0xb4..=0xb8 | 0xbb | 0xbd => 3,
        // 1-byte
        0x00 => 1,        // nop
        0x02..=0x0f => 1, // iconst/lconst/fconst/dconst
        0x1a..=0x35 => 1, // {i,l,f,d,a}load_0..3 + *aload (array load)
        0x3b..=0x56 => 1, // {i,l,f,d,a}store_0..3 + *astore (array store)
        0x57..=0x5f => 1, // pop/pop2/dup/dup_x1/dup_x2/dup2/.../swap
        0x60..=0x83 => 1, // arithmetic + negate + shifts/bitwise (int/long/float/double)
        0x85..=0x93 => 1, // numeric conversions
        0xac..=0xb1 => 1, // *return
        0xbe | 0xbf => 1, // arraylength, athrow
        0xb9 | 0xba => 5, // invokeinterface, invokedynamic
        _ => return None,
    })
}

fn unsupported(op: u8) -> RavaError {
    RavaError::Other(format!(
        "unsupported JVM bytecode opcode 0x{op:02x} (bytecode→RIR covers the integer subset \
         with control flow so far)"
    ))
}

fn read_u16(code: &[u8], at: usize) -> Result<u16> {
    Ok(read_i16(code, at)? as u16)
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

    /// Lower every (lowerable) method of a class into one module, then invoke `method`.
    fn run_class(fixture: &[u8], method: &str, args: Vec<RVal>) -> i64 {
        let cf = classfile::parse(fixture).unwrap();
        let mut module = RirModule::new("M");
        for f in &cf.fields {
            module
                .field_names
                .insert(crate::lowerer_hash::encode_builtin(f), f.clone());
        }
        for (i, m) in cf.methods.iter().enumerate() {
            if let Ok(func) = lower_method(&cf, m, i as u32) {
                module.functions.push(func);
            }
        }
        let target = format!("{}.{}", cf.name, method);
        RirInterpreter::new(module)
            .call(&target, args)
            .unwrap()
            .to_display()
            .parse()
            .unwrap()
    }

    const ADD: &[u8] = include_bytes!("fixtures/Add.class");
    const CALC: &[u8] = include_bytes!("fixtures/Calc.class");
    const CALLER: &[u8] = include_bytes!("fixtures/Caller.class");
    const OBJ: &[u8] = include_bytes!("fixtures/Obj.class");
    const STR: &[u8] = include_bytes!("fixtures/Str.class");
    const HELLO: &[u8] = include_bytes!("fixtures/Hello.class");
    const CONCAT: &[u8] = include_bytes!("fixtures/Concat.class");
    const INTFN: &[u8] = include_bytes!("fixtures/IntFn.class");
    const LAM: &[u8] = include_bytes!("fixtures/Lam.class");
    const LAM2: &[u8] = include_bytes!("fixtures/Lam2.class");
    const ARR: &[u8] = include_bytes!("fixtures/Arr.class");
    const NUM: &[u8] = include_bytes!("fixtures/Num.class");
    const SW: &[u8] = include_bytes!("fixtures/Sw.class");
    const BITS: &[u8] = include_bytes!("fixtures/Bits.class");
    const EXC: &[u8] = include_bytes!("fixtures/Exc.class");
    const STK: &[u8] = include_bytes!("fixtures/Stk.class");
    const APPEXC: &[u8] = include_bytes!("fixtures/AppExc.class");
    const TRYC: &[u8] = include_bytes!("fixtures/TryC.class");
    const MATHLIB: &[u8] = include_bytes!("fixtures/MathLib.class");
    const APP: &[u8] = include_bytes!("fixtures/App.class");

    /// Lower a class, run its `main`, and capture stdout.
    fn run_main_capture(fixture: &[u8]) -> String {
        let cf = classfile::parse(fixture).unwrap();
        let mut module = RirModule::new("M");
        for f in &cf.fields {
            module
                .field_names
                .insert(crate::lowerer_hash::encode_builtin(f), f.clone());
        }
        for (i, m) in cf.methods.iter().enumerate() {
            if let Ok(func) = lower_method(&cf, m, i as u32) {
                module.functions.push(func);
            }
        }
        let mut buf = Vec::new();
        RirInterpreter::new(module)
            .run_main_with_output(&mut buf)
            .unwrap();
        String::from_utf8(buf).unwrap()
    }

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

    #[test]
    fn static_method_calls() {
        // invokestatic: methods calling other static methods, plus recursion
        assert_eq!(run_class(CALLER, "square", vec![RVal::Int(6)]), 36);
        assert_eq!(run_class(CALLER, "sumSquares", vec![RVal::Int(3), RVal::Int(4)]), 25);
        assert_eq!(run_class(CALLER, "fib", vec![RVal::Int(10)]), 55);
    }

    #[test]
    fn objects_and_instance_methods() {
        // new + putfield + getfield (default constructor; super Object.<init> no-op)
        assert_eq!(run_class(OBJ, "make", vec![RVal::Int(3), RVal::Int(4)]), 7);
        // new + putfield + invokevirtual (instance method reading fields)
        assert_eq!(run_class(OBJ, "useSum", vec![RVal::Int(10), RVal::Int(20)]), 30);
    }

    #[test]
    fn strings_and_library_calls() {
        // ldc string constant + library invokevirtual (String.length / substring)
        assert_eq!(run_class(STR, "helloLen", vec![]), 11);
        assert_eq!(run_class(STR, "subLen", vec![]), 5);
        // library invokestatic (Integer.parseInt) routed to the builtin
        assert_eq!(run_class(STR, "parsed", vec![]), 42);
    }

    #[test]
    fn stack_ops_compound_assign() {
        // a[0] += 5 then *= 2 uses dup2 (duplicate arrayref+index): ((10+5)*2) = 30
        assert_eq!(run_class(STK, "compound", vec![RVal::Int(10)]), 30);
    }

    #[test]
    fn try_catch() {
        // try { if (n<0) throw new AppExc(); return n*2; } catch (AppExc e) { return -1; }
        let module = load_classes_module(&[APPEXC, TRYC]).unwrap();
        let interp = RirInterpreter::new(module);
        assert_eq!(interp.call("TryC.f", vec![RVal::Int(5)]).unwrap().to_display(), "10"); // no throw
        assert_eq!(interp.call("TryC.f", vec![RVal::Int(-1)]).unwrap().to_display(), "-1"); // caught
    }

    #[test]
    fn throw_propagates() {
        let cf = classfile::parse(EXC).unwrap();
        let mut module = RirModule::new("M");
        for (i, m) in cf.methods.iter().enumerate() {
            if let Ok(f) = lower_method(&cf, m, i as u32) {
                module.functions.push(f);
            }
        }
        let interp = RirInterpreter::new(module);
        // non-throwing path returns a value
        assert_eq!(interp.call("Exc.checked", vec![RVal::Int(5)]).unwrap().to_display(), "10");
        // throwing path (new + athrow) propagates as an error (no catch in bytecode yet)
        assert!(interp.call("Exc.checked", vec![RVal::Int(-1)]).is_err());
    }

    #[test]
    fn switches() {
        // tableswitch (dense 1..3)
        assert_eq!(run_class(SW, "tbl", vec![RVal::Int(1)]), 10);
        assert_eq!(run_class(SW, "tbl", vec![RVal::Int(2)]), 20);
        assert_eq!(run_class(SW, "tbl", vec![RVal::Int(3)]), 30);
        assert_eq!(run_class(SW, "tbl", vec![RVal::Int(9)]), -1); // default
        // lookupswitch (sparse 100, 500)
        assert_eq!(run_class(SW, "look", vec![RVal::Int(100)]), 1);
        assert_eq!(run_class(SW, "look", vec![RVal::Int(500)]), 5);
        assert_eq!(run_class(SW, "look", vec![RVal::Int(7)]), 0); // default
    }

    #[test]
    fn bitwise_ops() {
        // shifts (<<, >>) + and/or/xor: ((5<<2)|(7&3)) ^ (5>>1) = (20|3)^2 = 23^2 = 21
        assert_eq!(run_class(BITS, "ops", vec![RVal::Int(5), RVal::Int(7)]), 21);
        // >>> on non-negative matches the JVM (negatives differ: our int is i64, not i32)
        assert_eq!(run_class(BITS, "ushr", vec![RVal::Int(8)]), 4);
    }

    #[test]
    fn long_and_double() {
        // long arithmetic (i64), ldc2_w long const, lneg
        assert_eq!(run_class(NUM, "mul", vec![RVal::Int(1_000_000), RVal::Int(1_000_000)]), 1_000_000_000_000);
        assert_eq!(run_class(NUM, "withConst", vec![RVal::Int(7)]), 7_000_005);
        assert_eq!(run_class(NUM, "negate", vec![RVal::Int(5)]), -5);
        // double machinery: i2d, ldc2_w double const, dmul, d2i → (int)(4 * 2.5) = 10
        assert_eq!(run_class(NUM, "doubleStuff", vec![RVal::Int(4)]), 10);
    }

    #[test]
    fn arrays() {
        // new int[n], iastore in a loop, iaload, arraylength
        assert_eq!(run_class(ARR, "sumSquares", vec![RVal::Int(5)]), 30); // 0+1+4+9+16
        assert_eq!(run_class(ARR, "len", vec![]), 3);
    }

    #[test]
    fn cross_class_calls() {
        // Two classes in one module (like a JAR): App calls MathLib (static + instance).
        let module = load_classes_module(&[MATHLIB, APP]).unwrap();
        let interp = RirInterpreter::new(module);
        // App.compute(5) = MathLib.triple(5) + 1 = 16  (cross-class invokestatic)
        assert_eq!(interp.call("App.compute", vec![RVal::Int(5)]).unwrap().to_display(), "16");
        // App.viaInstance(4) = new MathLib(10).scaled(4) = 40  (cross-class new + invokevirtual)
        assert_eq!(
            interp.call("App.viaInstance", vec![RVal::Int(4)]).unwrap().to_display(),
            "40"
        );
    }

    #[test]
    fn multi_jar_classpath() {
        // A JAR (App3) plus its dependency JAR (MathLib) loaded together, like a classpath.
        let app = include_bytes!("fixtures/app3.jar");
        let lib = include_bytes!("fixtures/libmath.jar");
        let module = load_jars(&[app, lib]).unwrap();
        let mut buf = Vec::new();
        RirInterpreter::new(module)
            .run_main_with_output(&mut buf)
            .unwrap();
        // App3.main: MathLib.triple(7)=21; new MathLib(10).scaled(5)=50 (cross-JAR calls)
        assert_eq!(String::from_utf8(buf).unwrap().trim(), "21\n50");
    }

    #[test]
    fn run_jar() {
        // A real JAR (ZIP of App2.class + MathLib.class with a Main-Class manifest).
        let module = load_jar(include_bytes!("fixtures/app.jar")).unwrap();
        let mut buf = Vec::new();
        RirInterpreter::new(module)
            .run_main_with_output(&mut buf)
            .unwrap();
        // App2.main: println(MathLib.triple(7))=21; println(new MathLib(10).scaled(5))=50
        assert_eq!(String::from_utf8(buf).unwrap().trim(), "21\n50");
    }

    #[test]
    fn non_capturing_lambda() {
        // IntFn f = n -> n*2 (invokedynamic/LambdaMetafactory) + f.apply (invokeinterface)
        let module = load_classes_module(&[INTFN, LAM]).unwrap();
        let interp = RirInterpreter::new(module);
        // twice(5) = f(f(5)) = f(10) = 20
        assert_eq!(interp.call("Lam.twice", vec![RVal::Int(5)]).unwrap().to_display(), "20");
    }

    #[test]
    fn lambda_functional_interfaces() {
        // IntUnaryOperator (applyAsInt) and IntBinaryOperator — non-capturing lambdas
        let module = load_classes_module(&[LAM2]).unwrap();
        let interp = RirInterpreter::new(module);
        assert_eq!(interp.call("Lam2.op", vec![RVal::Int(5)]).unwrap().to_display(), "105");
        assert_eq!(
            interp.call("Lam2.combine", vec![RVal::Int(3), RVal::Int(4)]).unwrap().to_display(),
            "13"
        );
    }

    #[test]
    fn string_concat_invokedynamic() {
        // "x=" + x and "Hi " + name + ", you have " + n + " msgs" via makeConcatWithConstants
        let out = run_main_capture(CONCAT);
        assert_eq!(out.trim(), "x=5\nHi Bob, you have 3 msgs");
    }

    #[test]
    fn main_with_println() {
        // Full program: getstatic System.out + ldc + println (string and int) + a loop.
        let out = run_main_capture(HELLO);
        assert_eq!(out.trim(), "Hello from compiled bytecode!\n15");
    }
}
