//! Expression lowering for the RIR lowerer.

use super::*;

impl<'a> FuncCtx<'a> {
    /// Lower `expr`, storing the result into `var_name` if provided.
    pub(super) fn lower_expr_into(&mut self, expr: &Expr, var_name: Option<&str>) -> Result<Value> {
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
                let ch = char::from_u32(*c as u32).unwrap_or('\0');
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: ch.to_string() });
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
                    self.emit(RirInstr::ConstStr {
                        ret: ret.clone(),
                        value: format!("__copy__{}", src.0),
                    });
                }
                Ok(ret)
            }
        }
    }

    pub(super) fn lower_expr(&mut self, expr: &Expr) -> Result<Value> {
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
                // Store chars as single-character strings so println displays 'A' not 65
                let ch = char::from_u32(*c as u32).unwrap_or('\0');
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: ch.to_string() });
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
                if let Some(v) = self.vars.get(name) {
                    return Ok(v.clone());
                }
                // Not a local var — check if it's an instance field (this.name)
                if self.vars.contains_key("this") {
                    let ret = self.fresh_value();
                    self.emit(RirInstr::GetField {
                        obj: Value("this".into()),
                        field: FieldId(encode_builtin(name)),
                        ret: ret.clone(),
                    });
                    return Ok(ret);
                }
                // Static field in the current class
                if !self.class_name.is_empty() {
                    let key = format!("{}.{}", self.class_name, name);
                    let ret = self.fresh_value();
                    self.emit(RirInstr::GetStatic {
                        field: FieldId(encode_builtin(&key)),
                        ret: ret.clone(),
                    });
                    return Ok(ret);
                }
                Ok(Value(name.clone()))
            }
            Expr::This  => Ok(Value("this".into())),
            Expr::Super => Ok(Value("super".into())),

            Expr::Field { obj, name } => {
                let ret = self.fresh_value();
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

                    // super.method(...) — non-virtual dispatch to parent class method
                    if matches!(obj.as_ref(), Expr::Super) {
                        let arg_vals: Vec<Value> = args.iter()
                            .map(|a| self.lower_expr(a))
                            .collect::<Result<_>>()?;
                        let mut super_args = vec![Value("this".into())];
                        super_args.extend(arg_vals);
                        self.emit(RirInstr::Call {
                            func: FuncId(encode_builtin(&format!("super.{}", method_name))),
                            args: super_args,
                            ret: Some(ret.clone()),
                        });
                        return Ok(ret);
                    }

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

                if matches!(callee.as_ref(), Expr::Super) {
                    let mut arg_vals = vec![Value("this".into())];
                    for a in args { arg_vals.push(self.lower_expr(a)?); }
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin("super.<init>")),
                        args: arg_vals,
                        ret:  None,
                    });
                    return Ok(Value("this".into()));
                }

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

                let arg_vals: Vec<Value> = args.iter()
                    .map(|a| self.lower_expr(a))
                    .collect::<Result<_>>()?;
                if let Expr::Ident(name) = callee.as_ref() {
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
                // super.method(...) — non-virtual dispatch to parent class method
                if let Expr::Field { obj, name: method_name } = callee.as_ref() {
                    if matches!(obj.as_ref(), Expr::Super) {
                        let mut super_args = vec![Value("this".into())];
                        super_args.extend(arg_vals);
                        self.emit(RirInstr::Call {
                            func: FuncId(encode_builtin(&format!("super.{}", method_name))),
                            args: super_args,
                            ret: Some(ret.clone()),
                        });
                        return Ok(ret);
                    }
                }
                // Extract method name from `obj.method(...)` callee for virtual dispatch
                let method_hash = if let Expr::Field { name: method_name, .. } = callee.as_ref() {
                    encode_builtin(method_name)
                } else {
                    0
                };
                self.emit(RirInstr::CallVirtual {
                    receiver,
                    method: MethodId(method_hash),
                    args: arg_vals,
                    ret: Some(ret.clone()),
                });
                Ok(ret)
            }

            Expr::BinOp { op, lhs, rhs } => {
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
                        let neg1 = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: neg1.clone(), value: -1 });
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Xor, lhs: val, rhs: neg1, ret: ret.clone(),
                        });
                    }
                    UnaryOp::PostInc | UnaryOp::PreInc => {
                        let one = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: one.clone(), value: 1 });
                        let new_val = self.fresh_value();
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Add, lhs: val, rhs: one, ret: new_val.clone(),
                        });
                        self.write_back(expr, new_val.clone());
                        return Ok(new_val);
                    }
                    UnaryOp::PostDec | UnaryOp::PreDec => {
                        let one = self.fresh_value();
                        self.emit(RirInstr::ConstInt { ret: one.clone(), value: 1 });
                        let new_val = self.fresh_value();
                        self.emit(RirInstr::BinOp {
                            op: RirBinOp::Sub, lhs: val, rhs: one, ret: new_val.clone(),
                        });
                        self.write_back(expr, new_val.clone());
                        return Ok(new_val);
                    }
                }
                Ok(ret)
            }

            Expr::Assign { lhs, rhs } => {
                if let Expr::Index { arr, idx } = lhs.as_ref() {
                    let arr_val = self.lower_expr(arr)?;
                    let idx_val = self.lower_expr(idx)?;
                    let rhs_val = self.lower_expr(rhs)?;
                    self.emit(RirInstr::ArrayStore { arr: arr_val, idx: idx_val, val: rhs_val.clone() });
                    return Ok(rhs_val);
                }
                if let Expr::Field { obj, name } = lhs.as_ref() {
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
                let val = self.lower_expr(rhs)?;
                if let Expr::Ident(name) = lhs.as_ref() {
                    if self.vars.contains_key(name.as_str()) {
                        // Known local variable — update mapping; emit copy if SSA names differ
                        let old = self.vars[name.as_str()].clone();
                        self.vars.insert(name.clone(), val.clone());
                        if old != val {
                            self.emit(RirInstr::ConstStr {
                                ret: old,
                                value: format!("__copy__{}", val.0),
                            });
                        }
                    } else if self.vars.contains_key("this") {
                        // Instance field assignment: this.name = val
                        self.emit(RirInstr::SetField {
                            obj: Value("this".into()),
                            field: FieldId(encode_builtin(name)),
                            val: val.clone(),
                        });
                    } else if !self.class_name.is_empty() {
                        // Static field assignment in current class
                        let key = format!("{}.{}", self.class_name, name);
                        self.emit(RirInstr::SetStatic {
                            field: FieldId(encode_builtin(&key)),
                            val: val.clone(),
                        });
                    } else {
                        self.vars.insert(name.clone(), val.clone());
                    }
                }
                Ok(val)
            }

            Expr::CompoundAssign { op, lhs, rhs } => {
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
                let mut ctor_args = vec![ret.clone()];
                ctor_args.extend(arg_vals);
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin(&format!("{}.<init>", class_name))),
                    args: ctor_args,
                    ret:  None,
                });
                // For anonymous classes, store captured local variables as __cap__xxx fields
                if class_name.starts_with("__anon_") {
                    let captures: Vec<(String, Value)> = self.vars.iter()
                        .filter(|(k, _)| k.as_str() != "this")
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (var_name, var_val) in captures {
                        let field_name = format!("__cap__{}", var_name);
                        let field_id = FieldId(encode_builtin(&format!("{}.{}", class_name, field_name)));
                        self.emit(RirInstr::SetField {
                            obj: ret.clone(),
                            field: field_id,
                            val: var_val,
                        });
                    }
                }
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
                let from = RirType::I64;
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
                self.vars.insert(name.clone(), val);
                Ok(ret)
            }

            Expr::RecordPattern { expr, ty, components } => {
                let val = self.lower_expr(expr)?;
                let ret = self.fresh_value();
                self.emit(RirInstr::Instanceof {
                    obj: val.clone(), class: ClassId(encode_builtin(&ty.name)), ret: ret.clone(),
                });
                for (_, comp_name) in components {
                    let comp_ret = self.fresh_value();
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin(&format!("__method__{}", comp_name))),
                        args: vec![val.clone()],
                        ret: Some(comp_ret.clone()),
                    });
                    self.vars.insert(comp_name.clone(), comp_ret);
                }
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
                let lambda_id = *self.lambda_counter;
                *self.lambda_counter += 1;
                let lambda_name = format!("__lambda_{}", lambda_id);

                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                let captures: Vec<String> = self.vars.keys()
                    .filter(|k| *k != "this")
                    .cloned()
                    .collect();

                self.pending_lambdas.push(PendingLambda {
                    name: lambda_name.clone(),
                    params: param_names,
                    body: body.as_ref().clone(),
                    captures,
                });

                let ret = self.fresh_value();
                self.emit(RirInstr::ConstStr { ret: ret.clone(), value: lambda_name });
                Ok(ret)
            }

            Expr::MethodRef { obj, name } => {
                let ret = self.fresh_value();
                let obj_str = expr_to_str(obj);
                // If obj is a variable (lowercase start, not a known class), emit a
                // bound method ref that captures the receiver at runtime.
                let is_var = obj_str.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
                    && !matches!(obj_str.as_str(), "int" | "long" | "double" | "float" | "boolean");
                if is_var {
                    // Emit: __bound_methodref__<varname>::<method>
                    // The interpreter will look up the variable value at call time.
                    let sentinel = format!("__bound_methodref__{}::{}", obj_str, name);
                    self.emit(RirInstr::ConstStr { ret: ret.clone(), value: sentinel });
                    // Also emit a capture instruction so the interpreter can find the receiver.
                    // We encode this as a lambda capture via a special Call.
                    let recv_val = Value(obj_str.clone());
                    self.emit(RirInstr::Call {
                        func: FuncId(encode_builtin("__capture_bound_methodref__")),
                        args: vec![ret.clone(), recv_val],
                        ret: None,
                    });
                } else {
                    self.emit(RirInstr::ConstStr {
                        ret: ret.clone(),
                        value: format!("__methodref__{}::{}", obj_str, name),
                    });
                }
                Ok(ret)
            }

            Expr::SwitchExpr { expr, cases } => {
                let switch_val = self.lower_expr(expr)?;
                let exit_bb = self.new_block();
                let result = self.fresh_value();
                let mut check = self.current;

                self.yield_stack.push((exit_bb, result.clone()));

                for case in cases {
                    match &case.labels {
                        Some(labels) => {
                            self.switch_to(self.blocks[check].id);
                            let mut or_val = None;
                            let mut pattern_bindings: Vec<(String, Value)> = Vec::new();
                            for label_expr in labels {
                                let cmp = self.lower_switch_label(label_expr, &switch_val, &mut pattern_bindings)?;
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
                            for (name, val) in pattern_bindings {
                                self.vars.insert(name, val);
                            }
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

                self.yield_stack.pop();

                self.switch_to(self.blocks[check].id);
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(exit_bb));
                }
                self.switch_to(exit_bb);
                Ok(result)
            }
        }
    }

    pub(super) fn lower_short_circuit(&mut self, lhs: &Expr, rhs: &Expr, is_and: bool) -> Result<Value> {
        let l = self.lower_expr(lhs)?;
        let pre = self.current;
        let rhs_bb   = self.new_block();
        let merge_bb = self.new_block();
        let short_bb = self.new_block();
        let result = self.fresh_value();

        if is_and {
            self.blocks[pre].instrs.push(RirInstr::Branch {
                cond: l.clone(), then_bb: rhs_bb, else_bb: short_bb,
            });
        } else {
            self.blocks[pre].instrs.push(RirInstr::Branch {
                cond: l.clone(), then_bb: short_bb, else_bb: rhs_bb,
            });
        }

        self.switch_to(short_bb);
        let default_val = if is_and { 0 } else { 1 };
        self.emit(RirInstr::ConstInt { ret: result.clone(), value: default_val });
        self.emit(RirInstr::Jump(merge_bb));

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
