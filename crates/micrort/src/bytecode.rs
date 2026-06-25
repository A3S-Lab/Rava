//! Lower a parsed `.class` method's stack-based JVM bytecode to RIR, so the existing
//! RIR interpreter can execute **compiled** Java (instead of maintaining a separate
//! bytecode VM). Supports the integer-arithmetic subset today; an unsupported opcode
//! returns an error so the method is simply "not runnable yet" rather than mis-run.

use crate::classfile::{ClassFile, Method};
use rava_common::error::{RavaError, Result};
use rava_rir::module::{BasicBlock, FuncFlags, RirFunction};
use rava_rir::{BinOp, BlockId, ClassId, FuncId, RirInstr, RirType, Value};

/// Lower one method to a single-basic-block RIR function named `Class.method`.
pub fn lower_method(class: &ClassFile, method: &Method, func_id: u32) -> Result<RirFunction> {
    let code = method
        .code
        .as_ref()
        .ok_or_else(|| RavaError::Other(format!("method {} has no Code attribute", method.name)))?;

    // Parameters occupy local slots starting at 0 (or 1 for instance methods: local 0 = this).
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

    let mut lo = Lowering::default();
    lo.lower(code)?;

    Ok(RirFunction {
        id: FuncId(func_id),
        name: format!("{}.{}", class.name, method.name),
        params,
        return_type: parse_return_type(&method.descriptor)?,
        basic_blocks: vec![BasicBlock {
            id: BlockId(0),
            params: Vec::new(),
            instrs: lo.instrs,
        }],
        flags: FuncFlags::default(),
    })
}

#[derive(Default)]
struct Lowering {
    instrs: Vec<RirInstr>,
    stack: Vec<Value>,
    tmp: u32,
}

impl Lowering {
    fn fresh(&mut self) -> Value {
        let v = Value(format!("t{}", self.tmp));
        self.tmp += 1;
        v
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| RavaError::Other("bytecode operand stack underflow".into()))
    }

    fn const_int(&mut self, value: i64) {
        let ret = self.fresh();
        self.instrs.push(RirInstr::ConstInt {
            ret: ret.clone(),
            value,
        });
        self.stack.push(ret);
    }

    fn binop(&mut self, op: BinOp) -> Result<()> {
        let rhs = self.pop()?;
        let lhs = self.pop()?;
        let ret = self.fresh();
        self.instrs.push(RirInstr::BinOp {
            op,
            lhs,
            rhs,
            ret: ret.clone(),
        });
        self.stack.push(ret);
        Ok(())
    }

    fn lower(&mut self, code: &[u8]) -> Result<()> {
        let mut pc = 0;
        while pc < code.len() {
            let op = code[pc];
            pc += 1;
            match op {
                0x1a..=0x1d => self.stack.push(Value(format!("l{}", op - 0x1a))), // iload_0..3
                0x15 => {
                    // iload <index>
                    let idx = *code.get(pc).ok_or_else(eof)?;
                    pc += 1;
                    self.stack.push(Value(format!("l{idx}")));
                }
                0x02..=0x08 => self.const_int(op as i64 - 0x03), // iconst_m1..iconst_5
                0x10 => {
                    // bipush <byte>
                    let b = *code.get(pc).ok_or_else(eof)? as i8;
                    pc += 1;
                    self.const_int(b as i64);
                }
                0x11 => {
                    // sipush <short>
                    let hi = *code.get(pc).ok_or_else(eof)? as i16;
                    let lo = *code.get(pc + 1).ok_or_else(eof)? as i16;
                    pc += 2;
                    self.const_int((((hi << 8) | (lo & 0xff)) as i16) as i64);
                }
                0x60 => self.binop(BinOp::Add)?, // iadd
                0x64 => self.binop(BinOp::Sub)?, // isub
                0x68 => self.binop(BinOp::Mul)?, // imul
                0x6c => self.binop(BinOp::Div)?, // idiv
                0x70 => self.binop(BinOp::Rem)?, // irem
                0xac => {
                    // ireturn
                    let v = self.pop()?;
                    self.instrs.push(RirInstr::Return(Some(v)));
                }
                0xb1 => self.instrs.push(RirInstr::Return(None)), // return
                other => {
                    return Err(RavaError::Other(format!(
                        "unsupported JVM bytecode opcode 0x{other:02x} (bytecode→RIR covers the \
                         integer-arithmetic subset so far)"
                    )))
                }
            }
        }
        Ok(())
    }
}

fn eof() -> RavaError {
    RavaError::Other("unexpected end of bytecode".into())
}

fn slots(ty: &RirType) -> u16 {
    matches!(ty, RirType::I64 | RirType::F64) as u16 + 1
}

/// Parse the argument types from a method descriptor like `(IJ)V`.
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
            // object type `Lpkg/Class;` — consume up to ';'
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

    fn interp_for(method: &str) -> (RirInterpreter, String) {
        let bytes = include_bytes!("fixtures/Add.class");
        let cf = classfile::parse(bytes).unwrap();
        let m = cf.methods.iter().find(|m| m.name == method).unwrap();
        let func = lower_method(&cf, m, 0).unwrap();
        let name = func.name.clone();
        let mut module = RirModule::new("Add");
        module.functions.push(func);
        (RirInterpreter::new(module), name)
    }

    #[test]
    fn executes_compiled_static_add() {
        let (interp, name) = interp_for("add");
        let r = interp
            .call(&name, vec![RVal::Int(2), RVal::Int(3)])
            .unwrap();
        assert_eq!(r.to_display(), "5");
    }

    #[test]
    fn executes_compiled_instance_triple() {
        let (interp, name) = interp_for("triple");
        // instance method: arg 0 is `this` (unused), arg 1 is x
        let r = interp
            .call(&name, vec![RVal::Null, RVal::Int(7)])
            .unwrap();
        assert_eq!(r.to_display(), "21");
    }
}
