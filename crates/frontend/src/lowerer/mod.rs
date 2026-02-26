//! AST -> RIR lowerer.
//!
//! Walks the AST and emits RIR instructions into basic blocks.

use rava_common::error::Result;
use rava_rir::{
    BasicBlock, BinOp as RirBinOp, BlockId, ClassId, FieldId, FuncFlags, FuncId, MethodId,
    RirFunction, RirInstr, RirModule, RirType, UnaryOp as RirUnaryOp, Value,
};
use crate::ast::*;

mod helpers;
mod stmt;
mod expr;
mod tests;

pub use helpers::encode_builtin;
use helpers::{is_static_path, lower_type, lower_type_name, lower_binop, expr_to_str};

pub struct Lowerer {
    module:    RirModule,
    func_id:   u32,
    block_id:  u32,
    value_id:  u32,
    varargs_methods: std::collections::HashMap<String, usize>,
    lambda_counter: u32,
    pending_lambdas: Vec<PendingLambda>,
    anon_counter: u32,
    pending_anon_classes: Vec<PendingAnonClass>,
}

pub(super) struct PendingLambda {
    name: String,
    params: Vec<String>,
    body: LambdaBody,
    captures: Vec<String>,
}

pub(super) struct PendingAnonClass {
    name: String,
    parent: String,
    members: Vec<Member>,
}

impl Lowerer {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module:   RirModule::new(module_name),
            func_id:  0,
            block_id: 0,
            value_id: 0,
            varargs_methods: std::collections::HashMap::new(),
            lambda_counter: 0,
            pending_lambdas: Vec::new(),
            anon_counter: 0,
            pending_anon_classes: Vec::new(),
        }
    }

    pub fn lower_file(mut self, file: &SourceFile) -> Result<RirModule> {
        for class in &file.classes {
            self.lower_class(class)?;
        }
        Ok(self.module)
    }

    fn lower_class(&mut self, class: &ClassDecl) -> Result<()> {
        if let Some(ref parent) = class.superclass {
            self.module.class_hierarchy.insert(class.name.clone(), parent.clone());
        }
        for iface in &class.interfaces {
            let key = format!("{}:{}", class.name, iface);
            self.module.class_hierarchy.insert(key, iface.clone());
        }

        self.module.class_names.insert(encode_builtin(&class.name), class.name.clone());

        let field_inits: Vec<(String, Expr)> = class.members.iter()
            .filter_map(|m| {
                if let Member::Field(f) = m {
                    if !f.modifiers.contains(&Modifier::Static) {
                        if let Some(init) = &f.init {
                            return Some((f.name.clone(), init.clone()));
                        }
                    }
                }
                None
            })
            .collect();

        for member in &class.members {
            if let Member::Field(f) = member {
                let hash = encode_builtin(&f.name);
                self.module.field_names.insert(hash, f.name.clone());
                let rir_type = type_expr_to_rir_type(&f.ty);
                self.module.field_types.insert(hash, rir_type);
            }
            if let Member::Method(m) = member {
                let key = format!("__method__{}", m.name);
                self.module.method_names.insert(encode_builtin(&key), m.name.clone());
            }
        }

        for member in &class.members {
            if let Member::Method(m) = member {
                if let Some(last_param) = m.params.last() {
                    if last_param.variadic {
                        let fixed = m.params.len() - 1;
                        let full_key = format!("{}.{}", class.name, m.name);
                        self.varargs_methods.insert(full_key, fixed);
                        self.varargs_methods.insert(m.name.clone(), fixed);
                    }
                }
            }
        }

        let has_constructor = class.members.iter().any(|m| matches!(m, Member::Constructor(_)));
        let mut static_field_inits: Vec<&FieldDecl> = Vec::new();
        let mut enum_constants: Vec<&EnumConstant> = Vec::new();

        let static_field_names: std::collections::HashSet<String> = class.members.iter()
            .filter_map(|m| {
                if let Member::Field(f) = m {
                    if f.modifiers.contains(&Modifier::Static) {
                        return Some(f.name.clone());
                    }
                }
                None
            })
            .collect();

        let enum_constant_names: std::collections::HashSet<String> = class.members.iter()
            .filter_map(|m| if let Member::EnumConstant(ec) = m { Some(ec.name.clone()) } else { None })
            .collect();

        for member in &class.members {
            match member {
                Member::Method(m)      => self.lower_method(&class.name, m, &static_field_names, &enum_constant_names)?,
                Member::Constructor(c) => self.lower_constructor(&class.name, c, &field_inits, &static_field_names)?,
                Member::Field(f)       => {
                    if f.init.is_some() && f.modifiers.contains(&Modifier::Static) {
                        static_field_inits.push(f);
                    }
                }
                Member::StaticInit(block) => {
                    let func_id = self.next_func_id();
                    let name = format!("{}.<clinit>", class.name);
                    let flags = FuncFlags { is_clinit: true, ..Default::default() };
                    let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                    ctx.class_name = class.name.clone();
                    ctx.lower_block(block)?;
                    if !ctx.current_block_ends_with_terminator() {
                        ctx.emit(RirInstr::Return(None));
                    }
                    let func = RirFunction {
                        id: func_id, name, params: vec![],
                        return_type: RirType::Void, basic_blocks: ctx.finish(), flags,
                    };
                    self.module.functions.push(func);
                }
                Member::EnumConstant(ec) => {
                    enum_constants.push(ec);
                }
                Member::InnerClass(inner) => {
                    self.lower_class(inner)?;
                    let prefixed = format!("{}.{}", class.name, inner.name);
                    let prefixed_hash = encode_builtin(&prefixed);
                    self.module.class_names.insert(prefixed_hash, inner.name.clone());
                    if let Some(parent) = self.module.class_hierarchy.get(&inner.name).cloned() {
                        self.module.class_hierarchy.insert(prefixed.clone(), parent);
                    }
                    let ctor_name = format!("{}.<init>", inner.name);
                    let prefixed_ctor = format!("{}.<init>", prefixed);
                    if let Some(idx) = self.module.functions.iter().position(|f| f.name == ctor_name) {
                        let mut ctor_copy = self.module.functions[idx].clone();
                        ctor_copy.name = prefixed_ctor;
                        self.module.functions.push(ctor_copy);
                    }
                    let method_prefix = format!("{}.", inner.name);
                    let mut extra_funcs = Vec::new();
                    for f in &self.module.functions {
                        if f.name.starts_with(&method_prefix) && !f.flags.is_constructor && !f.flags.is_clinit {
                            let new_name = format!("{}.{}", prefixed, &f.name[method_prefix.len()..]);
                            let mut copy = f.clone();
                            copy.name = new_name;
                            extra_funcs.push(copy);
                        }
                    }
                    self.module.functions.extend(extra_funcs);
                }
            }
        }

        // Default constructor if none exists but field inits present
        if !has_constructor && !field_inits.is_empty() {
            let func_id = self.next_func_id();
            let name = format!("{}.<init>", class.name);
            let flags = FuncFlags { is_constructor: true, ..Default::default() };
            let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
            let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
            ctx.vars.insert("this".into(), Value("this".into()));
            for (field_name, init_expr) in &field_inits {
                let val = ctx.lower_expr(init_expr)?;
                ctx.emit(RirInstr::SetField {
                    obj: Value("this".into()),
                    field: FieldId(encode_builtin(field_name)),
                    val,
                });
            }
            ctx.emit(RirInstr::Return(None));
            let func = RirFunction {
                id: func_id, name, params,
                return_type: RirType::Void, basic_blocks: ctx.finish(), flags,
            };
            self.module.functions.push(func);
        }

        // <clinit> for static field initializers
        if !static_field_inits.is_empty() {
            let func_id = self.next_func_id();
            let name = format!("{}.<clinit>", class.name);
            let flags = FuncFlags { is_clinit: true, ..Default::default() };
            let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
            for f in &static_field_inits {
                if let Some(init) = &f.init {
                    let val = ctx.lower_expr(init)?;
                    let key = format!("{}.{}", class.name, f.name);
                    ctx.emit(RirInstr::SetStatic {
                        field: FieldId(encode_builtin(&key)),
                        val,
                    });
                }
            }
            ctx.emit(RirInstr::Return(None));
            let func = RirFunction {
                id: func_id, name, params: vec![],
                return_type: RirType::Void, basic_blocks: ctx.finish(), flags,
            };
            self.module.functions.push(func);
        }

        // Enum <clinit> and synthetic methods
        if !enum_constants.is_empty() {
            self.module.field_names.insert(encode_builtin("ordinal"), "ordinal".into());
            self.module.field_names.insert(encode_builtin("__name__"), "__name__".into());

            let func_id = self.next_func_id();
            let name = format!("{}.<clinit>", class.name);
            let flags = FuncFlags { is_clinit: true, ..Default::default() };
            let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
            for (ordinal, ec) in enum_constants.iter().enumerate() {
                let obj = ctx.fresh_value();
                ctx.emit(RirInstr::New {
                    class: ClassId(encode_builtin(&class.name)),
                    ret: obj.clone(),
                });
                let ord_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstInt { ret: ord_val.clone(), value: ordinal as i64 });
                ctx.emit(RirInstr::SetField {
                    obj: obj.clone(),
                    field: FieldId(encode_builtin("ordinal")),
                    val: ord_val,
                });
                let name_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstStr { ret: name_val.clone(), value: ec.name.clone() });
                ctx.emit(RirInstr::SetField {
                    obj: obj.clone(),
                    field: FieldId(encode_builtin("__name__")),
                    val: name_val,
                });
                if !ec.args.is_empty() {
                    let mut call_args = vec![obj.clone()];
                    for arg in &ec.args {
                        let v = ctx.fresh_value();
                        match arg {
                            Expr::IntLit(n)   => ctx.emit(RirInstr::ConstInt { ret: v.clone(), value: *n }),
                            Expr::FloatLit(f) => ctx.emit(RirInstr::ConstFloat { ret: v.clone(), value: *f }),
                            Expr::StrLit(s)   => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: s.clone() }),
                            Expr::BoolLit(b)  => ctx.emit(RirInstr::ConstInt { ret: v.clone(), value: if *b { 1 } else { 0 } }),
                            Expr::Null        => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: "null".into() }),
                            _                 => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: "?".into() }),
                        }
                        call_args.push(v);
                    }
                    let ctor_name = format!("{}.<init>", class.name);
                    ctx.emit(RirInstr::Call {
                        func: FuncId(encode_builtin(&ctor_name)),
                        args: call_args,
                        ret: None,
                    });
                }
                let key = format!("{}.{}", class.name, ec.name);
                ctx.emit(RirInstr::SetStatic {
                    field: FieldId(encode_builtin(&key)),
                    val: obj,
                });
            }
            ctx.emit(RirInstr::Return(None));
            let func = RirFunction {
                id: func_id, name, params: vec![],
                return_type: RirType::Void, basic_blocks: ctx.finish(), flags,
            };
            self.module.functions.push(func);

            // ordinal() method
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.ordinal", class.name);
                let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                ctx.vars.insert("this".into(), Value("this".into()));
                let ret = ctx.fresh_value();
                ctx.emit(RirInstr::GetField { obj: Value("this".into()), field: FieldId(encode_builtin("ordinal")), ret: ret.clone() });
                ctx.emit(RirInstr::Return(Some(ret)));
                self.module.functions.push(RirFunction {
                    id: func_id, name: fname, params,
                    return_type: RirType::I64, basic_blocks: ctx.finish(), flags: FuncFlags::default(),
                });
            }

            // toString() method — returns __name__ field (same as name())
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.toString", class.name);
                let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                ctx.vars.insert("this".into(), Value("this".into()));
                let ret = ctx.fresh_value();
                ctx.emit(RirInstr::GetField { obj: Value("this".into()), field: FieldId(encode_builtin("__name__")), ret: ret.clone() });
                ctx.emit(RirInstr::Return(Some(ret)));
                self.module.functions.push(RirFunction {
                    id: func_id, name: fname, params,
                    return_type: RirType::I64, basic_blocks: ctx.finish(), flags: FuncFlags::default(),
                });
            }

            // name() method
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.name", class.name);
                let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                ctx.vars.insert("this".into(), Value("this".into()));
                let ret = ctx.fresh_value();
                ctx.emit(RirInstr::GetField { obj: Value("this".into()), field: FieldId(encode_builtin("__name__")), ret: ret.clone() });
                ctx.emit(RirInstr::Return(Some(ret)));
                self.module.functions.push(RirFunction {
                    id: func_id, name: fname, params,
                    return_type: RirType::I64, basic_blocks: ctx.finish(), flags: FuncFlags::default(),
                });
            }

            // values() static method
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.values", class.name);
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                let len_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstInt { ret: len_val.clone(), value: enum_constants.len() as i64 });
                let arr = ctx.fresh_value();
                ctx.emit(RirInstr::NewArray { elem_type: RirType::I64, len: len_val, ret: arr.clone() });
                for (i, ec) in enum_constants.iter().enumerate() {
                    let idx = ctx.fresh_value();
                    ctx.emit(RirInstr::ConstInt { ret: idx.clone(), value: i as i64 });
                    let val = ctx.fresh_value();
                    let key = format!("{}.{}", class.name, ec.name);
                    ctx.emit(RirInstr::GetStatic { field: FieldId(encode_builtin(&key)), ret: val.clone() });
                    ctx.emit(RirInstr::ArrayStore { arr: arr.clone(), idx, val });
                }
                ctx.emit(RirInstr::Return(Some(arr)));
                self.module.functions.push(RirFunction {
                    id: func_id, name: fname, params: vec![],
                    return_type: RirType::I64, basic_blocks: ctx.finish(), flags: FuncFlags::default(),
                });
            }
        }

        Ok(())
    }

    fn lower_method(&mut self, class: &str, method: &MethodDecl, static_field_names: &std::collections::HashSet<String>, enum_constant_names: &std::collections::HashSet<String>) -> Result<()> {
        let body = match &method.body {
            Some(b) => b,
            None    => return Ok(()),
        };
        let func_id = self.next_func_id();
        let name = format!("{}.{}", class, method.name);
        let return_type = lower_type(&method.return_ty);
        let params: Vec<(Value, RirType)> = method.params.iter()
            .map(|p| (Value(p.name.clone()), lower_type(&p.ty)))
            .collect();
        let flags = FuncFlags {
            is_clinit:       false,
            is_constructor:  false,
            is_synchronized: method.modifiers.contains(&Modifier::Synchronized),
        };
        let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
        ctx.class_name = class.to_string();
        ctx.static_field_names = static_field_names.clone();
        ctx.enum_constant_names = enum_constant_names.clone();
        let is_instance = !method.modifiers.contains(&Modifier::Static);
        let params = if is_instance {
            let mut p = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(class))))];
            p.extend(params);
            ctx.vars.insert("this".into(), Value("this".into()));
            p
        } else {
            params
        };
        for p in &method.params {
            ctx.vars.insert(p.name.clone(), Value(p.name.clone()));
        }
        ctx.lower_block(body)?;
        if !ctx.current_block_ends_with_terminator() {
            ctx.emit(RirInstr::Return(None));
        }
        let func = RirFunction {
            id: func_id, name, params, return_type,
            basic_blocks: ctx.finish(), flags,
        };
        self.module.functions.push(func);
        self.emit_pending_lambdas()?;
        self.emit_pending_anon_classes()?;
        Ok(())
    }

    fn lower_constructor(&mut self, class: &str, ctor: &ConstructorDecl, field_inits: &[(String, Expr)], static_field_names: &std::collections::HashSet<String>) -> Result<()> {
        let func_id = self.next_func_id();
        let name = format!("{}.<init>", class);
        let mut params: Vec<(Value, RirType)> = vec![
            (Value("this".into()), RirType::Ref(ClassId(encode_builtin(class)))),
        ];
        params.extend(ctor.params.iter().map(|p| (Value(p.name.clone()), lower_type(&p.ty))));
        let flags = FuncFlags { is_constructor: true, ..Default::default() };
        let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
        ctx.class_name = class.to_string();
        ctx.static_field_names = static_field_names.clone();
        ctx.vars.insert("this".into(), Value("this".into()));
        for p in &ctor.params {
            ctx.vars.insert(p.name.clone(), Value(p.name.clone()));
        }
        for (field_name, init_expr) in field_inits {
            let val = ctx.lower_expr(init_expr)?;
            ctx.emit(RirInstr::SetField {
                obj: Value("this".into()),
                field: FieldId(encode_builtin(field_name)),
                val,
            });
        }
        ctx.lower_block(&ctor.body)?;
        if !ctx.current_block_ends_with_terminator() {
            ctx.emit(RirInstr::Return(None));
        }
        let func = RirFunction {
            id: func_id, name, params, return_type: RirType::Void,
            basic_blocks: ctx.finish(), flags,
        };
        self.module.functions.push(func);
        self.emit_pending_lambdas()?;
        self.emit_pending_anon_classes()?;
        Ok(())
    }

    fn emit_pending_lambdas(&mut self) -> Result<()> {
        while let Some(lam) = self.pending_lambdas.pop() {
            let func_id = self.next_func_id();
            let params: Vec<(Value, RirType)> = lam.params.iter()
                .map(|p| (Value(p.clone()), RirType::I64))
                .collect();
            let mut ctx = FuncCtx::new(
                func_id, &mut self.block_id, &mut self.value_id,
                &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter,
                &mut self.pending_anon_classes, &mut self.anon_counter,
            );
            for p in &lam.params {
                ctx.vars.insert(p.clone(), Value(p.clone()));
            }
            for cap in &lam.captures {
                ctx.vars.insert(cap.clone(), Value(cap.clone()));
            }
            match &lam.body {
                LambdaBody::Expr(expr) => {
                    let val = ctx.lower_expr(expr)?;
                    ctx.emit(RirInstr::Return(Some(val)));
                }
                LambdaBody::Block(block) => {
                    ctx.lower_block(block)?;
                    if !ctx.current_block_ends_with_terminator() {
                        ctx.emit(RirInstr::Return(None));
                    }
                }
            }
            self.module.functions.push(RirFunction {
                id: func_id, name: lam.name, params,
                return_type: RirType::I64, basic_blocks: ctx.finish(), flags: FuncFlags::default(),
            });
        }
        Ok(())
    }

    fn emit_pending_anon_classes(&mut self) -> Result<()> {
        while let Some(anon) = self.pending_anon_classes.pop() {
            let class = ClassDecl {
                name: anon.name.clone(),
                kind: ClassKind::Class,
                modifiers: vec![],
                annotations: vec![],
                superclass: Some(anon.parent.clone()),
                interfaces: vec![],
                members: anon.members,
            };
            self.lower_class(&class)?;
        }
        Ok(())
    }

    fn next_func_id(&mut self) -> FuncId {
        let id = FuncId(self.func_id);
        self.func_id += 1;
        id
    }
}

/// Per-function lowering context.
pub(super) struct FuncCtx<'a> {
    #[allow(dead_code)]
    pub(super) func_id:    FuncId,
    pub(super) blocks:     Vec<BasicBlock>,
    pub(super) current:    usize,
    pub(super) block_id:   &'a mut u32,
    pub(super) value_id:   &'a mut u32,
    pub(super) vars:       std::collections::HashMap<String, Value>,
    pub(super) loop_stack: Vec<(BlockId, BlockId)>,
    pub(super) yield_stack: Vec<(BlockId, Value)>,
    pub(super) varargs_methods: &'a std::collections::HashMap<String, usize>,
    pub(super) label_map: std::collections::HashMap<String, (BlockId, BlockId)>,
    pub(super) pending_label: Option<String>,
    pub(super) pending_lambdas: &'a mut Vec<PendingLambda>,
    pub(super) lambda_counter: &'a mut u32,
    pub(super) pending_anon_classes: &'a mut Vec<PendingAnonClass>,
    pub(super) anon_counter: &'a mut u32,
    /// The class this function belongs to (for static field resolution).
    pub(super) class_name: String,
    /// Static field names of the current class — used to distinguish `count` (static) from `this.count` (instance).
    pub(super) static_field_names: std::collections::HashSet<String>,
    /// Enum constant names of the current class — bare idents like `NORTH` resolve as GetStatic, not GetField(this).
    pub(super) enum_constant_names: std::collections::HashSet<String>,
}

impl<'a> FuncCtx<'a> {
    pub(super) fn new(
        func_id: FuncId,
        block_id: &'a mut u32,
        value_id: &'a mut u32,
        varargs_methods: &'a std::collections::HashMap<String, usize>,
        pending_lambdas: &'a mut Vec<PendingLambda>,
        lambda_counter: &'a mut u32,
        pending_anon_classes: &'a mut Vec<PendingAnonClass>,
        anon_counter: &'a mut u32,
    ) -> Self {
        let entry_id = BlockId(*block_id);
        *block_id += 1;
        Self {
            func_id,
            blocks: vec![BasicBlock { id: entry_id, params: vec![], instrs: vec![] }],
            current: 0,
            block_id,
            value_id,
            vars: std::collections::HashMap::new(),
            loop_stack: Vec::new(),
            yield_stack: Vec::new(),
            varargs_methods,
            label_map: std::collections::HashMap::new(),
            pending_label: None,
            pending_lambdas,
            lambda_counter,
            pending_anon_classes,
            anon_counter,
            class_name: String::new(),
            static_field_names: std::collections::HashSet::new(),
            enum_constant_names: std::collections::HashSet::new(),
        }
    }

    pub(super) fn fresh_value(&mut self) -> Value {
        let v = Value(format!("v{}", self.value_id));
        *self.value_id += 1;
        v
    }

    pub(super) fn named_value(&self, hint: &str) -> Value {
        Value(hint.to_string())
    }

    pub(super) fn new_block(&mut self) -> BlockId {
        let id = BlockId(*self.block_id);
        *self.block_id += 1;
        self.blocks.push(BasicBlock { id, params: vec![], instrs: vec![] });
        id
    }

    pub(super) fn switch_to(&mut self, id: BlockId) {
        self.current = self.blocks.iter().position(|b| b.id == id)
            .expect("block not found");
    }

    pub(super) fn emit(&mut self, instr: RirInstr) {
        self.blocks[self.current].instrs.push(instr);
    }

    pub(super) fn current_block_ends_with_terminator(&self) -> bool {
        matches!(self.blocks[self.current].instrs.last(),
            Some(RirInstr::Return(_) | RirInstr::Jump(_) | RirInstr::Branch { .. } |
                 RirInstr::Unreachable | RirInstr::Throw(_)))
    }

    pub(super) fn finish(self) -> Vec<BasicBlock> {
        self.blocks
    }

    /// Write `val` back to the location described by `expr` (local var, instance field, or static field).
    pub(super) fn write_back(&mut self, expr: &Expr, val: Value) {
        match expr {
            Expr::Ident(name) => {
                if self.vars.contains_key(name.as_str()) {
                    let old = self.vars[name.as_str()].clone();
                    self.vars.insert(name.clone(), val.clone());
                    if old != val {
                        self.emit(RirInstr::ConstStr { ret: old, value: format!("__copy__{}", val.0) });
                    }
                } else if !self.class_name.is_empty() && self.static_field_names.contains(name.as_str()) {
                    let key = format!("{}.{}", self.class_name, name);
                    self.emit(RirInstr::SetStatic { field: FieldId(encode_builtin(&key)), val });
                } else if self.vars.contains_key("this") {
                    self.emit(RirInstr::SetField {
                        obj: Value("this".into()),
                        field: FieldId(encode_builtin(name)),
                        val,
                    });
                } else if !self.class_name.is_empty() {
                    let key = format!("{}.{}", self.class_name, name);
                    self.emit(RirInstr::SetStatic { field: FieldId(encode_builtin(&key)), val });
                } else {
                    self.vars.insert(name.clone(), val);
                }
            }
            Expr::Field { obj, name } => {
                if let Expr::Ident(class_name) = obj.as_ref() {
                    if is_static_path(class_name) {
                        let key = format!("{}.{}", class_name, name);
                        self.emit(RirInstr::SetStatic { field: FieldId(encode_builtin(&key)), val });
                        return;
                    }
                }
                if let Ok(obj_val) = self.lower_expr(obj) {
                    self.emit(RirInstr::SetField { obj: obj_val, field: FieldId(encode_builtin(name)), val });
                }
            }
            Expr::Index { arr, idx } => {
                if let (Ok(arr_val), Ok(idx_val)) = (self.lower_expr(arr), self.lower_expr(idx)) {
                    self.emit(RirInstr::ArrayStore { arr: arr_val, idx: idx_val, val });
                }
            }
            _ => {}
        }
    }

    pub(super) fn register_pending_label(&mut self, exit_bb: BlockId, continue_bb: BlockId) {
        if let Some(label) = self.pending_label.take() {
            self.label_map.insert(label, (exit_bb, continue_bb));
        }
    }

    pub(super) fn pack_varargs(&mut self, method_key: &str, mut arg_vals: Vec<Value>) -> Vec<Value> {
        if let Some(&fixed_count) = self.varargs_methods.get(method_key) {
            if arg_vals.len() >= fixed_count {
                let vararg_vals: Vec<Value> = arg_vals.drain(fixed_count..).collect();
                let len_val = self.fresh_value();
                self.emit(RirInstr::ConstInt { ret: len_val.clone(), value: vararg_vals.len() as i64 });
                let arr = self.fresh_value();
                self.emit(RirInstr::NewArray {
                    elem_type: RirType::I64,
                    len: len_val,
                    ret: arr.clone(),
                });
                for (i, v) in vararg_vals.into_iter().enumerate() {
                    let idx = self.fresh_value();
                    self.emit(RirInstr::ConstInt { ret: idx.clone(), value: i as i64 });
                    self.emit(RirInstr::ArrayStore { arr: arr.clone(), idx, val: v });
                }
                arg_vals.push(arr);
            }
        }
        arg_vals
    }
}

/// Convert an AST TypeExpr to a RirType for field type tracking.
pub(crate) fn type_expr_to_rir_type(t: &crate::ast::TypeExpr) -> RirType {
    if t.array_dims > 0 {
        return RirType::Ref(rava_rir::ClassId(encode_builtin("Array")));
    }
    match t.name.as_str() {
        "int" | "long" | "short" | "byte" | "char" => RirType::I64,
        "float" | "double" => RirType::F64,
        "boolean" => RirType::Bool,
        "void" => RirType::Void,
        _ => RirType::Ref(rava_rir::ClassId(encode_builtin(&t.name))),
    }
}
