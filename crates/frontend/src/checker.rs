//! Lightweight generic semantic checks.

use crate::ast::*;
use rava_common::error::{RavaError, Result};
use std::collections::{HashMap, HashSet};

pub struct GenericChecker;

type SupertypeMap = HashMap<String, Vec<String>>;

#[derive(Debug, Clone)]
struct TypeParamDecl {
    name: String,
    bounds: Vec<String>,
}

#[derive(Debug, Clone)]
struct MethodGenericSig {
    arity: usize,
    bounds: Vec<Vec<String>>, // per type parameter bounds
}

#[derive(Debug, Clone)]
struct MethodParamSig {
    params: Vec<TypeExpr>,
    variadic: bool,
    type_vars: HashSet<String>,
    type_var_bounds: HashMap<String, Vec<String>>,
}

impl GenericChecker {
    pub fn check_file(file: &SourceFile) -> Result<()> {
        let class_hierarchy = Self::build_class_hierarchy(file);
        for class in &file.classes {
            Self::check_type_param_decl(class.type_params_raw.as_deref(), &class.name, "class")?;

            let mut method_generic_arity: HashMap<String, HashSet<usize>> = HashMap::new();
            let mut method_generic_sigs: HashMap<String, Vec<MethodGenericSig>> = HashMap::new();
            let mut method_param_sigs: HashMap<String, Vec<MethodParamSig>> = HashMap::new();
            for member in &class.members {
                match member {
                    Member::Method(m) => {
                        Self::check_type_param_decl(
                            m.type_params_raw.as_deref(),
                            &format!("{}.{}", class.name, m.name),
                            "method",
                        )?;
                        let params = Self::parse_type_param_decls(m.type_params_raw.as_deref());
                        let arity = params.len();
                        let bounds = params.iter().map(|p| p.bounds.clone()).collect();
                        let type_vars = params.iter().map(|p| p.name.clone()).collect();
                        let type_var_bounds = params
                            .iter()
                            .map(|p| (p.name.clone(), p.bounds.clone()))
                            .collect();
                        method_generic_arity
                            .entry(m.name.clone())
                            .or_default()
                            .insert(arity);
                        method_generic_sigs
                            .entry(m.name.clone())
                            .or_default()
                            .push(MethodGenericSig { arity, bounds });
                        method_param_sigs
                            .entry(m.name.clone())
                            .or_default()
                            .push(MethodParamSig {
                                params: m.params.iter().map(|p| p.ty.clone()).collect(),
                                variadic: m.params.last().map(|p| p.variadic).unwrap_or(false),
                                type_vars,
                                type_var_bounds,
                            });
                    }
                    Member::Constructor(c) => {
                        Self::check_type_param_decl(
                            c.type_params_raw.as_deref(),
                            &format!("{}.{}", class.name, c.name),
                            "constructor",
                        )?;
                    }
                    Member::InnerClass(inner) => {
                        Self::check_file(&SourceFile {
                            package: None,
                            imports: vec![],
                            module: None,
                            classes: vec![inner.clone()],
                        })?;
                    }
                    _ => {}
                }
            }

            for member in &class.members {
                match member {
                    Member::Method(m) => {
                        if let Some(body) = &m.body {
                            Self::check_block(
                                body,
                                &class.name,
                                Some(&m.name),
                                &method_generic_arity,
                                &method_generic_sigs,
                                &class_hierarchy,
                            )?;
                            Self::check_block_pecs(
                                body,
                                &class.name,
                                Some(&m.name),
                                &class_hierarchy,
                                &method_param_sigs,
                            )?;
                        }
                    }
                    Member::Constructor(c) => {
                        Self::check_block(
                            &c.body,
                            &class.name,
                            Some(&c.name),
                            &method_generic_arity,
                            &method_generic_sigs,
                            &class_hierarchy,
                        )?;
                        Self::check_block_pecs(
                            &c.body,
                            &class.name,
                            Some(&c.name),
                            &class_hierarchy,
                            &method_param_sigs,
                        )?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn check_type_param_decl(raw: Option<&str>, owner: &str, kind: &str) -> Result<()> {
        let Some(raw) = raw else {
            return Ok(());
        };
        let params = Self::split_top_level_commas(raw);
        for p in params {
            let p = p.trim();
            if p.is_empty() {
                return Err(RavaError::Type {
                    location: owner.to_string(),
                    message: format!("empty {} type parameter", kind),
                });
            }
            let (name, bound) = if let Some((left, right)) = p.split_once("extends") {
                (left.trim(), Some(right.trim()))
            } else {
                (p, None)
            };
            if name.is_empty() {
                return Err(RavaError::Type {
                    location: owner.to_string(),
                    message: format!("{} type parameter missing name", kind),
                });
            }
            if let Some(b) = bound {
                if b.contains('?') {
                    return Err(RavaError::Type {
                        location: owner.to_string(),
                        message: format!(
                            "{} type parameter bound cannot contain wildcard: {}",
                            kind, p
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    fn check_block(
        block: &Block,
        class_name: &str,
        member_name: Option<&str>,
        arity_map: &HashMap<String, HashSet<usize>>,
        sigs_map: &HashMap<String, Vec<MethodGenericSig>>,
        class_hierarchy: &SupertypeMap,
    ) -> Result<()> {
        for stmt in &block.0 {
            Self::check_stmt(
                stmt,
                class_name,
                member_name,
                arity_map,
                sigs_map,
                class_hierarchy,
            )?;
        }
        Ok(())
    }

    fn check_stmt(
        stmt: &Stmt,
        class_name: &str,
        member_name: Option<&str>,
        arity_map: &HashMap<String, HashSet<usize>>,
        sigs_map: &HashMap<String, Vec<MethodGenericSig>>,
        class_hierarchy: &SupertypeMap,
    ) -> Result<()> {
        match stmt {
            Stmt::Expr(e)
            | Stmt::Throw(e)
            | Stmt::Yield(e)
            | Stmt::Assert {
                expr: e,
                message: None,
            } => Self::check_expr(
                e,
                class_name,
                member_name,
                arity_map,
                sigs_map,
                class_hierarchy,
            )?,
            Stmt::Assert {
                expr,
                message: Some(msg),
            } => {
                Self::check_expr(
                    expr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    msg,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::Return(Some(e)) => Self::check_expr(
                e,
                class_name,
                member_name,
                arity_map,
                sigs_map,
                class_hierarchy,
            )?,
            Stmt::Return(None) | Stmt::Empty => {}
            Stmt::LocalVar { init: Some(e), .. } => {
                Self::check_expr(
                    e,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::LocalVar { init: None, .. } => {}
            Stmt::If { cond, then, else_ } => {
                Self::check_expr(
                    cond,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_stmt(
                    then,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                if let Some(e) = else_ {
                    Self::check_stmt(
                        e,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
            }
            Stmt::While { cond, body } => {
                Self::check_expr(
                    cond,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_stmt(
                    body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::DoWhile { body, cond } => {
                Self::check_stmt(
                    body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    cond,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
            } => {
                if let Some(i) = init {
                    Self::check_stmt(
                        i,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
                if let Some(c) = cond {
                    Self::check_expr(
                        c,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
                for u in update {
                    Self::check_expr(
                        u,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
                Self::check_stmt(
                    body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::ForEach { iterable, body, .. } => {
                Self::check_expr(
                    iterable,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_stmt(
                    body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::Block(b) => Self::check_block(
                b,
                class_name,
                member_name,
                arity_map,
                sigs_map,
                class_hierarchy,
            )?,
            Stmt::Switch { expr, cases } => {
                Self::check_expr(
                    expr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                for case in cases {
                    if let Some(labels) = &case.labels {
                        for lbl in labels {
                            Self::check_expr(
                                lbl,
                                class_name,
                                member_name,
                                arity_map,
                                sigs_map,
                                class_hierarchy,
                            )?;
                        }
                    }
                    for s in &case.body {
                        Self::check_stmt(
                            s,
                            class_name,
                            member_name,
                            arity_map,
                            sigs_map,
                            class_hierarchy,
                        )?;
                    }
                }
            }
            Stmt::TryCatch {
                try_body,
                catches,
                finally_body,
            } => {
                Self::check_block(
                    try_body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                for c in catches {
                    Self::check_block(
                        &c.body,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
                if let Some(f) = finally_body {
                    Self::check_block(
                        f,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
            }
            Stmt::Labeled { stmt, .. } => {
                Self::check_stmt(
                    stmt,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::Synchronized { expr, body } => {
                Self::check_expr(
                    expr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_block(
                    body,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
        Ok(())
    }

    fn check_expr(
        expr: &Expr,
        class_name: &str,
        member_name: Option<&str>,
        arity_map: &HashMap<String, HashSet<usize>>,
        sigs_map: &HashMap<String, Vec<MethodGenericSig>>,
        class_hierarchy: &SupertypeMap,
    ) -> Result<()> {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args_raw,
            } => {
                if let Some(raw) = type_args_raw {
                    let actual = Self::count_type_args(raw);
                    if let Some(method_name) = Self::callee_method_name(callee) {
                        if let Some(expected_set) = arity_map.get(method_name) {
                            if !expected_set.contains(&actual) {
                                let mut expected: Vec<_> = expected_set.iter().copied().collect();
                                expected.sort_unstable();
                                return Err(RavaError::Type {
                                    location: format!(
                                        "{}.{}",
                                        class_name,
                                        member_name.unwrap_or("<init>")
                                    ),
                                    message: format!(
                                        "explicit type args for '{}' have arity {}, expected one of {:?}",
                                        method_name, actual, expected
                                    ),
                                });
                            }
                            if let Some(candidates) = sigs_map.get(method_name) {
                                let explicit_args = Self::parse_explicit_type_args(raw);
                                let ok = candidates
                                    .iter()
                                    .filter(|sig| sig.arity == explicit_args.len())
                                    .any(|sig| {
                                        sig.bounds.iter().enumerate().all(|(i, bounds)| {
                                            bounds.iter().all(|bound| {
                                                Self::satisfies_bound(
                                                    &explicit_args[i],
                                                    bound,
                                                    class_hierarchy,
                                                )
                                            })
                                        })
                                    });
                                if !ok {
                                    return Err(RavaError::Type {
                                        location: format!(
                                            "{}.{}",
                                            class_name,
                                            member_name.unwrap_or("<init>")
                                        ),
                                        message: format!(
                                            "explicit type args for '{}' violate declared bounds",
                                            method_name
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
                Self::check_expr(
                    callee,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                for arg in args {
                    Self::check_expr(
                        arg,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
            }
            Expr::Field { obj, .. }
            | Expr::UnaryOp { expr: obj, .. }
            | Expr::Cast { expr: obj, .. }
            | Expr::Instanceof { expr: obj, .. }
            | Expr::InstanceofPattern { expr: obj, .. }
            | Expr::MethodRef { obj, .. } => {
                Self::check_expr(
                    obj,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::Index { arr, idx } => {
                Self::check_expr(
                    arr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    idx,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::BinOp { lhs, rhs, .. }
            | Expr::Assign { lhs, rhs }
            | Expr::CompoundAssign { lhs, rhs, .. } => {
                Self::check_expr(
                    lhs,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    rhs,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::Ternary { cond, then, else_ } => {
                Self::check_expr(
                    cond,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    then,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                Self::check_expr(
                    else_,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::New { args, body, .. } => {
                for arg in args {
                    Self::check_expr(
                        arg,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
                if let Some(members) = body {
                    for m in members {
                        if let Member::Method(mm) = m {
                            if let Some(b) = &mm.body {
                                Self::check_block(
                                    b,
                                    class_name,
                                    Some(&mm.name),
                                    arity_map,
                                    sigs_map,
                                    class_hierarchy,
                                )?;
                            }
                        }
                    }
                }
            }
            Expr::NewArray { len, .. } => {
                Self::check_expr(
                    len,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::NewMultiArray { dims, .. } => {
                for d in dims {
                    Self::check_expr(
                        d,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
            }
            Expr::ArrayInit { elements, .. } => {
                for e in elements {
                    Self::check_expr(
                        e,
                        class_name,
                        member_name,
                        arity_map,
                        sigs_map,
                        class_hierarchy,
                    )?;
                }
            }
            Expr::SwitchExpr { expr, cases } => {
                Self::check_expr(
                    expr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
                for case in cases {
                    if let Some(labels) = &case.labels {
                        for lbl in labels {
                            Self::check_expr(
                                lbl,
                                class_name,
                                member_name,
                                arity_map,
                                sigs_map,
                                class_hierarchy,
                            )?;
                        }
                    }
                    for s in &case.body {
                        Self::check_stmt(
                            s,
                            class_name,
                            member_name,
                            arity_map,
                            sigs_map,
                            class_hierarchy,
                        )?;
                    }
                }
            }
            Expr::Lambda { body, .. } => match &**body {
                LambdaBody::Expr(e) => Self::check_expr(
                    e,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?,
                LambdaBody::Block(b) => Self::check_block(
                    b,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?,
            },
            Expr::RecordPattern { expr, .. } => {
                Self::check_expr(
                    expr,
                    class_name,
                    member_name,
                    arity_map,
                    sigs_map,
                    class_hierarchy,
                )?;
            }
            Expr::IntLit(_)
            | Expr::FloatLit(_)
            | Expr::StrLit(_)
            | Expr::CharLit(_)
            | Expr::BoolLit(_)
            | Expr::Null
            | Expr::Ident(_)
            | Expr::This
            | Expr::Super => {}
        }
        Ok(())
    }

    fn check_block_pecs(
        block: &Block,
        class_name: &str,
        member_name: Option<&str>,
        class_hierarchy: &SupertypeMap,
        method_param_sigs: &HashMap<String, Vec<MethodParamSig>>,
    ) -> Result<()> {
        let mut scopes = vec![HashMap::<String, TypeExpr>::new()];
        Self::check_block_pecs_with_scopes(
            block,
            class_name,
            member_name,
            class_hierarchy,
            method_param_sigs,
            &mut scopes,
        )
    }

    fn check_block_pecs_with_scopes(
        block: &Block,
        class_name: &str,
        member_name: Option<&str>,
        class_hierarchy: &SupertypeMap,
        method_param_sigs: &HashMap<String, Vec<MethodParamSig>>,
        scopes: &mut Vec<HashMap<String, TypeExpr>>,
    ) -> Result<()> {
        for stmt in &block.0 {
            Self::check_stmt_pecs(
                stmt,
                class_name,
                member_name,
                class_hierarchy,
                method_param_sigs,
                scopes,
            )?;
        }
        Ok(())
    }

    fn check_stmt_pecs(
        stmt: &Stmt,
        class_name: &str,
        member_name: Option<&str>,
        class_hierarchy: &SupertypeMap,
        method_param_sigs: &HashMap<String, Vec<MethodParamSig>>,
        scopes: &mut Vec<HashMap<String, TypeExpr>>,
    ) -> Result<()> {
        match stmt {
            Stmt::LocalVar { ty, name, init } => {
                if let Some(init_expr) = init {
                    Self::check_expr_pecs(
                        init_expr,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )?;
                    if let Some(src_ty) = Self::infer_expr_type(init_expr, scopes) {
                        Self::ensure_assignable(
                            ty,
                            &src_ty,
                            class_name,
                            member_name,
                            class_hierarchy,
                        )?;
                    }
                }
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(name.clone(), ty.clone());
                }
            }
            Stmt::Expr(e)
            | Stmt::Throw(e)
            | Stmt::Yield(e)
            | Stmt::Assert {
                expr: e,
                message: None,
            } => Self::check_expr_pecs(
                e,
                class_name,
                member_name,
                class_hierarchy,
                method_param_sigs,
                scopes,
            )?,
            Stmt::Assert {
                expr,
                message: Some(msg),
            } => {
                Self::check_expr_pecs(
                    expr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    msg,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Stmt::Return(Some(e)) => Self::check_expr_pecs(
                e,
                class_name,
                member_name,
                class_hierarchy,
                method_param_sigs,
                scopes,
            )?,
            Stmt::Return(None) | Stmt::Empty | Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::If { cond, then, else_ } => {
                Self::check_expr_pecs(
                    cond,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_stmt_pecs(
                        then,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
                if let Some(e) = else_ {
                    Self::with_child_scope(scopes, |scopes| {
                        Self::check_stmt_pecs(
                            e,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )
                    })?;
                }
            }
            Stmt::While { cond, body } => {
                Self::check_expr_pecs(
                    cond,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_stmt_pecs(
                        body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
            }
            Stmt::DoWhile { body, cond } => {
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_stmt_pecs(
                        body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
                Self::check_expr_pecs(
                    cond,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
            } => {
                Self::with_child_scope(scopes, |scopes| {
                    if let Some(i) = init {
                        Self::check_stmt_pecs(
                            i,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )?;
                    }
                    if let Some(c) = cond {
                        Self::check_expr_pecs(
                            c,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )?;
                    }
                    for u in update {
                        Self::check_expr_pecs(
                            u,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )?;
                    }
                    Self::check_stmt_pecs(
                        body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
            }
            Stmt::ForEach {
                ty,
                name,
                iterable,
                body,
            } => {
                Self::check_expr_pecs(
                    iterable,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::with_child_scope(scopes, |scopes| {
                    if let Some(scope) = scopes.last_mut() {
                        scope.insert(name.clone(), ty.clone());
                    }
                    Self::check_stmt_pecs(
                        body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
            }
            Stmt::Block(b) => {
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_block_pecs_with_scopes(
                        b,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
            }
            Stmt::Switch { expr, cases } => {
                Self::check_expr_pecs(
                    expr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                for case in cases {
                    if let Some(labels) = &case.labels {
                        for lbl in labels {
                            Self::check_expr_pecs(
                                lbl,
                                class_name,
                                member_name,
                                class_hierarchy,
                                method_param_sigs,
                                scopes,
                            )?;
                        }
                    }
                    Self::with_child_scope(scopes, |scopes| {
                        for s in &case.body {
                            Self::check_stmt_pecs(
                                s,
                                class_name,
                                member_name,
                                class_hierarchy,
                                method_param_sigs,
                                scopes,
                            )?;
                        }
                        Ok(())
                    })?;
                }
            }
            Stmt::TryCatch {
                try_body,
                catches,
                finally_body,
            } => {
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_block_pecs_with_scopes(
                        try_body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
                for c in catches {
                    Self::with_child_scope(scopes, |scopes| {
                        if let Some(scope) = scopes.last_mut() {
                            scope.insert(c.name.clone(), TypeExpr::simple("Exception"));
                        }
                        Self::check_block_pecs_with_scopes(
                            &c.body,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )
                    })?;
                }
                if let Some(f) = finally_body {
                    Self::with_child_scope(scopes, |scopes| {
                        Self::check_block_pecs_with_scopes(
                            f,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )
                    })?;
                }
            }
            Stmt::Labeled { stmt, .. } => {
                Self::check_stmt_pecs(
                    stmt,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Stmt::Synchronized { expr, body } => {
                Self::check_expr_pecs(
                    expr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::with_child_scope(scopes, |scopes| {
                    Self::check_block_pecs_with_scopes(
                        body,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )
                })?;
            }
        }
        Ok(())
    }

    fn check_expr_pecs(
        expr: &Expr,
        class_name: &str,
        member_name: Option<&str>,
        class_hierarchy: &SupertypeMap,
        method_param_sigs: &HashMap<String, Vec<MethodParamSig>>,
        scopes: &mut Vec<HashMap<String, TypeExpr>>,
    ) -> Result<()> {
        match expr {
            Expr::Assign { lhs, rhs } => {
                Self::check_expr_pecs(
                    lhs,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    rhs,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                if let (Some(lhs_ty), Some(rhs_ty)) = (
                    Self::infer_expr_type(lhs, scopes),
                    Self::infer_expr_type(rhs, scopes),
                ) {
                    Self::ensure_assignable(
                        &lhs_ty,
                        &rhs_ty,
                        class_name,
                        member_name,
                        class_hierarchy,
                    )?;
                }
            }
            Expr::Call { callee, args, .. } => {
                Self::check_expr_pecs(
                    callee,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                for arg in args {
                    Self::check_expr_pecs(
                        arg,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )?;
                }

                if Self::is_local_method_dispatch(callee) {
                    if let Some(method_name) = Self::callee_method_name(callee) {
                        if let Some(candidates) = method_param_sigs.get(method_name) {
                            let arg_types: Option<Vec<TypeExpr>> = args
                                .iter()
                                .map(|a| Self::infer_expr_type(a, scopes))
                                .collect();
                            if let Some(arg_types) = arg_types {
                                let mut applicable: Vec<(&MethodParamSig, i32)> = Vec::new();
                                for sig in candidates {
                                    if let Some(score) = Self::method_candidate_score(
                                        sig,
                                        &arg_types,
                                        class_hierarchy,
                                    ) {
                                        applicable.push((sig, score));
                                    }
                                }

                                if applicable.is_empty() {
                                    return Err(RavaError::Type {
                                    location: format!(
                                        "{}.{}",
                                        class_name,
                                        member_name.unwrap_or("<init>")
                                    ),
                                    message: format!(
                                        "call arguments for '{}' are not assignable to any matching parameterized signature",
                                        method_name
                                    ),
                                });
                                }

                                let max_score =
                                    applicable.iter().map(|(_, s)| *s).max().unwrap_or(0);
                                let top: Vec<&MethodParamSig> = applicable
                                    .iter()
                                    .filter_map(|(sig, score)| {
                                        if *score == max_score {
                                            Some(*sig)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();

                                let mut undominated: Vec<&MethodParamSig> = Vec::new();
                                for cand in &top {
                                    let dominated = top.iter().any(|other| {
                                        !std::ptr::eq(*other, *cand)
                                            && Self::is_more_specific(
                                                other,
                                                cand,
                                                &arg_types,
                                                class_hierarchy,
                                            )
                                    });
                                    if !dominated {
                                        undominated.push(*cand);
                                    }
                                }

                                if undominated.len() > 1 {
                                    let mut best_profile: Option<Vec<i32>> = None;
                                    let mut best_profile_count = 0usize;
                                    for sig in &undominated {
                                        if let Some(profile) = Self::method_argument_profile(
                                            sig,
                                            &arg_types,
                                            class_hierarchy,
                                        ) {
                                            match &best_profile {
                                                None => {
                                                    best_profile = Some(profile);
                                                    best_profile_count = 1;
                                                }
                                                Some(cur) if profile > *cur => {
                                                    best_profile = Some(profile);
                                                    best_profile_count = 1;
                                                }
                                                Some(cur) if profile == *cur => {
                                                    best_profile_count += 1;
                                                }
                                                Some(_) => {}
                                            }
                                        }
                                    }

                                    if best_profile_count != 1 {
                                        let mut best_shape: Option<i32> = None;
                                        let mut best_shape_count = 0usize;
                                        for sig in &undominated {
                                            let shape = Self::method_signature_shape_score(sig);
                                            match best_shape {
                                                None => {
                                                    best_shape = Some(shape);
                                                    best_shape_count = 1;
                                                }
                                                Some(cur) if shape > cur => {
                                                    best_shape = Some(shape);
                                                    best_shape_count = 1;
                                                }
                                                Some(cur) if shape == cur => {
                                                    best_shape_count += 1;
                                                }
                                                Some(_) => {}
                                            }
                                        }

                                        if best_shape_count > 1 {
                                            return Err(RavaError::Type {
                                            location: format!(
                                                "{}.{}",
                                                class_name,
                                                member_name.unwrap_or("<init>")
                                            ),
                                            message: format!(
                                                "call to '{}' is ambiguous between multiple matching signatures",
                                                method_name
                                            ),
                                        });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Expr::Field { obj, .. }
            | Expr::UnaryOp { expr: obj, .. }
            | Expr::Cast { expr: obj, .. }
            | Expr::Instanceof { expr: obj, .. }
            | Expr::InstanceofPattern { expr: obj, .. }
            | Expr::MethodRef { obj, .. } => {
                Self::check_expr_pecs(
                    obj,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::Index { arr, idx } => {
                Self::check_expr_pecs(
                    arr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    idx,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::CompoundAssign { lhs, rhs, .. } => {
                Self::check_expr_pecs(
                    lhs,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    rhs,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::Ternary { cond, then, else_ } => {
                Self::check_expr_pecs(
                    cond,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    then,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                Self::check_expr_pecs(
                    else_,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::New { args, body, .. } => {
                for arg in args {
                    Self::check_expr_pecs(
                        arg,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )?;
                }
                if let Some(members) = body {
                    for m in members {
                        if let Member::Method(mm) = m {
                            if let Some(b) = &mm.body {
                                Self::with_child_scope(scopes, |scopes| {
                                    Self::check_block_pecs_with_scopes(
                                        b,
                                        class_name,
                                        Some(&mm.name),
                                        class_hierarchy,
                                        method_param_sigs,
                                        scopes,
                                    )
                                })?;
                            }
                        }
                    }
                }
            }
            Expr::NewArray { len, .. } => {
                Self::check_expr_pecs(
                    len,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::NewMultiArray { dims, .. } => {
                for d in dims {
                    Self::check_expr_pecs(
                        d,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )?;
                }
            }
            Expr::ArrayInit { elements, .. } => {
                for e in elements {
                    Self::check_expr_pecs(
                        e,
                        class_name,
                        member_name,
                        class_hierarchy,
                        method_param_sigs,
                        scopes,
                    )?;
                }
            }
            Expr::SwitchExpr { expr, cases } => {
                Self::check_expr_pecs(
                    expr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
                for case in cases {
                    if let Some(labels) = &case.labels {
                        for lbl in labels {
                            Self::check_expr_pecs(
                                lbl,
                                class_name,
                                member_name,
                                class_hierarchy,
                                method_param_sigs,
                                scopes,
                            )?;
                        }
                    }
                    Self::with_child_scope(scopes, |scopes| {
                        for s in &case.body {
                            Self::check_stmt_pecs(
                                s,
                                class_name,
                                member_name,
                                class_hierarchy,
                                method_param_sigs,
                                scopes,
                            )?;
                        }
                        Ok(())
                    })?;
                }
            }
            Expr::Lambda { body, .. } => match &**body {
                LambdaBody::Expr(e) => Self::check_expr_pecs(
                    e,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?,
                LambdaBody::Block(b) => {
                    Self::with_child_scope(scopes, |scopes| {
                        Self::check_block_pecs_with_scopes(
                            b,
                            class_name,
                            member_name,
                            class_hierarchy,
                            method_param_sigs,
                            scopes,
                        )
                    })?;
                }
            },
            Expr::RecordPattern { expr, .. } => {
                Self::check_expr_pecs(
                    expr,
                    class_name,
                    member_name,
                    class_hierarchy,
                    method_param_sigs,
                    scopes,
                )?;
            }
            Expr::IntLit(_)
            | Expr::FloatLit(_)
            | Expr::StrLit(_)
            | Expr::CharLit(_)
            | Expr::BoolLit(_)
            | Expr::Null
            | Expr::Ident(_)
            | Expr::This
            | Expr::Super => {}
        }
        Ok(())
    }

    fn infer_expr_type(expr: &Expr, scopes: &[HashMap<String, TypeExpr>]) -> Option<TypeExpr> {
        match expr {
            Expr::Ident(name) => Self::lookup_local_type(name, scopes),
            Expr::New { ty, .. } => Some(ty.clone()),
            Expr::Cast { ty, .. } => Some(ty.clone()),
            Expr::Null => Some(TypeExpr::simple("null")),
            Expr::IntLit(_) => Some(TypeExpr::simple("int")),
            Expr::FloatLit(_) => Some(TypeExpr::simple("double")),
            Expr::BoolLit(_) => Some(TypeExpr::simple("boolean")),
            Expr::CharLit(_) => Some(TypeExpr::simple("char")),
            Expr::StrLit(_) => Some(TypeExpr::simple("String")),
            _ => None,
        }
    }

    fn lookup_local_type(name: &str, scopes: &[HashMap<String, TypeExpr>]) -> Option<TypeExpr> {
        for scope in scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn ensure_assignable(
        target: &TypeExpr,
        source: &TypeExpr,
        class_name: &str,
        member_name: Option<&str>,
        class_hierarchy: &SupertypeMap,
    ) -> Result<()> {
        if Self::is_assignable(target, source, class_hierarchy) {
            return Ok(());
        }
        Err(RavaError::Type {
            location: format!("{}.{}", class_name, member_name.unwrap_or("<init>")),
            message: format!(
                "type '{}' is not assignable to '{}' under generic variance rules",
                Self::format_type(source),
                Self::format_type(target)
            ),
        })
    }

    fn method_candidate_score(
        sig: &MethodParamSig,
        arg_types: &[TypeExpr],
        class_hierarchy: &SupertypeMap,
    ) -> Option<i32> {
        let fixed_len = if sig.variadic {
            sig.params.len().saturating_sub(1)
        } else {
            sig.params.len()
        };
        if arg_types.len() < fixed_len {
            return None;
        }
        if !sig.variadic && sig.params.len() != arg_types.len() {
            return None;
        }

        let mut score = 0i32;
        for (param, arg) in sig.params.iter().take(fixed_len).zip(arg_types.iter()) {
            score += Self::type_match_score(param, arg, class_hierarchy, &sig.type_vars)?;
        }
        if sig.variadic {
            if let Some(var_param) = sig.params.last() {
                for arg in arg_types.iter().skip(fixed_len) {
                    score +=
                        Self::type_match_score(var_param, arg, class_hierarchy, &sig.type_vars)?;
                }
            }
        }
        Some(score)
    }

    fn method_argument_profile(
        sig: &MethodParamSig,
        arg_types: &[TypeExpr],
        class_hierarchy: &SupertypeMap,
    ) -> Option<Vec<i32>> {
        let fixed_len = if sig.variadic {
            sig.params.len().saturating_sub(1)
        } else {
            sig.params.len()
        };
        if arg_types.len() < fixed_len {
            return None;
        }
        if !sig.variadic && sig.params.len() != arg_types.len() {
            return None;
        }

        let mut profile = Vec::new();
        for (param, arg) in sig.params.iter().take(fixed_len).zip(arg_types.iter()) {
            profile.push(Self::type_match_score(
                param,
                arg,
                class_hierarchy,
                &sig.type_vars,
            )?);
        }
        if sig.variadic {
            if let Some(var_param) = sig.params.last() {
                for arg in arg_types.iter().skip(fixed_len) {
                    profile.push(Self::type_match_score(
                        var_param,
                        arg,
                        class_hierarchy,
                        &sig.type_vars,
                    )?);
                }
            }
        }
        Some(profile)
    }

    fn type_match_score(
        param: &TypeExpr,
        arg: &TypeExpr,
        class_hierarchy: &SupertypeMap,
        type_vars: &HashSet<String>,
    ) -> Option<i32> {
        if Self::contains_type_var(param, type_vars) {
            // Inferred method type variables are solved by full type inference later.
            // Treat them as placeholders when the instantiated argument is assignable
            // to the generic parameter shape.
            if Self::typevar_name_if_plain(param, type_vars).is_some() {
                return Some(2);
            }
            if param.array_dims == arg.array_dims && param.name == arg.name {
                return Some(2);
            }
            if Self::is_assignable(param, arg, class_hierarchy) {
                return Some(2);
            }
            return None;
        }
        if Self::same_type(param, arg) {
            return Some(5);
        }

        if let Some(score) = Self::primitive_conversion_score(param, arg, class_hierarchy) {
            return Some(score);
        }

        if Self::is_assignable(param, arg, class_hierarchy) {
            return Some(1);
        }
        None
    }

    fn primitive_conversion_score(
        target: &TypeExpr,
        source: &TypeExpr,
        class_hierarchy: &SupertypeMap,
    ) -> Option<i32> {
        if target.array_dims != 0 || source.array_dims != 0 {
            return None;
        }

        let t = target.name.as_str();
        let s = source.name.as_str();

        // Widening primitive conversion beats boxing.
        if Self::is_primitive_type(t)
            && Self::is_primitive_type(s)
            && Self::can_widen_primitive(s, t)
        {
            return Some(4);
        }

        // Boxing conversion.
        if Self::is_primitive_type(s) {
            if let Some(wrapper) = Self::wrapper_for_primitive(s) {
                if wrapper == t {
                    return Some(3);
                }
                if Self::satisfies_bound(wrapper, t, class_hierarchy) {
                    return Some(2);
                }
            }
        }

        // Unboxing conversion.
        if Self::is_primitive_type(t) {
            if let Some(unboxed) = Self::primitive_for_wrapper(s) {
                if unboxed == t {
                    return Some(3);
                }
                if Self::can_widen_primitive(unboxed, t) {
                    return Some(2);
                }
            }
        }

        None
    }

    fn is_more_specific(
        a: &MethodParamSig,
        b: &MethodParamSig,
        arg_types: &[TypeExpr],
        class_hierarchy: &SupertypeMap,
    ) -> bool {
        if !a.variadic && b.variadic {
            return true;
        }

        let mut a_better = false;
        let mut b_better = false;

        for idx in 0..arg_types.len() {
            let Some(ap) = Self::param_at(a, idx) else {
                continue;
            };
            let Some(bp) = Self::param_at(b, idx) else {
                continue;
            };

            let a_has_var = Self::contains_type_var(ap, &a.type_vars);
            let b_has_var = Self::contains_type_var(bp, &b.type_vars);
            if a_has_var != b_has_var {
                let (a_more, b_more) =
                    Self::compare_bound_specificity_in_type(ap, bp, a, b, class_hierarchy);
                if a_more && !b_more {
                    a_better = true;
                    continue;
                }
                if b_more && !a_more {
                    b_better = true;
                    continue;
                }

                if !a_has_var {
                    a_better = true;
                } else {
                    b_better = true;
                }
                continue;
            }

            if a_has_var && b_has_var {
                let a_var = Self::typevar_name_if_plain(ap, &a.type_vars);
                let b_var = Self::typevar_name_if_plain(bp, &b.type_vars);
                if let (Some(a_var), Some(b_var)) = (a_var, b_var) {
                    let a_bounds = a.type_var_bounds.get(&a_var).cloned().unwrap_or_default();
                    let b_bounds = b.type_var_bounds.get(&b_var).cloned().unwrap_or_default();
                    let a_more = Self::bounds_more_specific(&a_bounds, &b_bounds, class_hierarchy);
                    let b_more = Self::bounds_more_specific(&b_bounds, &a_bounds, class_hierarchy);
                    if a_more && !b_more {
                        a_better = true;
                        continue;
                    }
                    if b_more && !a_more {
                        b_better = true;
                        continue;
                    }
                }

                let (a_more, b_more) =
                    Self::compare_bound_specificity_in_type(ap, bp, a, b, class_hierarchy);
                if a_more && !b_more {
                    a_better = true;
                    continue;
                }
                if b_more && !a_more {
                    b_better = true;
                    continue;
                }
            }

            if Self::same_type(ap, bp) {
                continue;
            }

            let a_to_b = Self::is_assignable(bp, ap, class_hierarchy);
            let b_to_a = Self::is_assignable(ap, bp, class_hierarchy);
            if a_to_b && !b_to_a {
                a_better = true;
            } else if b_to_a && !a_to_b {
                b_better = true;
            }
        }

        a_better && !b_better
    }

    fn contains_type_var(ty: &TypeExpr, type_vars: &HashSet<String>) -> bool {
        if type_vars.contains(&Self::simple_name(&ty.name)) {
            return true;
        }
        let Some(args) = &ty.generic_args else {
            return false;
        };
        args.iter()
            .any(|a| Self::type_arg_contains_type_var(a, type_vars))
    }

    fn type_arg_contains_type_var(arg: &TypeArg, type_vars: &HashSet<String>) -> bool {
        match arg {
            TypeArg::Type(t) | TypeArg::WildcardExtends(t) | TypeArg::WildcardSuper(t) => {
                Self::contains_type_var(t, type_vars)
            }
            TypeArg::Wildcard => false,
        }
    }

    fn typevar_name_if_plain(ty: &TypeExpr, type_vars: &HashSet<String>) -> Option<String> {
        if ty.array_dims != 0 || ty.generic_args.is_some() {
            return None;
        }
        let name = Self::simple_name(&ty.name);
        if type_vars.contains(&name) {
            Some(name)
        } else {
            None
        }
    }

    fn bounds_more_specific(
        a_bounds: &[String],
        b_bounds: &[String],
        class_hierarchy: &SupertypeMap,
    ) -> bool {
        // Unbounded type var ~= `extends Object` and is least specific.
        if b_bounds.is_empty() {
            return !a_bounds.is_empty();
        }
        if a_bounds.is_empty() {
            return false;
        }

        // A is more specific than B if each B bound is covered by some A bound,
        // and at least one coverage is strictly narrower.
        let mut strict = false;
        for b in b_bounds {
            let mut covered = false;
            for a in a_bounds {
                if Self::satisfies_bound(a, b, class_hierarchy) {
                    covered = true;
                    if !Self::satisfies_bound(b, a, class_hierarchy) {
                        strict = true;
                    }
                }
            }
            if !covered {
                return false;
            }
        }
        strict
    }

    fn compare_bound_specificity_in_type(
        a_ty: &TypeExpr,
        b_ty: &TypeExpr,
        a_sig: &MethodParamSig,
        b_sig: &MethodParamSig,
        class_hierarchy: &SupertypeMap,
    ) -> (bool, bool) {
        let a_var = Self::typevar_name_if_plain(a_ty, &a_sig.type_vars);
        let b_var = Self::typevar_name_if_plain(b_ty, &b_sig.type_vars);

        if let (Some(a_var), None) = (&a_var, &b_var) {
            if let Some(b_plain) = Self::plain_concrete_name(b_ty) {
                let a_bounds = a_sig
                    .type_var_bounds
                    .get(a_var)
                    .cloned()
                    .unwrap_or_default();
                let b_bounds = vec![b_plain];
                let a_more = Self::bounds_more_specific(&a_bounds, &b_bounds, class_hierarchy);
                let b_more = Self::bounds_more_specific(&b_bounds, &a_bounds, class_hierarchy);
                return (a_more, b_more);
            }
        }
        if let (None, Some(b_var)) = (&a_var, &b_var) {
            if let Some(a_plain) = Self::plain_concrete_name(a_ty) {
                let a_bounds = vec![a_plain];
                let b_bounds = b_sig
                    .type_var_bounds
                    .get(b_var)
                    .cloned()
                    .unwrap_or_default();
                let a_more = Self::bounds_more_specific(&a_bounds, &b_bounds, class_hierarchy);
                let b_more = Self::bounds_more_specific(&b_bounds, &a_bounds, class_hierarchy);
                return (a_more, b_more);
            }
        }

        if let (Some(a_var), Some(b_var)) = (a_var, b_var) {
            let a_bounds = a_sig
                .type_var_bounds
                .get(&a_var)
                .cloned()
                .unwrap_or_default();
            let b_bounds = b_sig
                .type_var_bounds
                .get(&b_var)
                .cloned()
                .unwrap_or_default();
            let a_more = Self::bounds_more_specific(&a_bounds, &b_bounds, class_hierarchy);
            let b_more = Self::bounds_more_specific(&b_bounds, &a_bounds, class_hierarchy);
            return (a_more, b_more);
        }

        if a_ty.name != b_ty.name || a_ty.array_dims != b_ty.array_dims {
            return (false, false);
        }
        let (Some(a_args), Some(b_args)) = (&a_ty.generic_args, &b_ty.generic_args) else {
            return (false, false);
        };
        if a_args.len() != b_args.len() {
            return (false, false);
        }

        let mut a_better = false;
        let mut b_better = false;
        for (aa, bb) in a_args.iter().zip(b_args.iter()) {
            let (am, bm) =
                Self::compare_bound_specificity_in_arg(aa, bb, a_sig, b_sig, class_hierarchy);
            a_better |= am;
            b_better |= bm;
        }
        (a_better, b_better)
    }

    fn compare_bound_specificity_in_arg(
        a_arg: &TypeArg,
        b_arg: &TypeArg,
        a_sig: &MethodParamSig,
        b_sig: &MethodParamSig,
        class_hierarchy: &SupertypeMap,
    ) -> (bool, bool) {
        if Self::is_effectively_unbounded_wildcard(a_arg)
            && Self::is_effectively_unbounded_wildcard(b_arg)
        {
            return (false, false);
        }

        match (a_arg, b_arg) {
            (TypeArg::Type(at), TypeArg::Type(bt))
            | (TypeArg::WildcardExtends(at), TypeArg::WildcardExtends(bt)) => {
                Self::compare_bound_specificity_in_type(at, bt, a_sig, b_sig, class_hierarchy)
            }
            (TypeArg::WildcardSuper(at), TypeArg::WildcardSuper(bt)) => {
                // Lower-bounded wildcard is contravariant: tighter lower bound is
                // the less specific type (`? super Number` is more specific than
                // `? super Integer`).
                let (a_more, b_more) =
                    Self::compare_bound_specificity_in_type(at, bt, a_sig, b_sig, class_hierarchy);
                (b_more, a_more)
            }
            (TypeArg::Type(at), TypeArg::WildcardExtends(bt)) => {
                if Self::same_type(at, bt) {
                    return (true, false);
                }
                Self::compare_bound_specificity_in_type(at, bt, a_sig, b_sig, class_hierarchy)
            }
            (TypeArg::WildcardExtends(at), TypeArg::Type(bt)) => {
                if Self::same_type(at, bt) {
                    return (false, true);
                }
                Self::compare_bound_specificity_in_type(at, bt, a_sig, b_sig, class_hierarchy)
            }
            (TypeArg::Type(at), TypeArg::WildcardSuper(bt)) => {
                // Concrete type is more specific than a compatible lower-bounded wildcard.
                let a_name = Self::simple_name(&at.name);
                let b_name = Self::simple_name(&bt.name);
                if Self::satisfies_bound(&b_name, &a_name, class_hierarchy) {
                    (true, false)
                } else {
                    (false, false)
                }
            }
            (TypeArg::WildcardSuper(at), TypeArg::Type(bt)) => {
                let a_name = Self::simple_name(&at.name);
                let b_name = Self::simple_name(&bt.name);
                if Self::satisfies_bound(&a_name, &b_name, class_hierarchy) {
                    (false, true)
                } else {
                    (false, false)
                }
            }
            (TypeArg::Type(_), TypeArg::Wildcard) => (true, false),
            (TypeArg::Wildcard, TypeArg::Type(_)) => (false, true),
            _ => (false, false),
        }
    }

    fn plain_concrete_name(ty: &TypeExpr) -> Option<String> {
        if ty.array_dims != 0 || ty.generic_args.is_some() {
            return None;
        }
        Some(Self::simple_name(&ty.name))
    }

    fn is_effectively_unbounded_wildcard(arg: &TypeArg) -> bool {
        match arg {
            TypeArg::Wildcard => true,
            TypeArg::WildcardExtends(t) => {
                t.array_dims == 0
                    && t.generic_args.is_none()
                    && matches!(t.name.as_str(), "Object" | "java.lang.Object")
            }
            _ => false,
        }
    }

    fn param_at<'a>(sig: &'a MethodParamSig, idx: usize) -> Option<&'a TypeExpr> {
        if sig.variadic {
            let fixed_len = sig.params.len().saturating_sub(1);
            if idx < fixed_len {
                sig.params.get(idx)
            } else {
                sig.params.last()
            }
        } else {
            sig.params.get(idx)
        }
    }

    fn method_signature_shape_score(sig: &MethodParamSig) -> i32 {
        let mut score = if sig.variadic { 0 } else { 2 };
        for p in &sig.params {
            score += Self::type_shape_score(p);
        }
        score
    }

    fn type_shape_score(ty: &TypeExpr) -> i32 {
        let mut score = 2;
        if let Some(args) = &ty.generic_args {
            for a in args {
                score += Self::type_arg_shape_score(a);
            }
        }
        score
    }

    fn type_arg_shape_score(arg: &TypeArg) -> i32 {
        match arg {
            TypeArg::Type(t) => 2 + Self::type_shape_score(t),
            TypeArg::Wildcard => 0,
            TypeArg::WildcardExtends(t)
                if t.array_dims == 0
                    && t.generic_args.is_none()
                    && matches!(t.name.as_str(), "Object" | "java.lang.Object") =>
            {
                0
            }
            TypeArg::WildcardExtends(t) | TypeArg::WildcardSuper(t) => {
                1 + Self::type_shape_score(t)
            }
        }
    }

    fn is_assignable(target: &TypeExpr, source: &TypeExpr, class_hierarchy: &SupertypeMap) -> bool {
        // `var` infers from initializer, so assignment is always valid here.
        if target.name == "var" {
            return true;
        }

        // Java `null` can be assigned to any reference type.
        if source.name == "null" {
            return !Self::is_primitive_type(&target.name);
        }

        if target.array_dims == 0 && source.array_dims == 0 {
            let t = target.name.as_str();
            let s = source.name.as_str();

            if Self::is_primitive_type(t) && Self::is_primitive_type(s) {
                return t == s || Self::can_widen_primitive(s, t);
            }

            if Self::is_primitive_type(s) {
                if let Some(wrapper) = Self::wrapper_for_primitive(s) {
                    if t == wrapper {
                        return true;
                    }
                    if Self::satisfies_bound(wrapper, t, class_hierarchy) {
                        return true;
                    }
                }
            }

            if Self::is_primitive_type(t) {
                if let Some(unboxed) = Self::primitive_for_wrapper(s) {
                    return t == unboxed || Self::can_widen_primitive(unboxed, t);
                }
            }
        }

        if target.array_dims != source.array_dims {
            return false;
        }

        if target.name != source.name {
            let src = Self::normalize_type_name(&source.name);
            let tgt = Self::normalize_type_name(&target.name);
            return Self::satisfies_bound(&src, &tgt, class_hierarchy);
        }

        // Diamond operator: `new Type<>()` defers concrete type args to context.
        // Treat it as assignable when raw type matches.
        if source.generic_args_raw.as_deref() == Some("") {
            return true;
        }

        match (&target.generic_args, &source.generic_args) {
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
            (Some(targs), Some(sargs)) => {
                if targs.len() != sargs.len() {
                    return false;
                }
                targs
                    .iter()
                    .zip(sargs.iter())
                    .all(|(targ, sarg)| Self::is_type_arg_assignable(targ, sarg, class_hierarchy))
            }
        }
    }

    fn is_type_arg_assignable(
        target: &TypeArg,
        source: &TypeArg,
        class_hierarchy: &SupertypeMap,
    ) -> bool {
        match target {
            TypeArg::Wildcard => true,
            TypeArg::Type(tt) => match source {
                TypeArg::Type(st) => Self::same_type(tt, st),
                _ => false,
            },
            TypeArg::WildcardExtends(tb) => match source {
                TypeArg::Wildcard
                    if tb.array_dims == 0
                        && tb.generic_args.is_none()
                        && matches!(tb.name.as_str(), "Object" | "java.lang.Object") =>
                {
                    true
                }
                TypeArg::Type(st) => {
                    let s = Self::normalize_type_name(&st.name);
                    let t = Self::normalize_type_name(&tb.name);
                    Self::satisfies_bound(&s, &t, class_hierarchy)
                }
                TypeArg::WildcardExtends(sb) => {
                    let s = Self::normalize_type_name(&sb.name);
                    let t = Self::normalize_type_name(&tb.name);
                    Self::satisfies_bound(&s, &t, class_hierarchy)
                }
                _ => false,
            },
            TypeArg::WildcardSuper(tb) => match source {
                TypeArg::Type(st) => {
                    let s = Self::normalize_type_name(&st.name);
                    let t = Self::normalize_type_name(&tb.name);
                    Self::satisfies_bound(&t, &s, class_hierarchy)
                }
                TypeArg::WildcardSuper(sb) => {
                    let s = Self::normalize_type_name(&sb.name);
                    let t = Self::normalize_type_name(&tb.name);
                    Self::satisfies_bound(&t, &s, class_hierarchy)
                }
                _ => false,
            },
        }
    }

    fn same_type(a: &TypeExpr, b: &TypeExpr) -> bool {
        if a.name != b.name || a.array_dims != b.array_dims {
            return false;
        }
        match (&a.generic_args, &b.generic_args) {
            (None, None) => true,
            (Some(aa), Some(bb)) if aa.len() == bb.len() => aa
                .iter()
                .zip(bb.iter())
                .all(|(x, y)| Self::same_type_arg(x, y)),
            _ => false,
        }
    }

    fn same_type_arg(a: &TypeArg, b: &TypeArg) -> bool {
        if Self::is_effectively_unbounded_wildcard(a) && Self::is_effectively_unbounded_wildcard(b)
        {
            return true;
        }
        match (a, b) {
            (TypeArg::Wildcard, TypeArg::Wildcard) => true,
            (TypeArg::Type(at), TypeArg::Type(bt)) => Self::same_type(at, bt),
            (TypeArg::WildcardExtends(at), TypeArg::WildcardExtends(bt)) => Self::same_type(at, bt),
            (TypeArg::WildcardSuper(at), TypeArg::WildcardSuper(bt)) => Self::same_type(at, bt),
            _ => false,
        }
    }

    fn format_type(ty: &TypeExpr) -> String {
        let mut s = ty.name.clone();
        if let Some(raw) = &ty.generic_args_raw {
            s.push('<');
            s.push_str(raw);
            s.push('>');
        }
        for _ in 0..ty.array_dims {
            s.push_str("[]");
        }
        s
    }

    fn with_child_scope<T>(
        scopes: &mut Vec<HashMap<String, TypeExpr>>,
        f: impl FnOnce(&mut Vec<HashMap<String, TypeExpr>>) -> Result<T>,
    ) -> Result<T> {
        scopes.push(HashMap::new());
        let res = f(scopes);
        let _ = scopes.pop();
        res
    }

    fn callee_method_name(callee: &Expr) -> Option<&str> {
        match callee {
            Expr::Ident(name) => Some(name.as_str()),
            Expr::Field { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    fn is_local_method_dispatch(callee: &Expr) -> bool {
        match callee {
            Expr::Ident(_) => true,
            Expr::Field { obj, .. } => matches!(&**obj, Expr::This | Expr::Super),
            _ => false,
        }
    }

    fn parse_type_param_decls(raw: Option<&str>) -> Vec<TypeParamDecl> {
        let Some(raw) = raw else {
            return vec![];
        };
        Self::split_top_level_commas(raw)
            .into_iter()
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    return None;
                }
                let (name, bounds) = if let Some((left, right)) = p.split_once("extends") {
                    let name = left.trim().to_string();
                    let bounds = right
                        .split('&')
                        .map(Self::normalize_type_name)
                        .filter(|b| !b.is_empty())
                        .collect::<Vec<_>>();
                    (name, bounds)
                } else {
                    (p.trim().to_string(), vec![])
                };
                Some(TypeParamDecl { name, bounds })
            })
            .collect()
    }

    fn parse_explicit_type_args(raw: &str) -> Vec<String> {
        Self::split_top_level_commas(raw)
            .into_iter()
            .map(|a| Self::normalize_type_name(&a))
            .collect()
    }

    fn normalize_type_name(raw: &str) -> String {
        let mut out = String::new();
        let mut depth = 0i32;
        for ch in raw.trim().chars() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                '[' | ']' if depth == 0 => {}
                _ if depth == 0 => out.push(ch),
                _ => {}
            }
        }
        out.trim().to_string()
    }

    fn build_class_hierarchy(file: &SourceFile) -> SupertypeMap {
        let mut map: SupertypeMap = HashMap::new();
        for c in &file.classes {
            let mut supers = Vec::new();
            if let Some(base) = &c.superclass {
                supers.push(Self::simple_name(base));
            }
            for iface in &c.interfaces {
                supers.push(Self::simple_name(iface));
            }
            map.insert(c.name.clone(), supers);
        }
        map
    }

    fn satisfies_bound(actual: &str, bound: &str, class_hierarchy: &SupertypeMap) -> bool {
        let actual = Self::simple_name(actual);
        let bound = Self::simple_name(bound);
        if bound.is_empty() || actual.is_empty() {
            return true;
        }
        if actual == bound || bound == "Object" {
            return true;
        }

        // Walk user-defined class/interface hierarchy (BFS through all direct supertypes).
        let mut queue = vec![actual.clone()];
        let mut seen = HashSet::new();
        while let Some(name) = queue.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if name == bound {
                return true;
            }
            if let Some(supers) = class_hierarchy.get(&name) {
                queue.extend(supers.iter().cloned());
            }
        }

        // Walk small builtin lattice used by tests.
        let mut cur = Some(actual.as_str());
        while let Some(name) = cur {
            if name == bound {
                return true;
            }
            cur = Self::builtin_super(name);
        }

        // If either side is user-defined and no path was found, it's a mismatch.
        if class_hierarchy.contains_key(&actual) || class_hierarchy.contains_key(&bound) {
            return false;
        }

        // For external unresolved types, keep permissive behavior to avoid
        // false positives until full type-resolution is implemented.
        !Self::is_known_simple_type(&actual) || !Self::is_known_simple_type(&bound)
    }

    fn simple_name(name: &str) -> String {
        name.rsplit('.').next().unwrap_or(name).trim().to_string()
    }

    fn builtin_super(name: &str) -> Option<&'static str> {
        match name {
            "Byte" | "Short" | "Integer" | "Long" | "Float" | "Double" => Some("Number"),
            "Number" | "String" | "Boolean" | "Character" => Some("Object"),
            _ => None,
        }
    }

    fn is_known_simple_type(name: &str) -> bool {
        matches!(
            name,
            "Object"
                | "Number"
                | "Byte"
                | "Short"
                | "Integer"
                | "Long"
                | "Float"
                | "Double"
                | "String"
                | "Boolean"
                | "Character"
        )
    }

    fn is_primitive_type(name: &str) -> bool {
        matches!(
            name,
            "byte" | "short" | "int" | "long" | "float" | "double" | "char" | "boolean" | "void"
        )
    }

    fn wrapper_for_primitive(name: &str) -> Option<&'static str> {
        match name {
            "byte" => Some("Byte"),
            "short" => Some("Short"),
            "int" => Some("Integer"),
            "long" => Some("Long"),
            "float" => Some("Float"),
            "double" => Some("Double"),
            "char" => Some("Character"),
            "boolean" => Some("Boolean"),
            _ => None,
        }
    }

    fn primitive_for_wrapper(name: &str) -> Option<&'static str> {
        match name {
            "Byte" => Some("byte"),
            "Short" => Some("short"),
            "Integer" => Some("int"),
            "Long" => Some("long"),
            "Float" => Some("float"),
            "Double" => Some("double"),
            "Character" => Some("char"),
            "Boolean" => Some("boolean"),
            _ => None,
        }
    }

    fn can_widen_primitive(src: &str, dst: &str) -> bool {
        if src == dst {
            return true;
        }
        matches!(
            (src, dst),
            ("byte", "short")
                | ("byte", "int")
                | ("byte", "long")
                | ("byte", "float")
                | ("byte", "double")
                | ("short", "int")
                | ("short", "long")
                | ("short", "float")
                | ("short", "double")
                | ("char", "int")
                | ("char", "long")
                | ("char", "float")
                | ("char", "double")
                | ("int", "long")
                | ("int", "float")
                | ("int", "double")
                | ("long", "float")
                | ("long", "double")
                | ("float", "double")
        )
    }

    fn count_type_args(raw: &str) -> usize {
        if raw.trim().is_empty() {
            return 0;
        }
        Self::split_top_level_commas(raw).len()
    }

    fn split_top_level_commas(raw: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut buf = String::new();
        let mut depth = 0i32;
        for ch in raw.chars() {
            match ch {
                '<' => {
                    depth += 1;
                    buf.push(ch);
                }
                '>' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    buf.push(ch);
                }
                ',' if depth == 0 => {
                    parts.push(buf.trim().to_string());
                    buf.clear();
                }
                _ => buf.push(ch),
            }
        }
        if !buf.trim().is_empty() || raw.is_empty() {
            parts.push(buf.trim().to_string());
        }
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::GenericChecker;
    use crate::{lexer::Lexer, parser::Parser};

    fn parse(src: &str) -> crate::ast::SourceFile {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_file().unwrap()
    }

    #[test]
    fn explicit_type_arg_arity_matches() {
        let src = r#"
            class T {
                <A, B> void id() {}
                void run() { this.<String, Integer>id(); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn explicit_type_arg_arity_mismatch_fails() {
        let src = r#"
            class T {
                <A, B> void id() {}
                void run() { this.<String>id(); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn wildcard_in_type_param_bound_fails() {
        let src = r#"
            class Bad<T extends ? extends Number> {}
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn explicit_type_arg_bound_matches() {
        let src = r#"
            class T {
                <A extends Number> void id() {}
                void run() { this.<Integer>id(); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn explicit_type_arg_bound_mismatch_fails() {
        let src = r#"
            class T {
                <A extends Number> void id() {}
                void run() { this.<String>id(); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn pecs_extends_allows_producer_assignment() {
        let src = r#"
            class T {
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    java.util.List<? extends Number> nums = ints;
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn pecs_invariant_rejects_concrete_mismatch() {
        let src = r#"
            class T {
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    java.util.List<Number> nums = ints;
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn pecs_super_allows_consumer_assignment() {
        let src = r#"
            class T {
                void run() {
                    java.util.List<Number> nums = new java.util.ArrayList<>();
                    java.util.List<? super Integer> sink = nums;
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn pecs_rejects_wildcard_to_concrete_assignment() {
        let src = r#"
            class T {
                void run() {
                    java.util.List<? extends Number> src = new java.util.ArrayList<Integer>();
                    java.util.List<Number> dst = src;
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn pecs_method_param_extends_allows_concrete_arg() {
        let src = r#"
            class T {
                void read(java.util.List<? extends Number> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    read(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn pecs_method_param_invariant_rejects_mismatch() {
        let src = r#"
            class T {
                void read(java.util.List<Number> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    read(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn pecs_method_param_super_allows_wider_arg() {
        let src = r#"
            class T {
                void write(java.util.List<? super Integer> xs) {}
                void run() {
                    java.util.List<Number> nums = new java.util.ArrayList<>();
                    write(nums);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn multi_bound_interface_satisfied() {
        let src = r#"
            interface A {}
            interface B {}
            class C implements A, B {}
            class T {
                <X extends A & B> void use(X x) {}
                void run() { this.<C>use(new C()); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn multi_bound_interface_missing_one_fails() {
        let src = r#"
            interface A {}
            interface B {}
            class C implements A {}
            class T {
                <X extends A & B> void use(X x) {}
                void run() { this.<C>use(new C()); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_more_specific_match() {
        let src = r#"
            class T {
                void read(java.util.List<? extends Number> xs) {}
                void read(java.util.List<Integer> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    read(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_ambiguous_generic_match_fails() {
        let src = r#"
            class T {
                void read(java.util.List<? extends Number> xs) {}
                void read(java.util.List<? super Integer> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    read(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_non_variadic_when_both_match() {
        let src = r#"
            class T {
                void f(Integer x) {}
                void f(Integer... xs) {}
                void run() { f(1); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_subtype_parameter() {
        let src = r#"
            class A {}
            class B extends A {}
            class T {
                void g(A x) {}
                void g(B x) {}
                void run() { g(new B()); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn transitive_interface_bound_is_satisfied() {
        let src = r#"
            interface A {}
            interface B extends A {}
            class C implements B {}
            class T {
                <X extends A> void use(X x) {}
                void run() { this.<C>use(new C()); }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_concrete_over_typevar_param() {
        let src = r#"
            class T {
                <X> void f(java.util.List<X> xs) {}
                void f(java.util.List<Integer> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_concrete_over_nested_typevar_param() {
        let src = r#"
            class T {
                <X> void f(java.util.Map<String, X> m) {}
                void f(java.util.Map<String, Integer> m) {}
                void run() {
                    java.util.Map<String, Integer> m = new java.util.HashMap<>();
                    f(m);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_narrower_typevar_bound() {
        let src = r#"
            class T {
                <X extends Number> void f(X x) {}
                <Y extends Integer> void f(Y y) {}
                void run() {
                    Integer i = 1;
                    f(i);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_narrower_nested_typevar_bound() {
        let src = r#"
            class T {
                <X extends Number> void f(java.util.List<X> xs) {}
                <Y extends Integer> void f(java.util.List<Y> ys) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_incomparable_nested_bounds_is_ambiguous() {
        let src = r#"
            class T {
                <X extends Number> void f(java.util.List<X> xs) {}
                <Y extends Comparable<Y>> void f(java.util.List<Y> ys) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_wildcard_with_narrower_typevar_bound() {
        let src = r#"
            class T {
                <X extends Integer> void f(java.util.List<? extends X> xs) {}
                void f(java.util.List<? extends Number> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_wildcard_incomparable_bounds_is_ambiguous() {
        let src = r#"
            class T {
                <X extends Number> void f(java.util.List<? extends X> xs) {}
                <Y extends Comparable<Y>> void f(java.util.List<? extends Y> ys) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_wildcard_super_with_tighter_lower_bound() {
        let src = r#"
            class T {
                void f(java.util.List<? super Number> xs) {}
                void f(java.util.List<? super Integer> xs) {}
                void run() {
                    java.util.List<Number> nums = new java.util.ArrayList<>();
                    f(nums);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_wildcard_super_incomparable_bounds_is_ambiguous() {
        let src = r#"
            class T {
                <X extends Number> void f(java.util.List<? super X> xs) {}
                <Y extends Comparable<Y>> void f(java.util.List<? super Y> ys) {}
                void run() {
                    java.util.List<Object> any = new java.util.ArrayList<>();
                    f(any);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_concrete_over_wildcard_extends_same_bound() {
        let src = r#"
            class T {
                void f(java.util.List<Integer> xs) {}
                void f(java.util.List<? extends Integer> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_concrete_over_wildcard_super_compatible() {
        let src = r#"
            class T {
                void f(java.util.List<Number> xs) {}
                void f(java.util.List<? super Integer> xs) {}
                void run() {
                    java.util.List<Number> nums = new java.util.ArrayList<>();
                    f(nums);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn wildcard_extends_object_accepts_unbounded_wildcard_source() {
        let src = r#"
            class T {
                void run() {
                    java.util.List<?> a = new java.util.ArrayList<Integer>();
                    java.util.List<? extends Object> b = a;
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_unbounded_and_extends_object_is_ambiguous() {
        let src = r#"
            class T {
                void f(java.util.List<?> xs) {}
                void f(java.util.List<? extends Object> xs) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_uses_parameter_profile_tiebreak() {
        let src = r#"
            class T {
                void f(java.util.List<Integer> a, java.util.List<? extends Number> b) {}
                void f(java.util.List<? extends Number> a, java.util.List<Integer> b) {}
                void run() {
                    java.util.List<Integer> ints = new java.util.ArrayList<>();
                    f(ints, ints);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_null_prefers_more_specific_reference() {
        let src = r#"
            class A {}
            class B extends A {}
            class T {
                void f(A a) {}
                void f(B b) {}
                void run() {
                    f(null);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_null_unrelated_references_is_ambiguous() {
        let src = r#"
            class A {}
            class C {}
            class T {
                void f(A a) {}
                void f(C c) {}
                void run() {
                    f(null);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_err());
    }

    #[test]
    fn overload_prefers_primitive_exact_over_boxing() {
        let src = r#"
            class T {
                void f(int x) {}
                void f(Integer x) {}
                void run() {
                    f(1);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_primitive_widening_over_boxing() {
        let src = r#"
            class T {
                void f(long x) {}
                void f(Integer x) {}
                void run() {
                    f(1);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }

    #[test]
    fn overload_prefers_boxing_to_wrapper_over_supertype() {
        let src = r#"
            class T {
                void f(Integer x) {}
                void f(Number x) {}
                void run() {
                    f(1);
                }
            }
        "#;
        let file = parse(src);
        assert!(GenericChecker::check_file(&file).is_ok());
    }
}
