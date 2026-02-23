//! Free helper functions used across the lowerer.

use super::*;

pub(crate) fn is_static_path(path: &str) -> bool {
    if matches!(path,
        "System.out" | "System.err" | "System.in" |
        "Math" | "String" | "Integer" | "Long" | "Double" |
        "Float" | "Boolean" | "Character" | "Byte" | "Short" |
        "Arrays" | "Collections" | "Objects" |
        "System" | "Runtime" | "Thread" | "List"
    ) || path.starts_with("System.")
      || path.starts_with("Math.")
    {
        return true;
    }
    if path.contains('.') {
        return false;
    }
    path.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
}

pub(crate) fn lower_type(ty: &TypeExpr) -> RirType {
    if ty.array_dims > 0 {
        return RirType::Array(Box::new(lower_type_name(&ty.name)));
    }
    lower_type_name(&ty.name)
}

pub(crate) fn lower_type_name(name: &str) -> RirType {
    match name {
        "int" | "short" | "byte" | "char" => RirType::I32,
        "long"    => RirType::I64,
        "float"   => RirType::F32,
        "double"  => RirType::F64,
        "boolean" => RirType::Bool,
        "void"    => RirType::Void,
        _         => RirType::Ref(ClassId(encode_builtin(name))),
    }
}

pub(crate) fn lower_binop(op: &BinOp) -> RirBinOp {
    match op {
        BinOp::Add    => RirBinOp::Add,
        BinOp::Sub    => RirBinOp::Sub,
        BinOp::Mul    => RirBinOp::Mul,
        BinOp::Div    => RirBinOp::Div,
        BinOp::Rem    => RirBinOp::Rem,
        BinOp::Eq     => RirBinOp::Eq,
        BinOp::Ne     => RirBinOp::Ne,
        BinOp::Lt     => RirBinOp::Lt,
        BinOp::Le     => RirBinOp::Le,
        BinOp::Gt     => RirBinOp::Gt,
        BinOp::Ge     => RirBinOp::Ge,
        BinOp::And    => RirBinOp::And,
        BinOp::Or     => RirBinOp::Or,
        BinOp::BitAnd => RirBinOp::BitAnd,
        BinOp::BitOr  => RirBinOp::BitOr,
        BinOp::BitXor => RirBinOp::Xor,
        BinOp::Shl    => RirBinOp::Shl,
        BinOp::Shr    => RirBinOp::Shr,
        BinOp::UShr   => RirBinOp::UShr,
    }
}

/// Encode a string name as a stable u32 (FNV-1a hash, truncated).
pub fn encode_builtin(name: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in name.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

pub(crate) fn expr_to_str(expr: &Expr) -> String {
    match expr {
        Expr::Ident(s)            => s.clone(),
        Expr::This                => "this".into(),
        Expr::Super               => "super".into(),
        Expr::Field { obj, name } => format!("{}.{}", expr_to_str(obj), name),
        _                         => "<expr>".into(),
    }
}
