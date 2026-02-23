//! Statement lowering for the RIR lowerer.

use super::*;

impl<'a> FuncCtx<'a> {
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

                self.lower_block(try_body)?;

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
                self.emit(RirInstr::Throw(msg));
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
