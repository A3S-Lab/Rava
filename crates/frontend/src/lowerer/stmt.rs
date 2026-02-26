//! Statement lowering for the RIR lowerer.

use super::*;

impl<'a> FuncCtx<'a> {
    /// Lower a single switch case label against `switch_val`, returning a boolean Value.
    /// For type patterns and guarded patterns, also populates `bindings` with the bound name.
    pub(super) fn lower_switch_label(
        &mut self,
        label: &Expr,
        switch_val: &Value,
        bindings: &mut Vec<(String, Value)>,
    ) -> Result<Value> {
        // case null — emit null-equality check
        if matches!(label, Expr::Null) {
            let null_val = self.fresh_value();
            self.emit(RirInstr::ConstNull { ret: null_val.clone() });
            let cmp = self.fresh_value();
            self.emit(RirInstr::BinOp {
                op: RirBinOp::Eq, lhs: switch_val.clone(), rhs: null_val, ret: cmp.clone(),
            });
            return Ok(cmp);
        }
        // Plain type pattern: __type_pattern__Type#name
        if let Expr::Ident(s) = label {
            if let Some(rest) = s.strip_prefix("__type_pattern__") {
                if let Some((type_name, bind_name)) = rest.split_once('#') {
                    let cmp = self.fresh_value();
                    self.emit(RirInstr::Instanceof {
                        obj: switch_val.clone(),
                        class: ClassId(encode_builtin(type_name)),
                        ret: cmp.clone(),
                    });
                    bindings.push((bind_name.to_string(), switch_val.clone()));
                    return Ok(cmp);
                }
            }
        }
        // Guarded pattern: Ternary { cond: InstanceofPattern { .. }, then: guard, else_: false }
        if let Expr::Ternary { cond, then: guard, .. } = label {
            if let Expr::InstanceofPattern { ty, name, .. } = cond.as_ref() {
                let instanceof = self.fresh_value();
                self.emit(RirInstr::Instanceof {
                    obj: switch_val.clone(),
                    class: ClassId(encode_builtin(&ty.name)),
                    ret: instanceof.clone(),
                });
                bindings.push((name.clone(), switch_val.clone()));
                // Short-circuit: only eval guard if instanceof passes
                let pre = self.current;
                let guard_bb  = self.new_block();
                let merge_bb  = self.new_block();
                let false_bb  = self.new_block();
                let result    = self.fresh_value();
                self.blocks[pre].instrs.push(RirInstr::Branch {
                    cond: instanceof, then_bb: guard_bb, else_bb: false_bb,
                });
                self.switch_to(guard_bb);
                // Temporarily bind the pattern variable so the guard can reference it
                self.vars.insert(name.clone(), switch_val.clone());
                let guard_val = self.lower_expr(guard)?;
                self.emit(RirInstr::ConstStr {
                    ret: result.clone(),
                    value: format!("__copy__{}", guard_val.0),
                });
                self.emit(RirInstr::Jump(merge_bb));
                self.switch_to(false_bb);
                self.emit(RirInstr::ConstBool { ret: result.clone(), value: false });
                self.emit(RirInstr::Jump(merge_bb));
                self.switch_to(merge_bb);
                return Ok(result);
            }
        }
        // Bare identifier that isn't a local variable — treat as enum constant string
        // (enum values are stored as RVal::Str("CONSTANT_NAME") at runtime)
        if let Expr::Ident(name) = label {
            if !self.vars.contains_key(name.as_str()) {
                let label_val = self.fresh_value();
                self.emit(RirInstr::ConstStr { ret: label_val.clone(), value: name.clone() });
                let cmp = self.fresh_value();
                self.emit(RirInstr::BinOp {
                    op: RirBinOp::Eq, lhs: switch_val.clone(), rhs: label_val, ret: cmp.clone(),
                });
                return Ok(cmp);
            }
        }
        // Default: plain equality check
        let label_val = self.lower_expr(label)?;
        let cmp = self.fresh_value();
        self.emit(RirInstr::BinOp {
            op: RirBinOp::Eq, lhs: switch_val.clone(), rhs: label_val, ret: cmp.clone(),
        });
        Ok(cmp)
    }

    pub(super) fn lower_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.0 {
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    pub(super) fn lower_stmt(&mut self, stmt: &Stmt) -> Result<()> {
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
                // Snapshot vars before lowering body
                let pre_body_vars = self.vars.clone();
                self.switch_to(body_bb);
                self.lower_stmt(body)?;
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(header_bb));
                }
                self.loop_stack.pop();
                // In the exit block, emit phi-like copies for variables modified in the body.
                // post_val is the fresh SSA name used after the loop.
                // pre_val is the named slot kept current by __copy__ when the loop runs.
                // If the loop didn't run, post_val was never set — copy from pre_val.
                // If the loop ran, post_val was set by the body — the copy is a no-op
                // because post_val is already in env (the __copy__ in the body set pre_val,
                // but post_val itself was set by the BinOp/etc that produced it).
                let post_body_vars = self.vars.clone();
                self.switch_to(exit_bb);
                for (name, post_val) in &post_body_vars {
                    if let Some(pre_val) = pre_body_vars.get(name) {
                        if pre_val != post_val {
                            // Phi copy: post_val = pre_val (named slot).
                            // When loop ran: pre_val was updated by __copy__ to final value.
                            // When loop didn't run: pre_val has the pre-loop value.
                            // Either way, post_val gets the correct current value.
                            self.emit(RirInstr::ConstStr {
                                ret: post_val.clone(),
                                value: format!("__copy__{}", pre_val.0),
                            });
                        }
                    }
                }
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
                self.loop_stack.push((exit_bb, update_bb));
                self.register_pending_label(exit_bb, update_bb);
                // Snapshot vars before body so post-loop code uses named slots
                let pre_body_vars = self.vars.clone();
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
                // Restore vars to pre-body snapshot
                self.vars = pre_body_vars;
            }
            Stmt::ForEach { ty: _, name, iterable, body } => {
                let collection = self.lower_expr(iterable)?;

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

                self.switch_to(header_bb);
                let has_next = self.fresh_value();
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin("__method__hasNext")),
                    args: vec![iter_var.clone()],
                    ret:  Some(has_next.clone()),
                });
                self.emit(RirInstr::Branch { cond: has_next, then_bb: body_bb, else_bb: exit_bb });

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
                if self.pending_label.take().is_some() {
                    self.label_map.remove(label);
                }
            }
            Stmt::Throw(e) => {
                let val = self.lower_expr(e)?;
                self.emit(RirInstr::Throw(val));
            }
            Stmt::TryCatch { try_body, catches, finally_body } => {
                let finally_bb = if finally_body.is_some() { Some(self.new_block()) } else { None };
                let exit_bb = self.new_block();

                let mut catch_blocks: Vec<(BlockId, Vec<String>)> = Vec::new();
                for catch in catches {
                    let cbb = self.new_block();
                    let types: Vec<String> = catch.exception_types.iter()
                        .map(|t| t.name.clone())
                        .collect();
                    catch_blocks.push((cbb, types));
                }

                for (cbb, types) in catch_blocks.iter().rev() {
                    let type_list = types.join("|");
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: format!("__try_catch__{}:{}", cbb.0, type_list),
                    });
                }
                // Register finally block as a catch-all that runs before re-throwing
                if let Some(fbb) = finally_bb {
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: format!("__try_finally__{}:{}", fbb.0, exit_bb.0),
                    });
                }

                self.lower_block(try_body)?;

                for _ in &catch_blocks {
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: "__try_end__".into(),
                    });
                }
                if finally_bb.is_some() {
                    let marker = self.fresh_value();
                    self.emit(RirInstr::ConstStr {
                        ret: marker,
                        value: "__try_finally_end__".into(),
                    });
                }
                if !self.current_block_ends_with_terminator() {
                    self.emit(RirInstr::Jump(finally_bb.unwrap_or(exit_bb)));
                }

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

                self.loop_stack.push((exit_bb, exit_bb));

                // Pre-allocate body blocks so fall-through can reference the next one
                let body_bbs: Vec<BlockId> = cases.iter().map(|_| self.new_block()).collect();

                for (case_idx, case) in cases.iter().enumerate() {
                    let body_bb = body_bbs[case_idx];
                    let next_body_bb = body_bbs.get(case_idx + 1).copied().unwrap_or(exit_bb);

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
                                // Fall through to next case body (colon-syntax) or exit (arrow-syntax)
                                self.emit(RirInstr::Jump(next_body_bb));
                            }
                            check = self.blocks.iter().position(|b| b.id == next_bb).unwrap();
                        }
                        None => {
                            self.switch_to(self.blocks[check].id);
                            self.emit(RirInstr::Jump(body_bb));
                            self.switch_to(body_bb);
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
                self.lower_block(body)?;
            }
            Stmt::Assert { expr, message } => {
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
                // Throw as AssertionError object so catch(AssertionError e) works
                let err_obj = self.fresh_value();
                self.emit(RirInstr::New {
                    class: ClassId(encode_builtin("AssertionError")),
                    ret: err_obj.clone(),
                });
                self.emit(RirInstr::Call {
                    func: FuncId(encode_builtin("AssertionError.<init>")),
                    args: vec![err_obj.clone(), msg],
                    ret: None,
                });
                self.emit(RirInstr::Throw(err_obj));
                self.switch_to(cont_bb);
            }
            Stmt::Yield(expr) => {
                let val = self.lower_expr(expr)?;
                if let Some(&(exit_bb, ref result)) = self.yield_stack.last() {
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
}
