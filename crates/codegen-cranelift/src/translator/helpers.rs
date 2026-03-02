//! Free helper functions for RIR → Cranelift translation.

use cranelift_codegen::ir::{self, types, AbiParam, Signature};
use cranelift_codegen::isa::CallConv;
use rava_rir::{RirFunction, RirInstr, RirType};

pub(super) fn build_signature(func: &RirFunction) -> Signature {
    let mut sig = Signature::new(CallConv::SystemV);
    for (_, ty) in &func.params {
        sig.params.push(AbiParam::new(rir_type_to_clif(ty)));
    }
    match &func.return_type {
        RirType::Void => {}
        ty => {
            sig.returns.push(AbiParam::new(rir_type_to_clif(ty)));
        }
    }
    sig
}

pub(super) fn rir_type_to_clif(ty: &RirType) -> ir::Type {
    match ty {
        RirType::I8 | RirType::I16 | RirType::I32 | RirType::Bool => types::I64,
        RirType::I64 => types::I64,
        RirType::F32 => types::I64, // stored as bits
        RirType::F64 => types::I64, // stored as bits
        RirType::Ref(_) | RirType::Array(_) | RirType::RawPtr => types::I64,
        RirType::Void => types::I64, // shouldn't happen for params
    }
}

pub(super) fn block_ends_with_terminator(instrs: &[RirInstr]) -> bool {
    matches!(
        instrs.last(),
        Some(
            RirInstr::Return(_)
                | RirInstr::Jump(_)
                | RirInstr::Branch { .. }
                | RirInstr::Unreachable
                | RirInstr::Throw(_)
        )
    )
}

/// Collect all value names defined by an instruction.
pub(super) fn collect_def_names(instr: &RirInstr, names: &mut Vec<String>) {
    match instr {
        RirInstr::ConstInt { ret, .. }
        | RirInstr::ConstFloat { ret, .. }
        | RirInstr::ConstStr { ret, .. }
        | RirInstr::ConstBool { ret, .. }
        | RirInstr::ConstNull { ret } => {
            names.push(ret.0.clone());
        }
        RirInstr::BinOp { ret, .. } | RirInstr::UnaryOp { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::Call { ret: Some(ret), .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::New { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::GetField { ret, .. } | RirInstr::GetStatic { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::NewArray { ret, .. }
        | RirInstr::NewMultiArray { ret, .. }
        | RirInstr::ArrayLoad { ret, .. }
        | RirInstr::ArrayLen { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::Instanceof { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::Convert { ret, .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::CallVirtual { ret: Some(ret), .. }
        | RirInstr::CallInterface { ret: Some(ret), .. } => {
            names.push(ret.0.clone());
        }
        RirInstr::MicroRtReflect { ret, .. }
        | RirInstr::MicroRtProxy { ret, .. }
        | RirInstr::MicroRtClassLoad { ret, .. } => {
            names.push(ret.0.clone());
        }
        _ => {}
    }
}

/// Mangle a Java-style name (e.g. `Main.main`) to a C-compatible symbol (`Main_main`).
pub(super) fn mangle_name(name: &str) -> String {
    name.replace(['.', '<', '>'], "_")
}
