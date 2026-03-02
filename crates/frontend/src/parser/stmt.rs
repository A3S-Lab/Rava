use super::Parser;
use crate::ast::*;
use crate::lexer::Token;
use rava_common::error::Result;

impl Parser {
    pub(crate) fn parse_block(&mut self) -> Result<Block> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Block(stmts))
    }

    pub(crate) fn parse_stmt(&mut self) -> Result<Stmt> {
        // Check for labeled statement: `ident: stmt`
        if matches!(self.peek(), Token::Ident(_)) && self.peek2() == &Token::Colon {
            let label = self.expect_ident()?;
            self.advance(); // consume ':'
            let stmt = self.parse_stmt()?;
            return Ok(Stmt::Labeled {
                label,
                stmt: Box::new(stmt),
            });
        }

        match self.peek().clone() {
            Token::Semi => {
                self.advance();
                Ok(Stmt::Empty)
            }
            Token::LBrace => Ok(Stmt::Block(self.parse_block()?)),
            Token::Return => {
                self.advance();
                let expr = if self.peek() == &Token::Semi {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Return(expr))
            }
            Token::If => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let then = Box::new(self.parse_stmt()?);
                let else_ = if self.eat(&Token::Else) {
                    Some(Box::new(self.parse_stmt()?))
                } else {
                    None
                };
                Ok(Stmt::If { cond, then, else_ })
            }
            Token::While => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Ok(Stmt::While { cond, body })
            }
            Token::Do => {
                self.advance();
                let body = Box::new(self.parse_stmt()?);
                self.expect(&Token::While)?;
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::DoWhile { body, cond })
            }
            Token::For => {
                self.advance();
                self.expect(&Token::LParen)?;
                // Detect for-each: `for (Type name : iterable)`
                if self.is_for_each() {
                    let ty = self.parse_type_expr()?;
                    let name = self.expect_ident()?;
                    self.expect(&Token::Colon)?;
                    let iterable = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    let body = Box::new(self.parse_stmt()?);
                    return Ok(Stmt::ForEach {
                        ty,
                        name,
                        iterable,
                        body,
                    });
                }
                // Regular for
                let init = if self.peek() == &Token::Semi {
                    self.advance();
                    None
                } else {
                    Some(Box::new(self.parse_stmt()?))
                };
                let cond = if self.peek() == &Token::Semi {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(&Token::Semi)?;
                let mut update = Vec::new();
                while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                    update.push(self.parse_expr()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Ok(Stmt::For {
                    init,
                    cond,
                    update,
                    body,
                })
            }
            Token::Break => {
                self.advance();
                let label = if matches!(self.peek(), Token::Ident(_)) {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Break(label))
            }
            Token::Continue => {
                self.advance();
                let label = if matches!(self.peek(), Token::Ident(_)) {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Continue(label))
            }
            Token::Throw => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Throw(expr))
            }
            Token::Try => {
                self.advance();
                // Try-with-resources: try (Type name = expr; ...) { ... }
                let mut resources: Vec<(String, Expr)> = Vec::new();
                if self.peek() == &Token::LParen {
                    let saved = self.pos;
                    self.advance(); // consume (
                    if self.peek() != &Token::RParen
                        && !matches!(self.peek(), Token::Catch | Token::LBrace)
                    {
                        let is_resource = matches!(self.peek(), Token::Ident(_) | Token::Final);
                        if is_resource {
                            self.pos = saved;
                            self.advance(); // consume (
                            while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                                self.eat(&Token::Final);
                                let _ty = self.parse_type_expr()?;
                                let name = self.expect_ident()?;
                                self.expect(&Token::Assign)?;
                                let init = self.parse_expr()?;
                                resources.push((name, init));
                                self.eat(&Token::Semi);
                            }
                            self.expect(&Token::RParen)?;
                        } else {
                            self.pos = saved;
                        }
                    } else {
                        self.pos = saved;
                    }
                }

                let try_body = self.parse_block()?;
                let mut catches = Vec::new();
                let mut finally_body = None;
                while self.peek() == &Token::Catch {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let mut exception_types = vec![self.parse_type_expr()?];
                    while self.eat(&Token::BitOr) {
                        exception_types.push(self.parse_type_expr()?);
                    }
                    let name = self.expect_ident()?;
                    self.expect(&Token::RParen)?;
                    let body = self.parse_block()?;
                    catches.push(CatchClause {
                        exception_types,
                        name,
                        body,
                    });
                }
                if self.eat(&Token::Finally) {
                    finally_body = Some(self.parse_block()?);
                }

                // Desugar try-with-resources
                if !resources.is_empty() {
                    let mut full_body = Vec::new();
                    for (name, init) in &resources {
                        full_body.push(Stmt::LocalVar {
                            ty: TypeExpr::simple("var"),
                            name: name.clone(),
                            init: Some(init.clone()),
                        });
                    }
                    full_body.extend(try_body.0);

                    let mut close_stmts: Vec<Stmt> = resources
                        .iter()
                        .rev()
                        .map(|(name, _)| {
                            Stmt::Expr(Expr::Call {
                                callee: Box::new(Expr::Field {
                                    obj: Box::new(Expr::Ident(name.clone())),
                                    name: "close".into(),
                                }),
                                args: vec![],
                                type_args_raw: None,
                            })
                        })
                        .collect();

                    if let Some(existing_finally) = finally_body {
                        close_stmts.extend(existing_finally.0);
                    }

                    Ok(Stmt::TryCatch {
                        try_body: Block(full_body),
                        catches,
                        finally_body: Some(Block(close_stmts)),
                    })
                } else {
                    Ok(Stmt::TryCatch {
                        try_body,
                        catches,
                        finally_body,
                    })
                }
            }
            Token::Switch => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::LBrace)?;
                let mut cases = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    let label = if self.peek() == &Token::Case {
                        self.advance();
                        let mut labels = vec![self.parse_case_label()?];
                        while self.eat(&Token::Comma) {
                            labels.push(self.parse_case_label()?);
                        }
                        Some(labels)
                    } else if self.peek() == &Token::Default {
                        self.advance();
                        None
                    } else {
                        break;
                    };

                    // Arrow syntax: case X -> expr; or case X -> { ... }
                    if self.peek() == &Token::Arrow {
                        self.advance();
                        let mut body = Vec::new();
                        if self.peek() == &Token::LBrace {
                            body = self.parse_block()?.0;
                        } else {
                            body.push(self.parse_stmt()?);
                        }
                        // Arrow cases have implicit break
                        body.push(Stmt::Break(None));
                        cases.push(SwitchCase {
                            labels: label,
                            body,
                        });
                    } else {
                        // Colon syntax: case X: ...
                        self.expect(&Token::Colon)?;
                        let mut body = Vec::new();
                        loop {
                            match self.peek() {
                                Token::Case | Token::Default | Token::RBrace | Token::Eof => break,
                                Token::Break => {
                                    self.advance();
                                    self.eat(&Token::Semi);
                                    body.push(Stmt::Break(None));
                                    break;
                                }
                                _ => body.push(self.parse_stmt()?),
                            }
                        }
                        cases.push(SwitchCase {
                            labels: label,
                            body,
                        });
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Switch { expr, cases })
            }
            Token::Synchronized => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let body = self.parse_block()?;
                Ok(Stmt::Synchronized { expr, body })
            }
            Token::Assert => {
                self.advance();
                let expr = self.parse_expr()?;
                let message = if self.eat(&Token::Colon) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Assert { expr, message })
            }
            Token::Yield => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Yield(expr))
            }
            Token::Var => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::Assign)?;
                let init = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::LocalVar {
                    ty: TypeExpr::simple("var"),
                    name,
                    init: Some(init),
                })
            }
            // `final Type name = ...` — treat final as a no-op modifier
            Token::Final => {
                self.advance(); // consume `final`
                let ty = self.parse_type_expr()?;
                let name = self.expect_ident()?;
                let init = if self.eat(&Token::Assign) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(&Token::Semi)?;
                Ok(Stmt::LocalVar { ty, name, init })
            }
            // local variable declaration (skip optional `final` modifier)
            tok if self.is_type_start(&tok)
                && self.is_local_var_decl()
                && !self.is_explicit_type_arg_call_start() =>
            {
                self.eat(&Token::Final);
                let ty = self.parse_type_expr()?;
                let name = self.expect_ident()?;
                let init = if self.eat(&Token::Assign) {
                    if self.peek() == &Token::LBrace {
                        let elements = self.parse_array_init()?;
                        Some(Expr::ArrayInit { ty: None, elements })
                    } else {
                        Some(self.parse_expr()?)
                    }
                } else {
                    None
                };
                // multi-var decl: int a, b, c; or int a = 1, b = 2;
                if self.peek() == &Token::Comma {
                    let mut stmts = vec![Stmt::LocalVar {
                        ty: ty.clone(),
                        name,
                        init,
                    }];
                    while self.eat(&Token::Comma) {
                        let extra_name = self.expect_ident()?;
                        let extra_init = if self.eat(&Token::Assign) {
                            Some(self.parse_expr()?)
                        } else {
                            None
                        };
                        stmts.push(Stmt::LocalVar {
                            ty: ty.clone(),
                            name: extra_name,
                            init: extra_init,
                        });
                    }
                    self.expect(&Token::Semi)?;
                    return Ok(Stmt::Block(crate::ast::Block(stmts)));
                }
                self.expect(&Token::Semi)?;
                Ok(Stmt::LocalVar { ty, name, init })
            }
            // this(...) constructor delegation
            Token::This if matches!(self.peek_at(1), Token::LParen) => {
                self.advance(); // consume 'this'
                let args = self.parse_args()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::This),
                    args,
                    type_args_raw: None,
                }))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    pub(crate) fn is_type_start(&self, tok: &Token) -> bool {
        matches!(
            tok,
            Token::Int
                | Token::Long
                | Token::Double
                | Token::Float
                | Token::Boolean
                | Token::Byte
                | Token::Short
                | Token::Char
                | Token::Ident(_)
        )
    }

    /// Heuristic: current token is a type, next is an identifier -> local var decl.
    pub(crate) fn is_local_var_decl(&self) -> bool {
        let mut i = self.pos;
        match self.tokens.get(i) {
            Some(
                Token::Ident(_)
                | Token::Int
                | Token::Long
                | Token::Double
                | Token::Float
                | Token::Boolean
                | Token::Byte
                | Token::Short
                | Token::Char,
            ) => {
                i += 1;
            }
            _ => return false,
        }
        // skip qualified name dots
        while matches!(self.tokens.get(i), Some(Token::Dot)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::Ident(_))) {
                i += 1;
            } else {
                break;
            }
        }
        // skip generic params
        if matches!(self.tokens.get(i), Some(Token::Lt)) {
            let mut depth = 0i32;
            loop {
                match self.tokens.get(i) {
                    Some(Token::Lt) => {
                        depth += 1;
                        i += 1;
                    }
                    Some(Token::Gt) => {
                        depth -= 1;
                        i += 1;
                        if depth <= 0 {
                            break;
                        }
                    }
                    Some(Token::Shr) => {
                        depth -= 2;
                        i += 1;
                        if depth <= 0 {
                            break;
                        }
                    }
                    None | Some(Token::Eof) => break,
                    _ => {
                        i += 1;
                    }
                }
            }
        }
        // skip array dims []
        while matches!(self.tokens.get(i), Some(Token::LBracket))
            && matches!(self.tokens.get(i + 1), Some(Token::RBracket))
        {
            i += 2;
        }
        matches!(self.tokens.get(i), Some(Token::Ident(_)))
    }

    /// Detect for-each: `for (Type name : iterable)`.
    /// We're right after `(`. Look ahead for `Type Ident :`.
    pub(crate) fn is_for_each(&self) -> bool {
        let mut i = self.pos;
        // skip `final`
        if matches!(self.tokens.get(i), Some(Token::Final)) {
            i += 1;
        }
        // skip type token
        match self.tokens.get(i) {
            Some(
                Token::Ident(_)
                | Token::Int
                | Token::Long
                | Token::Double
                | Token::Float
                | Token::Boolean
                | Token::Byte
                | Token::Short
                | Token::Char
                | Token::Var,
            ) => {
                i += 1;
            }
            _ => return false,
        }
        // skip qualified name
        while matches!(self.tokens.get(i), Some(Token::Dot)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::Ident(_))) {
                i += 1;
            } else {
                break;
            }
        }
        // skip generics
        if matches!(self.tokens.get(i), Some(Token::Lt)) {
            let mut depth = 0i32;
            loop {
                match self.tokens.get(i) {
                    Some(Token::Lt) => {
                        depth += 1;
                        i += 1;
                    }
                    Some(Token::Gt) => {
                        depth -= 1;
                        i += 1;
                        if depth <= 0 {
                            break;
                        }
                    }
                    None | Some(Token::Eof) => break,
                    _ => {
                        i += 1;
                    }
                }
            }
        }
        // skip array dims
        while matches!(self.tokens.get(i), Some(Token::LBracket))
            && matches!(self.tokens.get(i + 1), Some(Token::RBracket))
        {
            i += 2;
        }
        // expect Ident then Colon
        if !matches!(self.tokens.get(i), Some(Token::Ident(_))) {
            return false;
        }
        i += 1;
        matches!(self.tokens.get(i), Some(Token::Colon))
    }

    /// Detect invocation start like `Util.<String>id(...)`.
    fn is_explicit_type_arg_call_start(&self) -> bool {
        matches!(self.peek(), Token::Ident(_))
            && self.peek2() == &Token::Dot
            && self.peek_at(2) == &Token::Lt
    }

    /// Parse a case label expression — restricted to avoid lambda ambiguity with `->`.
    pub(crate) fn parse_case_label(&mut self) -> Result<Expr> {
        // case null (Java 21+)
        if self.peek() == &Token::Null {
            self.advance();
            return Ok(Expr::Null);
        }
        // Type pattern or guarded pattern: case Type name [when guard]
        if self.is_type_pattern_case() {
            let ty = self.parse_type_expr()?;
            let name = self.expect_ident()?;
            // Guarded pattern: case Type name when guard
            if matches!(self.peek(), Token::Ident(w) if w == "when") {
                self.advance(); // consume 'when'
                let guard = self.parse_expr()?;
                // Encode as InstanceofPattern with guard stored in a Ternary wrapper:
                // the guard is preserved as the condition of a ternary so the lowerer can extract it.
                return Ok(Expr::Ternary {
                    cond: Box::new(Expr::InstanceofPattern {
                        expr: Box::new(Expr::Ident("__switch_val__".into())),
                        ty,
                        name,
                    }),
                    then: Box::new(guard),
                    else_: Box::new(Expr::BoolLit(false)),
                });
            }
            // Plain type pattern in switch: case Type name ->
            return Ok(Expr::Ident(format!("__type_pattern__{}#{}", ty.name, name)));
        }
        let mut expr = match self.peek().clone() {
            Token::IntLit(n) => {
                self.advance();
                Expr::IntLit(n)
            }
            Token::FloatLit(n) => {
                self.advance();
                Expr::FloatLit(n)
            }
            Token::StrLit(s) => {
                self.advance();
                Expr::StrLit(s)
            }
            Token::CharLit(c) => {
                self.advance();
                Expr::CharLit(c)
            }
            Token::BoolLit(b) => {
                self.advance();
                Expr::BoolLit(b)
            }
            Token::Minus => {
                self.advance();
                let inner = self.parse_case_label()?;
                Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(inner),
                }
            }
            Token::Ident(name) => {
                self.advance();
                let mut e = Expr::Ident(name);
                // Allow dotted access: Color.RED
                while self.peek() == &Token::Dot {
                    self.advance();
                    let field = self.expect_ident()?;
                    e = Expr::Field {
                        obj: Box::new(e),
                        name: field,
                    };
                }
                e
            }
            _ => return self.parse_expr(), // fallback
        };
        let _ = &mut expr;
        Ok(expr)
    }

    /// Detect type pattern in case: `Type name ->` or `Type name when`
    pub(crate) fn is_type_pattern_case(&self) -> bool {
        let mut i = self.pos;
        // Must start with a type token
        match self.tokens.get(i) {
            Some(
                Token::Ident(_)
                | Token::Int
                | Token::Long
                | Token::Double
                | Token::Float
                | Token::Boolean
                | Token::Byte
                | Token::Short
                | Token::Char,
            ) => {
                i += 1;
            }
            _ => return false,
        }
        // Skip qualified name and generics
        while matches!(self.tokens.get(i), Some(Token::Dot)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::Ident(_))) {
                i += 1;
            } else {
                return false;
            }
        }
        if matches!(self.tokens.get(i), Some(Token::Lt)) {
            let mut depth = 0i32;
            loop {
                match self.tokens.get(i) {
                    Some(Token::Lt) => {
                        depth += 1;
                        i += 1;
                    }
                    Some(Token::Gt) => {
                        depth -= 1;
                        i += 1;
                        if depth <= 0 {
                            break;
                        }
                    }
                    None | Some(Token::Eof) => return false,
                    _ => {
                        i += 1;
                    }
                }
            }
        }
        // Skip array dims
        while matches!(self.tokens.get(i), Some(Token::LBracket))
            && matches!(self.tokens.get(i + 1), Some(Token::RBracket))
        {
            i += 2;
        }
        // Must be followed by an identifier (the binding name)
        if !matches!(self.tokens.get(i), Some(Token::Ident(_))) {
            return false;
        }
        i += 1;
        // Then -> or , or 'when' keyword (as Ident)
        matches!(self.tokens.get(i), Some(Token::Arrow | Token::Comma))
            || matches!(self.tokens.get(i), Some(Token::Ident(w)) if w == "when")
    }
}
