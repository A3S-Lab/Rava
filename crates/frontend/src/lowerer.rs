//! AST → RIR lowerer.
//!
//! Walks the AST and emits RIR instructions into basic blocks.
//! Full Java syntax coverage: all statements (if, while, do-while, for,
//! for-each, switch, try/catch, break, continue, synchronized, assert),
//! all expressions (lambda, method ref, ternary, cast, instanceof pattern,
//! array init, compound assign, char lit, super).

use rava_common::error::Result;
use rava_rir::{
    BasicBlock, BinOp as RirBinOp, BlockId, ClassId, FieldId, FuncFlags, FuncId, MethodId,
    RirFunction, RirInstr, RirModule, RirType, UnaryOp as RirUnaryOp, Value,
};
use crate::ast::*;

pub struct Lowerer {
    module:    RirModule,
    func_id:   u32,
    block_id:  u32,
    value_id:  u32,
    /// Maps "Class.method" -> (param_count_excluding_vararg, is_variadic)
    varargs_methods: std::collections::HashMap<String, usize>,
    /// Counter for generating unique lambda names.
    lambda_counter: u32,
    /// Pending lambda functions to emit after current function completes.
    pending_lambdas: Vec<PendingLambda>,
    /// Counter for generating unique anonymous class names.
    anon_counter: u32,
    /// Pending anonymous classes to emit after current function completes.
    pending_anon_classes: Vec<PendingAnonClass>,
}

/// A lambda body captured during lowering, to be emitted as a separate function.
struct PendingLambda {
    name: String,
    params: Vec<String>,
    body: LambdaBody,
    /// Captured variables from the enclosing scope.
    captures: Vec<String>,
}

/// An anonymous class captured during lowering, to be emitted as a synthetic class.
struct PendingAnonClass {
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
        // Record class hierarchy for instanceof chain walking
        if let Some(ref parent) = class.superclass {
            self.module.class_hierarchy.insert(class.name.clone(), parent.clone());
        }
        // Also record implemented interfaces
        for iface in &class.interfaces {
            // Store as "ClassName:InterfaceName" so we can check interface instanceof
            let key = format!("{}:{}", class.name, iface);
            self.module.class_hierarchy.insert(key, iface.clone());
        }

        // Register class name for reverse-lookup (needed for classes with no methods)
        self.module.class_names.insert(encode_builtin(&class.name), class.name.clone());

        // Collect instance field initializers for injection into constructors
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

        // Register all field names in the module for reverse-lookup
        for member in &class.members {
            if let Member::Field(f) = member {
                let hash = encode_builtin(&f.name);
                self.module.field_names.insert(hash, f.name.clone());
            }
        }

        // Pre-scan: register varargs methods
        for member in &class.members {
            if let Member::Method(m) = member {
                if let Some(last_param) = m.params.last() {
                    if last_param.variadic {
                        let fixed = m.params.len() - 1;
                        let full_key = format!("{}.{}", class.name, m.name);
                        self.varargs_methods.insert(full_key, fixed);
                        // Also register short name for same-class calls
                        self.varargs_methods.insert(m.name.clone(), fixed);
                    }
                }
            }
        }

        let has_constructor = class.members.iter().any(|m| matches!(m, Member::Constructor(_)));
        let mut static_field_inits: Vec<&FieldDecl> = Vec::new();
        let mut enum_constants: Vec<&EnumConstant> = Vec::new();

        for member in &class.members {
            match member {
                Member::Method(m)      => self.lower_method(&class.name, m)?,
                Member::Constructor(c) => self.lower_constructor(&class.name, c, &field_inits)?,
                Member::Field(f)       => {
                    // Static field with initializer: emit SetStatic in a <clinit> function
                    if f.init.is_some() && f.modifiers.contains(&Modifier::Static) {
                        static_field_inits.push(f);
                    }
                    // Instance field inits are injected into constructors above
                }
                Member::StaticInit(block) => {
                    // Lower static initializer as a <clinit> function
                    let func_id = self.next_func_id();
                    let name = format!("{}.<clinit>", class.name);
                    let flags = FuncFlags { is_clinit: true, ..Default::default() };
                    let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
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
                    // Lower inner class with its own name
                    self.lower_class(inner)?;
                    // Also register with prefixed name (Outer.Inner) as alias
                    let prefixed = format!("{}.{}", class.name, inner.name);
                    let prefixed_hash = encode_builtin(&prefixed);
                    self.module.class_names.insert(prefixed_hash, inner.name.clone());
                    // Copy class hierarchy entries for the prefixed name
                    if let Some(parent) = self.module.class_hierarchy.get(&inner.name).cloned() {
                        self.module.class_hierarchy.insert(prefixed.clone(), parent);
                    }
                    // Duplicate constructor functions with prefixed name
                    let ctor_name = format!("{}.<init>", inner.name);
                    let prefixed_ctor = format!("{}.<init>", prefixed);
                    if let Some(idx) = self.module.functions.iter().position(|f| f.name == ctor_name) {
                        let mut ctor_copy = self.module.functions[idx].clone();
                        ctor_copy.name = prefixed_ctor;
                        self.module.functions.push(ctor_copy);
                    }
                    // Also duplicate methods with prefixed class name
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

        // Generate default constructor if none exists and there are field initializers
        if !has_constructor && !field_inits.is_empty() {
            let func_id = self.next_func_id();
            let name = format!("{}.<init>", class.name);
            let flags = FuncFlags { is_constructor: true, ..Default::default() };
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
                id: func_id, name, params: vec![],
                return_type: RirType::Void, basic_blocks: ctx.finish(), flags,
            };
            self.module.functions.push(func);
        }

        // Generate <clinit> for static field initializers
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

        // Generate enum <clinit> and synthetic methods
        if !enum_constants.is_empty() {
            // Register field names for ordinal and __name__
            self.module.field_names.insert(encode_builtin("ordinal"), "ordinal".into());
            self.module.field_names.insert(encode_builtin("__name__"), "__name__".into());

            // <clinit>: create an object for each enum constant, store as static field
            let func_id = self.next_func_id();
            let name = format!("{}.<clinit>", class.name);
            let flags = FuncFlags { is_clinit: true, ..Default::default() };
            let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
            for (ordinal, ec) in enum_constants.iter().enumerate() {
                // Allocate object
                let obj = ctx.fresh_value();
                ctx.emit(RirInstr::New {
                    class: ClassId(encode_builtin(&class.name)),
                    ret: obj.clone(),
                });
                // Set ordinal field
                let ord_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstInt { ret: ord_val.clone(), value: ordinal as i64 });
                ctx.emit(RirInstr::SetField {
                    obj: obj.clone(),
                    field: FieldId(encode_builtin("ordinal")),
                    val: ord_val,
                });
                // Set __name__ field
                let name_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstStr { ret: name_val.clone(), value: ec.name.clone() });
                ctx.emit(RirInstr::SetField {
                    obj: obj.clone(),
                    field: FieldId(encode_builtin("__name__")),
                    val: name_val,
                });
                // Call constructor with args if present
                if !ec.args.is_empty() {
                    let mut call_args = vec![obj.clone()]; // this
                    for arg in &ec.args {
                        let v = ctx.fresh_value();
                        match arg {
                            Expr::IntLit(n) => ctx.emit(RirInstr::ConstInt { ret: v.clone(), value: *n }),
                            Expr::FloatLit(f) => ctx.emit(RirInstr::ConstFloat { ret: v.clone(), value: *f }),
                            Expr::StrLit(s) => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: s.clone() }),
                            Expr::BoolLit(b) => ctx.emit(RirInstr::ConstInt { ret: v.clone(), value: if *b { 1 } else { 0 } }),
                            Expr::Null => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: "null".into() }),
                            _ => ctx.emit(RirInstr::ConstStr { ret: v.clone(), value: "?".into() }),
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
                // Store as static field: ClassName.CONSTANT_NAME
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

            // Generate ordinal() method
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.ordinal", class.name);
                let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                ctx.vars.insert("this".into(), Value("this".into()));
                let ret = ctx.fresh_value();
                ctx.emit(RirInstr::GetField {
                    obj: Value("this".into()),
                    field: FieldId(encode_builtin("ordinal")),
                    ret: ret.clone(),
                });
                ctx.emit(RirInstr::Return(Some(ret)));
                let func = RirFunction {
                    id: func_id, name: fname, params,
                    return_type: RirType::I64, basic_blocks: ctx.finish(),
                    flags: FuncFlags::default(),
                };
                self.module.functions.push(func);
            }

            // Generate name() method
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.name", class.name);
                let params = vec![(Value("this".into()), RirType::Ref(ClassId(encode_builtin(&class.name))))];
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                ctx.vars.insert("this".into(), Value("this".into()));
                let ret = ctx.fresh_value();
                ctx.emit(RirInstr::GetField {
                    obj: Value("this".into()),
                    field: FieldId(encode_builtin("__name__")),
                    ret: ret.clone(),
                });
                ctx.emit(RirInstr::Return(Some(ret)));
                let func = RirFunction {
                    id: func_id, name: fname, params,
                    return_type: RirType::I64, basic_blocks: ctx.finish(),
                    flags: FuncFlags::default(),
                };
                self.module.functions.push(func);
            }

            // Generate values() static method — returns array of all constants
            {
                let func_id = self.next_func_id();
                let fname = format!("{}.values", class.name);
                let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
                let len_val = ctx.fresh_value();
                ctx.emit(RirInstr::ConstInt { ret: len_val.clone(), value: enum_constants.len() as i64 });
                let arr = ctx.fresh_value();
                ctx.emit(RirInstr::NewArray {
                    elem_type: RirType::I64,
                    len: len_val,
                    ret: arr.clone(),
                });
                for (i, ec) in enum_constants.iter().enumerate() {
                    let idx = ctx.fresh_value();
                    ctx.emit(RirInstr::ConstInt { ret: idx.clone(), value: i as i64 });
                    let val = ctx.fresh_value();
                    let key = format!("{}.{}", class.name, ec.name);
                    ctx.emit(RirInstr::GetStatic {
                        field: FieldId(encode_builtin(&key)),
                        ret: val.clone(),
                    });
                    ctx.emit(RirInstr::ArrayStore { arr: arr.clone(), idx, val });
                }
                ctx.emit(RirInstr::Return(Some(arr)));
                let func = RirFunction {
                    id: func_id, name: fname, params: vec![],
                    return_type: RirType::I64, basic_blocks: ctx.finish(),
                    flags: FuncFlags::default(),
                };
                self.module.functions.push(func);
            }
        }

        Ok(())
    }

    fn lower_method(&mut self, class: &str, method: &MethodDecl) -> Result<()> {
        let body = match &method.body {
            Some(b) => b,
            None    => return Ok(()), // abstract / interface method
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
        // Make `this` available in instance methods — prepend to params
        let is_instance = !method.modifiers.contains(&Modifier::Static);
        let params = if is_instance {
            let mut p = vec![
                (Value("this".into()), RirType::Ref(ClassId(encode_builtin(class)))),
            ];
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

    fn lower_constructor(&mut self, class: &str, ctor: &ConstructorDecl, field_inits: &[(String, Expr)]) -> Result<()> {
        let func_id = self.next_func_id();
        let name = format!("{}.<init>", class);
        // "this" is the first implicit parameter
        let mut params: Vec<(Value, RirType)> = vec![
            (Value("this".into()), RirType::Ref(ClassId(encode_builtin(class)))),
        ];
        params.extend(ctor.params.iter()
            .map(|p| (Value(p.name.clone()), lower_type(&p.ty))));
        let flags = FuncFlags { is_constructor: true, ..Default::default() };
        let mut ctx = FuncCtx::new(func_id, &mut self.block_id, &mut self.value_id, &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter, &mut self.pending_anon_classes, &mut self.anon_counter);
        // "this" is already in params, register in vars
        ctx.vars.insert("this".into(), Value("this".into()));
        for p in &ctor.params {
            ctx.vars.insert(p.name.clone(), Value(p.name.clone()));
        }
        // Inject field initializers before the constructor body
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

    /// Emit all pending lambdas as top-level RIR functions.
    fn emit_pending_lambdas(&mut self) -> Result<()> {
        while let Some(lam) = self.pending_lambdas.pop() {
            let func_id = self.next_func_id();
            let params: Vec<(Value, RirType)> = lam.params.iter()
                .map(|p| (Value(p.clone()), RirType::I64))
                .collect();
            let flags = FuncFlags::default();
            let mut ctx = FuncCtx::new(
                func_id, &mut self.block_id, &mut self.value_id,
                &self.varargs_methods, &mut self.pending_lambdas, &mut self.lambda_counter,
                &mut self.pending_anon_classes, &mut self.anon_counter,
            );
            for p in &lam.params {
                ctx.vars.insert(p.clone(), Value(p.clone()));
            }
            // Inject captured variables as parameters too
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
            let func = RirFunction {
                id: func_id,
                name: lam.name,
                params,
                return_type: RirType::I64,
                basic_blocks: ctx.finish(),
                flags,
            };
            self.module.functions.push(func);
        }
        Ok(())
    }

    /// Emit all pending anonymous classes as synthetic ClassDecls.
    fn emit_pending_anon_classes(&mut self) -> Result<()> {
        while let Some(anon) = self.pending_anon_classes.pop() {
            let class = ClassDecl {
                name: anon.name.clone(),
                kind: ClassKind::Class,
                modifiers: vec![],
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
struct FuncCtx<'a> {
    #[allow(dead_code)]
    func_id:    FuncId,
    blocks:     Vec<BasicBlock>,
    current:    usize,
    block_id:   &'a mut u32,
    value_id:   &'a mut u32,
    vars:       std::collections::HashMap<String, Value>,
    /// Stack of (break_target, continue_target) for loops.
    loop_stack: Vec<(BlockId, BlockId)>,
    /// Stack of (exit_bb, result_value) for switch-expression yield.
    yield_stack: Vec<(BlockId, Value)>,
    /// Maps "Class.method" -> param_count_before_vararg
    varargs_methods: &'a std::collections::HashMap<String, usize>,
    /// Maps label name -> (break_target, continue_target) for labeled loops.
    label_map: std::collections::HashMap<String, (BlockId, BlockId)>,
    /// Pending label for the next loop statement.
    pending_label: Option<String>,
    /// Pending lambdas to emit after this function.
    pending_lambdas: &'a mut Vec<PendingLambda>,
    /// Lambda counter for unique names.
    lambda_counter: &'a mut u32,
    /// Pending anonymous classes to emit after this function.
    pending_anon_classes: &'a mut Vec<PendingAnonClass>,
    /// Anonymous class counter for unique names.
    anon_counter: &'a mut u32,
}

impl<'a> FuncCtx<'a> {
    fn new(func_id: FuncId, block_id: &'a mut u32, value_id: &'a mut u32, varargs_methods: &'a std::collections::HashMap<String, usize>, pending_lambdas: &'a mut Vec<PendingLambda>, lambda_counter: &'a mut u32, pending_anon_classes: &'a mut Vec<PendingAnonClass>, anon_counter: &'a mut u32) -> Self {
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
        }
    }

    fn fresh_value(&mut self) -> Value {
        let v = Value(format!("v{}", self.value_id));
        *self.value_id += 1;
        v
    }

    fn named_value(&self, hint: &str) -> Value {
        Value(hint.to_string())
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(*self.block_id);
        *self.block_id += 1;
        self.blocks.push(BasicBlock { id, params: vec![], instrs: vec![] });
        id
    }

    fn switch_to(&mut self, id: BlockId) {
        self.current = self.blocks.iter().position(|b| b.id == id)
            .expect("block not found");
    }

    fn emit(&mut self, instr: RirInstr) {
        self.blocks[self.current].instrs.push(instr);
    }

    fn current_block_ends_with_terminator(&self) -> bool {
        matches!(self.blocks[self.current].instrs.last(),
            Some(RirInstr::Return(_) | RirInstr::Jump(_) | RirInstr::Branch { .. } |
                 RirInstr::Unreachable | RirInstr::Throw(_)))
    }

    fn finish(self) -> Vec<BasicBlock> {
        self.blocks
    }

    /// Register a pending label (from Stmt::Labeled) for the current loop's targets.
    fn register_pending_label(&mut self, exit_bb: BlockId, continue_bb: BlockId) {
        if let Some(label) = self.pending_label.take() {
            self.label_map.insert(label, (exit_bb, continue_bb));
        }
    }

    /// If `method_key` is a known varargs method, pack trailing args into an array.
    fn pack_varargs(&mut self, method_key: &str, mut arg_vals: Vec<Value>) -> Vec<Value> {
        if let Some(&fixed_count) = self.varargs_methods.get(method_key) {
            if arg_vals.len() >= fixed_count {
                let vararg_vals: Vec<Value> = arg_vals.drain(fixed_count..).collect();
                // Create array with vararg elements
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

    // ── Statements ────────────────────────────────────────────────────────────

    fn lower_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.0 {
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Empty => {}
            Stmt::Expr(e) => { self.lower_expr(e)?; }
            Stmt::Return(e) => {
                let val = e.as_ref().map(|e| self.lower_expr(e)).transpose()?;
                self.emit(RirInstr::Return(val));
            }
            Stmt::LocalVar { name, init, .. } => {
                if let Some(init) = init {
                    let val = self.lower_expr_into(init, Some(name))?;
                    self.vars.insert(name.clone(), val);
                }
            }
            Stmt::Block(b) => self.lower_block(b)?,
            Stmt::If { cond, then, else_ } => {
                let cond_val = self.lower_expr(cond)?;
                let pre_if   = self.current;
                let then_bb  = self.new_block();
                let else_bb  = self.new_block();
                let merge_bb = self.new_block();
                self.blocks[pre_if].instrs.push(RirInstr::Branch {
                    cond: cond_val, then_bb, else_bb,
                });
                self.switch_to(then_bb);
                self.lower_stmt(then)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(merge_bb));
                }
                self.switch_to(else_bb);
                if let Some(else_stmt) = else_ {
                    self.lower_stmt(else_stmt)?;
                }
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(merge_bb));
                }
                self.switch_to(merge_bb);
            }
            Stmt::While { cond, body } => {
                let pre_while = self.current;
                let header_bb = self.new_block();
                let body_bb   = self.new_block();
                let exit_bb   = self.new_block();
                self.blocks[pre_while].instrs.push(RirInstr::Jump(header_bb));
                self.switch_to(header_bb);
                let cond_val = self.lower_expr(cond)?;
                self.emit(RirInstr::Branch { cond: cond_val, then_bb: body_bb, else_bb: exit_bb });
                self.loop_stack.push((exit_bb, header_bb));
                self.register_pending_label(exit_bb, header_bb);
                self.switch_to(body_bb);
                self.lower_stmt(body)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(header_bb));
                }
                self.loop_stack.pop();
                self.switch_to(exit_bb);
            }
            Stmt::DoWhile { body, cond } => {
                let pre = self.current;
                let body_bb   = self.new_block();
                let header_bb = self.new_block();
                let exit_bb   = self.new_block();
                self.blocks[pre].instrs.push(RirInstr::Jump(body_bb));
                self.loop_stack.push((exit_bb, header_bb));
                self.register_pending_label(exit_bb, header_bb);
                self.switch_to(body_bb);
                self.lower_stmt(body)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(header_bb));
                }
                self.loop_stack.pop();
                self.switch_to(header_bb);
                let cond_val = self.lower_expr(cond)?;
                self.emit(RirInstr::Branch { cond: cond_val, then_bb: body_bb, else_bb: exit_bb });
                self.switch_to(exit_bb);
            }
            Stmt::For { init, cond, update, body } => {
                if let Some(init) = init { self.lower_stmt(init)?; }
                let pre_for   = self.current;
                let header_bb = self.new_block();
                let body_bb   = self.new_block();
                let update_bb = self.new_block();
                let exit_bb   = self.new_block();
                self.blocks[pre_for].instrs.push(RirInstr::Jump(header_bb));
                self.switch_to(header_bb);
                if let Some(cond) = cond {
                    let cond_val = self.lower_expr(cond)?;
                    self.emit(RirInstr::Branch { cond: cond_val, then_bb: body_bb, else_bb: exit_bb });
                } else {
                    self.emit(RirInstr::Jump(body_bb));
                }
                // continue jumps to update_bb, break jumps to exit_bb
                self.loop_stack.push((exit_bb, update_bb));
                self.register_pending_label(exit_bb, update_bb);
                self.switch_to(body_bb);
                self.lower_stmt(body)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(update_bb));
                }
                self.loop_stack.pop();
                self.switch_to(update_bb);
                for u in update { self.lower_expr(u)?; }
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(header_bb));
                }
                self.switch_to(exit_bb);
            }
            Stmt::ForEach { ty: _, name, iterable, body } => {
                // Lower as iterator pattern:
                //   var __it = iterable.__method__iterator();
                //   while (__it.__method__hasNext()) { var name = __it.__method__next(); body; }
                let collection = self.lower_expr(iterable)?;

                // __it = collection.iterator()
                let iter_var = self.fresh_value();
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin("__method__iterator")),
                    args: vec![collection],
                    ret:  Some(iter_var.clone()),
                });

                let pre = self.current;
                let header_bb = self.new_block();
                let body_bb   = self.new_block();
                let exit_bb   = self.new_block();
                self.blocks[pre].instrs.push(RirInstr::Jump(header_bb));

                // header: __it.hasNext()
                self.switch_to(header_bb);
                let has_next = self.fresh_value();
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin("__method__hasNext")),
                    args: vec![iter_var.clone()],
                    ret:  Some(has_next.clone()),
                });
                self.emit(RirInstr::Branch { cond: has_next, then_bb: body_bb, else_bb: exit_bb });

                // body: var name = __it.next(); ...
                self.loop_stack.push((exit_bb, header_bb));
                self.register_pending_label(exit_bb, header_bb);
                self.switch_to(body_bb);
                let elem = self.named_value(name);
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin("__method__next")),
                    args: vec![iter_var.clone()],
                    ret:  Some(elem.clone()),
                });
                self.vars.insert(name.clone(), elem);
                self.lower_stmt(body)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(header_bb));
                }
                self.loop_stack.pop();

                self.switch_to(exit_bb);
            }
            Stmt::Break(label) => {
                let target = if let Some(lbl) = label {
                    self.label_map.get(lbl).map(|&(exit, _)| exit)
                } else {
                    self.loop_stack.last().map(|&(exit, _)| exit)
                };
                if let Some(exit_bb) = target {
                    self.emit(RirInstr::Jump(exit_bb));
                }
            }
            Stmt::Continue(label) => {
                let target = if let Some(lbl) = label {
                    self.label_map.get(lbl).map(|&(_, cont)| cont)
                } else {
                    self.loop_stack.last().map(|&(_, cont)| cont)
                };
                if let Some(cont_bb) = target {
                    self.emit(RirInstr::Jump(cont_bb));
                }
            }
            Stmt::Labeled { label, stmt } => {
                self.pending_label = Some(label.clone());
                self.lower_stmt(stmt)?;
                // If the inner wasn't a loop (pending_label not consumed), clean up
                if self.pending_label.take().is_some() {
                    self.label_map.remove(label);
                }
            }
            Stmt::Throw(e) => {
                let val = self.lower_expr(e)?;
                self.emit(RirInstr::Throw(val));
            }
            Stmt::TryCatch { try_body, catches, finally_body } => {
                // Emit try body with per-catch-type blocks.
                // Each catch clause gets its own block; the interpreter matches exception types.
                let finally_bb = if finally_body.is_some() { Some(self.new_block()) } else { None };
                let exit_bb = self.new_block();

                // Create a block for each catch clause and register type markers
                let mut catch_blocks: Vec<(BlockId, Vec<String>)> = Vec::new();
                for catch in catches {
                    let cbb = self.new_block();
                    let types: Vec<String> = catch.exception_types.iter()
                        .map(|t| t.name.clone())
                        .collect();
                    catch_blocks.push((cbb, types));
                }

                // Mark try region: emit handler registrations (last catch first = stack order)
                for (cbb, types) in catch_blocks.iter().rev() {
                    let type_list = types.join("|");
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: format!("__try_catch__{}:{}", cbb.0, type_list),
                    });
                }

                // Lower try body
                self.lower_block(try_body)?;

                // End of try: clear all exception handlers for this try
                for _ in &catch_blocks {
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: "__try_end__".into(),
                    });
                }
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(finally_bb.unwrap_or(exit_bb)));
                }

                // Catch blocks
                for (i, (cbb, _types)) in catch_blocks.iter().enumerate() {
                    self.switch_to(*cbb);
                    let catch = &catches[i];
                    let exc_val = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: exc_val.clone(),
                        value: "__exception__".into(),
                    });
                    self.vars.insert(catch.name.clone(), exc_val);
                    self.lower_block(&catch.body)?;
                    if !self.current_block_ends_with_terminator() {
                        self.emit(RirInstr::Jump(finally_bb.unwrap_or(exit_bb)));
                    }
                }

                // Finally block
                if let Some(fbb) = finally_bb {
                    self.switch_to(fbb);
                    self.lower_block(finally_body.as_ref().unwrap())?;
                    if !self.current_block_ends_with_terminator() {
                        self.emit(RirInstr::Jump(exit_bb));
                    }
                }

                self.switch_to(exit_bb);
            }
            Stmt::Switch { expr, cases } => {
                let switch_val = self.lower_expr(expr)?;
                let exit_bb = self.new_block();
                let mut check = self.current;

                // Push a pseudo loop for break statements in switch
                self.loop_stack.push((exit_bb, exit_bb));

                for case in cases {
                    match &case.labels {
                        Some(labels) => {
                            self.switch_to(self.blocks[check].id);
                            // Multi-label: OR all comparisons together
                            let mut or_val = None;
                            for label_expr in labels {
                                let label_val = self.lower_expr(label_expr)?;
                                let cmp = self.fresh_value();
                                self.emit(RirInstr::BinOp {
                                    op: RirBinOp::Eq, lhs: switch_val.clone(),
                                    rhs: label_val, ret: cmp.clone(),
                                });
                                or_val = Some(match or_val {
                                    None => cmp,
                                    Some(prev) => {
                                        let merged = self.fresh_value();
                                        self.emit(RirInstr::BinOp {
                                            op: RirBinOp::Or, lhs: prev, rhs: cmp, ret: merged.clone(),
                                        });
                                        merged
                                    }
                                });
                            }
                            let cond_ret = or_val.unwrap();
                            let body_bb = self.new_block();
                            let next_bb = self.new_block();
                            self.blocks[check].instrs.push(
                                RirInstr::Branch { cond: cond_ret, then_bb: body_bb, else_bb: next_bb }
                            );
                            self.switch_to(body_bb);
                            for stmt in &case.body { self.lower_stmt(stmt)?; }
                            if !self.current_block_ends_with_terminator() {
                                self.emit(RirInstr::Jump(exit_bb));
                            }
                            check = self.blocks.iter().position(|b| b.id == next_bb).unwrap();
                        }
                        None => {
                            self.switch_to(self.blocks[check].id);
                            for stmt in &case.body { self.lower_stmt(stmt)?; }
                            if !self.current_block_ends_with_terminator() {
                                self.emit(RirInstr::Jump(exit_bb));
                            }
                        }
                    }
                }
                self.loop_stack.pop();
                self.switch_to(self.blocks[check].id);
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(exit_bb));
                }
                self.switch_to(exit_bb);
            }
            Stmt::Synchronized { body, .. } => {
                // Lower the body directly (monitor enter/exit is Phase 3)
                self.lower_block(body)?;
            }
            Stmt::Assert { expr, message } => {
                // Lower as: if (!expr) throw new AssertionError(message)
                let cond_val = self.lower_expr(expr)?;
                let pre = self.current;
                let throw_bb = self.new_block();
                let cont_bb  = self.new_block();
                self.blocks[pre].instrs.push(RirInstr::Branch {
                    cond: cond_val, then_bb: cont_bb, else_bb: throw_bb,
                });
                self.switch_to(throw_bb);
                let msg = if let Some(m) = message {
                    self.lower_expr(m)?
                } else {
                    let v = self.fresh_value();
                    self.emit(RirInstr::ConstStr { ret: v.clone(), value: "Assertion failed".into() });
                    v
                };
                self.emit(RirInstr::Throw(msg));
                self.switch_to(cont_bb);
            }
            Stmt::Yield(expr) => {
                let val = self.lower_expr(expr)?;
                if let Some(&(exit_bb, ref result)) = self.yield_stack.last() {
                    // Copy yield value into the result variable via __copy__ marker
                    let result_clone = result.clone();
                    self.emit(RirInstr::ConstStr {
                        ret: result_clone,
                        value: format!("__copy__{}", val.0),
                    });
                    self.emit(RirInstr::Jump(exit_bb));
                }
            }
        }
        Ok(())
    }

    // ── Expression helpers ────────────────────────────────────────────────────

    /// Lower `expr`, storing the result into `var_name` if provided.
    fn lower_expr_into(&mut self, expr: &Expr, var_name: Option<&str>) -> Result<Value> {
        let name = match var_name {
            Some(n) => n,
            None    => return self.lower_expr(expr),
        };
        match expr {
            Expr::IntLit(n) => {
                let ret = self.named_value(name);
                self.emit(RirInstr::ConstInt { ret: ret.clone(), value: *n });
                Ok(ret)
            }
            Expr::FloatLit(f) => {
                let ret = self.named_value(name);
                self.emit(RirInstr::ConstFloat { ret: ret.clone(), value: *f });
                Ok(ret)
            }
            Expr::StrLit(s) => {
                let ret = self.named_value(name);
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: s.clone() });
                Ok(ret)
            }
            Expr::CharLit(c) => {
                let ret = self.named_value(name);
                self.emit(RirInstr::ConstInt { ret: ret.clone(), value: *c });
                Ok(ret)
            }
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.lower_expr(lhs)?;
                let r = self.lower_expr(rhs)?;
                let ret = self.named_value(name);
                self.emit(RirInstr::BinOp { op: lower_binop(op), lhs: l, rhs: r, ret: ret.clone() });
                Ok(ret)
            }
            _ => {
                let src = self.lower_expr(expr)?;
                let ret = self.named_value(name);
                if src != ret {
                    // Copy via ConstStr marker that interpreter resolves
                    self.emit(RirInstr::ConstStr {
                        ret: ret.clone(),
                        value: format!("__copy__{}", src.0),
                    });
                }
                Ok(ret)
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::IntLit(n) => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstInt { ret: ret.clone(), value: *n });
                Ok(ret)
            }
            Expr::FloatLit(f) => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstFloat { ret: ret.clone(), value: *f });
                Ok(ret)
            }
            Expr::StrLit(s) => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: s.clone() });
                Ok(ret)
            }
            Expr::CharLit(c) => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstInt { ret: ret.clone(), value: *c });
                Ok(ret)
            }
            Expr::BoolLit(b) => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstBool { ret: ret.clone(), value: *b });
                Ok(ret)
            }
            Expr::Null => {
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstNull { ret: ret.clone() });
                Ok(ret)
            }
            Expr::Ident(name) => {
                Ok(self.vars.get(name).cloned().unwrap_or_else(|| Value(name.clone())))
            }
            Expr::This  => Ok(Value("this".into())),
            Expr::Super => Ok(Value("super".into())),

            Expr::Field { obj, name } => {
                let ret = self.fresh_value();
                // Static field access: ClassName.field
                if let Expr::Ident(class_name) = obj.as_ref() {
                    if is_static_path(class_name) {
                        let key = format!("{}.{}", class_name, name);
                        self.emit(RirInstr::GetStatic {
                            field: FieldId(encode_builtin(&key)),
                            ret: ret.clone(),
                        });
                        return Ok(ret);
                    }
                }
                let obj_val = self.lower_expr(obj)?;
                self.emit(RirInstr::GetField {
                    obj: obj_val,
                    field: FieldId(encode_builtin(name)),
                    ret: ret.clone(),
                });
                Ok(ret)
            }

            Expr::Call { callee, args } => {
                let ret = self.fresh_value();

                if let Expr::Field { obj, name: method_name } = callee.as_ref() {
                    let callee_str = expr_to_str(obj);
                    let full = format!("{}.{}", callee_str, method_name);

                    if is_static_path(&callee_str) {
                        let arg_vals: Vec<Value> = args.iter()
                            .map(|a| self.lower_expr(a))
                            .collect::<Result<_>>()?;
                        let arg_vals = self.pack_varargs(&full, arg_vals);
                        self.emit(RirInstr::Call {
                            func: FuncId(encode_builtin(&full)),
                            args: arg_vals,
                            ret:  Some(ret.clone()),
                        });
                        return Ok(ret);
                    }

                    // Instance method call
                    let receiver_val = self.lower_expr(obj)?;
                    let mut arg_vals = vec![receiver_val];
                    for a in args { arg_vals.push(self.lower_expr(a)?); }
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin(&format!("__method__{}", method_name))),
                        args: arg_vals,
                        ret:  Some(ret.clone()),
                    });
                    return Ok(ret);
                }

                // super(...) call
                if matches!(callee.as_ref(), Expr::Super) {
                    let arg_vals: Vec<Value> = args.iter()
                        .map(|a| self.lower_expr(a))
                        .collect::<Result<_>>()?;
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin("super.<init>")),
                        args: arg_vals,
                        ret:  None,
                    });
                    return Ok(Value("this".into()));
                }

                // this(...) constructor delegation
                if matches!(callee.as_ref(), Expr::This) {
                    let mut arg_vals = vec![Value("this".into())];
                    for a in args {
                        arg_vals.push(self.lower_expr(a)?);
                    }
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin("this.<init>")),
                        args: arg_vals,
                        ret:  None,
                    });
                    return Ok(Value("this".into()));
                }

                // Regular static/free call — check for same-class varargs
                let arg_vals: Vec<Value> = args.iter()
                    .map(|a| self.lower_expr(a))
                    .collect::<Result<_>>()?;
                if let Expr::Ident(name) = callee.as_ref() {
                    // If `this` is in scope, treat bare method calls as this.method()
                    // so virtual dispatch works correctly for inherited methods
                    if self.vars.contains_key("this") {
                        let mut method_args = vec![Value("this".into())];
                        method_args.extend(self.pack_varargs(name, arg_vals));
                        self.emit(RirInstr::Call {
                            func: FuncId(encode_builtin(&format!("__method__{}", name))),
                            args: method_args,
                            ret:  Some(ret.clone()),
                        });
                        return Ok(ret);
                    }
                    let arg_vals = self.pack_varargs(name, arg_vals);
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin(name)),
                        args: arg_vals,
                        ret:  Some(ret.clone()),
                    });
                    return Ok(ret);
                }

                let receiver = self.lower_expr(callee)?;
                self.emit(RirInstr::CallVirtual {
                    receiver,
                    method: MethodId(0),
                    args: arg_vals,
                    ret: Some(ret.clone()),
                });
                Ok(ret)
            }

            Expr::BinOp { op, lhs, rhs } => {
                // Short-circuit for && and ||
                if *op == BinOp::And {
                    return self.lower_short_circuit(lhs, rhs, true);
                }
                if *op == BinOp::Or {
                    return self.lower_short_circuit(lhs, rhs, false);
                }
                let l = self.lower_expr(lhs)?;
                let r = self.lower_expr(rhs)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::BinOp { op: lower_binop(op), lhs: l, rhs: r, ret: ret.clone() });
                Ok(ret)
            }

            Expr::UnaryOp { op, expr } => {
                let var_name = if let Expr::Ident(n) = expr.as_ref() { Some(n.clone()) } else { None };
                let val = self.lower_expr(expr)?;
                let ret = self.fresh_value();
                match op {
                    UnaryOp::Neg => {
                        self.emit(RirInstr::UnaryOp { op: RirUnaryOp::Neg, operand: val, ret: ret.clone() });
                    }
                    UnaryOp::Not => {
                        self.emit(RirInstr::UnaryOp { op: RirUnaryOp::Not, operand: val, ret: ret.clone() });
                    }
                    UnaryOp::BitNot => {
                        // ~x = x ^ -1
                        let neg1 = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: neg1.clone(), value: -1 });
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Xor, lhs: val, rhs: neg1, ret: ret.clone(),
                        });
                    }
                    UnaryOp::PostInc | UnaryOp::PreInc => {
                        let one = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: one.clone(), value: 1 });
                        let write_ret = var_name.as_deref()
                            .map(|n| self.named_value(n))
                            .unwrap_or_else(|| ret.clone());
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Add, lhs: val, rhs: one, ret: write_ret.clone(),
                        });
                        if let Some(n) = &var_name { self.vars.insert(n.clone(), write_ret.clone()); }
                        return Ok(write_ret);
                    }
                    UnaryOp::PostDec | UnaryOp::PreDec => {
                        let one = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: one.clone(), value: 1 });
                        let write_ret = var_name.as_deref()
                            .map(|n| self.named_value(n))
                            .unwrap_or_else(|| ret.clone());
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Sub, lhs: val, rhs: one, ret: write_ret.clone(),
                        });
                        if let Some(n) = &var_name { self.vars.insert(n.clone(), write_ret.clone()); }
                        return Ok(write_ret);
                    }
                }
                Ok(ret)
            }

            Expr::Assign { lhs, rhs } => {
                // Array element assignment
                if let Expr::Index { arr, idx } = lhs.as_ref() {
                    let arr_val = self.lower_expr(arr)?;
                    let idx_val = self.lower_expr(idx)?;
                    let rhs_val = self.lower_expr(rhs)?;
                    self.emit(RirInstr::ArrayStore { arr: arr_val, idx: idx_val, val: rhs_val.clone() });
                    return Ok(rhs_val);
                }
                // Field assignment
                if let Expr::Field { obj, name } = lhs.as_ref() {
                    // Static field assignment: ClassName.field = val
                    if let Expr::Ident(class_name) = obj.as_ref() {
                        if is_static_path(class_name) {
                            let rhs_val = self.lower_expr(rhs)?;
                            let key = format!("{}.{}", class_name, name);
                            self.emit(RirInstr::SetStatic {
                                field: FieldId(encode_builtin(&key)),
                                val: rhs_val.clone(),
                            });
                            return Ok(rhs_val);
                        }
                    }
                    let obj_val = self.lower_expr(obj)?;
                    let rhs_val = self.lower_expr(rhs)?;
                    self.emit(RirInstr::SetField {
                        obj: obj_val, field: FieldId(encode_builtin(name)), val: rhs_val.clone(),
                    });
                    return Ok(rhs_val);
                }
                // Simple variable assignment
                let var_name = if let Expr::Ident(n) = lhs.as_ref() { Some(n.as_str()) } else { None };
                let val = self.lower_expr_into(rhs, var_name)?;
                if let Expr::Ident(name) = lhs.as_ref() {
                    self.vars.insert(name.clone(), val.clone());
                }
                Ok(val)
            }

            Expr::CompoundAssign { op, lhs, rhs } => {
                // Desugar: lhs op= rhs → lhs = lhs op rhs
                let desugared = Expr::Assign {
                    lhs: lhs.clone(),
                    rhs: Box::new(Expr::BinOp {
                        op: op.clone(), lhs: lhs.clone(), rhs: rhs.clone(),
                    }),
                };
                self.lower_expr(&desugared)
            }

            Expr::New { ty, args, body } => {
                let arg_vals: Vec<Value> = args.iter()
                    .map(|a| self.lower_expr(a))
                    .collect::<Result<_>>()?;

                // Anonymous class: generate a synthetic class name
                let class_name = if let Some(members) = body {
                    let idx = *self.anon_counter;
                    *self.anon_counter += 1;
                    let anon_name = format!("__anon_{}_{}", ty.name, idx);
                    self.pending_anon_classes.push(PendingAnonClass {
                        name: anon_name.clone(),
                        parent: ty.name.clone(),
                        members: members.clone(),
                    });
                    anon_name
                } else {
                    ty.name.clone()
                };

                let ret = self.fresh_value();
                let class_id = ClassId(encode_builtin(&class_name));
                self.emit(RirInstr::New { class: class_id, ret: ret.clone() });
                // Call constructor with `this` as first arg
                let mut ctor_args = vec![ret.clone()];
                ctor_args.extend(arg_vals);
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin(&format!("{}.<init>", class_name))),
                    args: ctor_args,
                    ret:  None,
                });
                Ok(ret)
            }

            Expr::NewArray { ty, len } => {
                let len_val = self.lower_expr(len)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::NewArray {
                    elem_type: lower_type_name(&ty.name), len: len_val, ret: ret.clone(),
                });
                Ok(ret)
            }

            Expr::NewMultiArray { ty, dims } => {
                let dim_vals: Vec<Value> = dims.iter()
                    .map(|d| self.lower_expr(d))
                    .collect::<Result<_>>()?;
                let ret = self.fresh_value();
                self.emit(RirInstr::NewMultiArray {
                    elem_type: lower_type_name(&ty.name), dims: dim_vals, ret: ret.clone(),
                });
                Ok(ret)
            }

            Expr::ArrayInit { elements, .. } => {
                // Create array with elements
                let len = self.fresh_value();
                self.emit(RirInstr::ConstInt { ret: len.clone(), value: elements.len() as i64 });
                let arr = self.fresh_value();
                self.emit(RirInstr::NewArray {
                    elem_type: RirType::I32, len, ret: arr.clone(),
                });
                for (i, elem) in elements.iter().enumerate() {
                    let val = self.lower_expr(elem)?;
                    let idx = self.fresh_value();
                    self.emit(RirInstr::ConstInt { ret: idx.clone(), value: i as i64 });
                    self.emit(RirInstr::ArrayStore { arr: arr.clone(), idx, val });
                }
                Ok(arr)
            }

            Expr::Index { arr, idx } => {
                let arr_val = self.lower_expr(arr)?;
                let idx_val = self.lower_expr(idx)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::ArrayLoad { arr: arr_val, idx: idx_val, ret: ret.clone() });
                Ok(ret)
            }

            Expr::Cast { ty, expr } => {
                let val = self.lower_expr(expr)?;
                let from = RirType::I64; // approximate
                let to = lower_type_name(&ty.name);
                if from != to {
                    let ret = self.fresh_value();
                    self.emit(RirInstr::Convert { val, from, to, ret: ret.clone() });
                    Ok(ret)
                } else {
                    Ok(val)
                }
            }

            Expr::Instanceof { expr, ty } => {
                let val = self.lower_expr(expr)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::Instanceof {
                    obj: val, class: ClassId(encode_builtin(&ty.name)), ret: ret.clone(),
                });
                Ok(ret)
            }

            Expr::InstanceofPattern { expr, ty, name } => {
                let val = self.lower_expr(expr)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::Instanceof {
                    obj: val.clone(), class: ClassId(encode_builtin(&ty.name)), ret: ret.clone(),
                });
                // Bind the pattern variable
                self.vars.insert(name.clone(), val);
                Ok(ret)
            }

            Expr::Ternary { cond, then, else_ } => {
                let cond_val = self.lower_expr(cond)?;
                let pre = self.current;
                let then_bb  = self.new_block();
                let else_bb  = self.new_block();
                let merge_bb = self.new_block();
                let result = self.fresh_value();

                self.blocks[pre].instrs.push(RirInstr::Branch {
                    cond: cond_val, then_bb, else_bb,
                });

                self.switch_to(then_bb);
                let then_val = self.lower_expr(then)?;
                // Copy to result
                self.emit(RirInstr::ConstStr {
                    ret: result.clone(),
                    value: format!("__copy__{}", then_val.0),
                });
                self.emit(RirInstr::Jump(merge_bb));

                self.switch_to(else_bb);
                let else_val = self.lower_expr(else_)?;
                self.emit(RirInstr::ConstStr {
                    ret: result.clone(),
                    value: format!("__copy__{}", else_val.0),
                });
                self.emit(RirInstr::Jump(merge_bb));

                self.switch_to(merge_bb);
                Ok(result)
            }

            Expr::Lambda { params, body } => {
                // Generate a unique lambda function name
                let lambda_id = *self.lambda_counter;
                *self.lambda_counter += 1;
                let lambda_name = format!("__lambda_{}", lambda_id);

                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Collect captured variables (variables from enclosing scope used in lambda)
                let captures: Vec<String> = self.vars.keys()
                    .filter(|k| *k != "this")
                    .cloned()
                    .collect();

                // Register the lambda for later emission
                self.pending_lambdas.push(PendingLambda {
                    name: lambda_name.clone(),
                    params: param_names,
                    body: body.as_ref().clone(),
                    captures,
                });

                // Return the lambda function name as a reference
                let ret = self.fresh_value();
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: lambda_name });
                Ok(ret)
            }

            Expr::MethodRef { obj, name } => {
                // Lower method reference as a synthetic string for the interpreter
                let ret = self.fresh_value();
                let obj_str = expr_to_str(obj);
                self.emit(RirInstr::ConstStr {
                    ret: ret.clone(),
                    value: format!("__methodref__{}::{}", obj_str, name),
                });
                Ok(ret)
            }

            Expr::SwitchExpr { expr, cases } => {
                let switch_val = self.lower_expr(expr)?;
                let exit_bb = self.new_block();
                let result = self.fresh_value();
                let mut check = self.current;

                // Push yield target
                self.yield_stack.push((exit_bb, result.clone()));

                for case in cases {
                    match &case.labels {
                        Some(labels) => {
                            self.switch_to(self.blocks[check].id);
                            let mut or_val = None;
                            for label_expr in labels {
                                let label_val = self.lower_expr(label_expr)?;
                                let cmp = self.fresh_value();
                                self.emit(RirInstr::BinOp {
                                    op: RirBinOp::Eq, lhs: switch_val.clone(),
                                    rhs: label_val, ret: cmp.clone(),
                                });
                                or_val = Some(match or_val {
                                    None => cmp,
                                    Some(prev) => {
                                        let merged = self.fresh_value();
                                        self.emit(RirInstr::BinOp {
                                            op: RirBinOp::Or, lhs: prev, rhs: cmp, ret: merged.clone(),
                                        });
                                        merged
                                    }
                                });
                            }
                            let cond_ret = or_val.unwrap();
                            let body_bb = self.new_block();
                            let next_bb = self.new_block();
                            self.blocks[check].instrs.push(
                                RirInstr::Branch { cond: cond_ret, then_bb: body_bb, else_bb: next_bb }
                            );
                            self.switch_to(body_bb);
                            for stmt in &case.body { self.lower_stmt(stmt)?; }
                            if !self.current_block_ends_with_terminator() {
                                self.emit(RirInstr::Jump(exit_bb));
                            }
                            check = self.blocks.iter().position(|b| b.id == next_bb).unwrap();
                        }
                        None => {
                            // default case
                            self.switch_to(self.blocks[check].id);
                            for stmt in &case.body { self.lower_stmt(stmt)?; }
                            if !self.current_block_ends_with_terminator() {
                                self.emit(RirInstr::Jump(exit_bb));
                            }
                        }
                    }
                }

                self.yield_stack.pop();

                // If no default matched, jump to exit
                self.switch_to(self.blocks[check].id);
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(exit_bb));
                }
                self.switch_to(exit_bb);
                Ok(result)
            }
        }
    }

    /// Lower short-circuit && or ||.
    fn lower_short_circuit(&mut self, lhs: &Expr, rhs: &Expr, is_and: bool) -> Result<Value> {
        let l = self.lower_expr(lhs)?;
        let pre = self.current;
        let rhs_bb     = self.new_block();
        let merge_bb   = self.new_block();
        let short_bb   = self.new_block(); // short-circuit default value block
        let result = self.fresh_value();

        if is_and {
            // &&: if lhs is false, short-circuit to false; else evaluate rhs
            self.blocks[pre].instrs.push(RirInstr::Branch {
                cond: l.clone(), then_bb: rhs_bb, else_bb: short_bb,
            });
        } else {
            // ||: if lhs is true, short-circuit to true; else evaluate rhs
            self.blocks[pre].instrs.push(RirInstr::Branch {
                cond: l.clone(), then_bb: short_bb, else_bb: rhs_bb,
            });
        }

        // Short-circuit block: set default value and jump to merge
        self.switch_to(short_bb);
        let default_val = if is_and { 0 } else { 1 };
        self.emit(RirInstr::ConstInt { ret: result.clone(), value: default_val });
        self.emit(RirInstr::Jump(merge_bb));

        // RHS block: evaluate rhs, copy to result, jump to merge
        self.switch_to(rhs_bb);
        let r = self.lower_expr(rhs)?;
        self.emit(RirInstr::ConstStr {
            ret: result.clone(),
            value: format!("__copy__{}", r.0),
        });
        self.emit(RirInstr::Jump(merge_bb));

        self.switch_to(merge_bb);
        Ok(result)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_static_path(path: &str) -> bool {
    // Known JDK static paths
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
    // Convention: class names start with uppercase, variables with lowercase.
    // Only treat single-segment names as static (e.g., "Color", not "Color.RED")
    // Multi-segment paths like "Color.RED" are static field accesses, not static call targets
    if path.contains('.') {
        return false;
    }
    path.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
}

fn lower_type(ty: &TypeExpr) -> RirType {
    if ty.array_dims > 0 {
        return RirType::Array(Box::new(lower_type_name(&ty.name)));
    }
    lower_type_name(&ty.name)
}

fn lower_type_name(name: &str) -> RirType {
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

fn lower_binop(op: &BinOp) -> RirBinOp {
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

fn expr_to_str(expr: &Expr) -> String {
    match expr {
        Expr::Ident(s)           => s.clone(),
        Expr::This               => "this".into(),
        Expr::Super              => "super".into(),
        Expr::Field { obj, name } => format!("{}.{}", expr_to_str(obj), name),
        _                        => "<expr>".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::SourceFile, lexer::Lexer, parser::Parser};

    fn lower(src: &str) -> RirModule {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let file = Parser::new(tokens).parse_file().unwrap();
        Lowerer::new("test").lower_file(&file).unwrap()
    }

    #[test]
    fn lower_hello_world_produces_functions() {
        let src = r#"
            class Main {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "Main.main");
    }

    #[test]
    fn lower_arithmetic() {
        let src = r#"
            class Calc {
                int add(int a, int b) { return a + b; }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let instrs = &module.functions[0].basic_blocks[0].instrs;
        assert!(instrs.iter().any(|i| matches!(i, RirInstr::BinOp { .. })));
        assert!(instrs.iter().any(|i| matches!(i, RirInstr::Return(_))));
    }

    #[test]
    fn lower_do_while() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    do { i = i + 1; } while (i < 10);
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        // Should have multiple basic blocks for the do-while
        assert!(module.functions[0].basic_blocks.len() >= 3);
    }

    #[test]
    fn lower_break_continue() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    while (i < 10) {
                        if (i == 5) break;
                        i = i + 1;
                        continue;
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        // Should have Jump instructions for break/continue
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        let jump_count = all_instrs.iter().filter(|i| matches!(i, RirInstr::Jump(_))).count();
        assert!(jump_count >= 2, "expected at least 2 jumps for break/continue");
    }

    #[test]
    fn lower_ternary_branches() {
        let src = r#"
            class T {
                int f(int x) {
                    return x > 0 ? x : -x;
                }
            }
        "#;
        let module = lower(src);
        // Ternary should produce Branch instruction
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        assert!(all_instrs.iter().any(|i| matches!(i, RirInstr::Branch { .. })));
    }

    #[test]
    fn lower_for_each() {
        let src = r#"
            class T {
                void f(String[] items) {
                    for (String s : items) {
                        System.out.println(s);
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        // Should have ArrayLen and ArrayLoad for the for-each desugaring
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        assert!(all_instrs.iter().any(|i| matches!(i, RirInstr::ArrayLen { .. })));
        assert!(all_instrs.iter().any(|i| matches!(i, RirInstr::ArrayLoad { .. })));
    }
}