use rava_common::error::{RavaError, Result};
use crate::ast::*;
use crate::lexer::Token;
use super::Parser;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr> {
        let lhs = self.parse_ternary()?;
        if self.eat(&Token::Assign) {
            let rhs = self.parse_assign()?;
            return Ok(Expr::Assign { lhs: Box::new(lhs), rhs: Box::new(rhs) });
        }
        let compound_op = match self.peek() {
            Token::PlusAssign    => Some(BinOp::Add),
            Token::MinusAssign   => Some(BinOp::Sub),
            Token::StarAssign    => Some(BinOp::Mul),
            Token::SlashAssign   => Some(BinOp::Div),
            Token::PercentAssign => Some(BinOp::Rem),
            Token::BitAndAssign  => Some(BinOp::BitAnd),
            Token::BitOrAssign   => Some(BinOp::BitOr),
            Token::BitXorAssign  => Some(BinOp::BitXor),
            Token::ShlAssign     => Some(BinOp::Shl),
            Token::ShrAssign     => Some(BinOp::Shr),
            Token::UShrAssign    => Some(BinOp::UShr),
            _ => None,
        };
        if let Some(op) = compound_op {
            self.advance();
            let rhs = self.parse_assign()?;
            return Ok(Expr::CompoundAssign { op, lhs: Box::new(lhs), rhs: Box::new(rhs) });
        }
        Ok(lhs)
    }

    fn parse_ternary(&mut self) -> Result<Expr> {
        let cond = self.parse_or()?;
        if self.eat(&Token::Question) {
            let then = self.parse_expr()?;
            self.expect(&Token::Colon)?;
            let else_ = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond), then: Box::new(then), else_: Box::new(else_),
            });
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::BinOp { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_bitor()?;
        while self.peek() == &Token::And {
            self.advance();
            let rhs = self.parse_bitor()?;
            lhs = Expr::BinOp { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_bitor(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_bitxor()?;
        while self.peek() == &Token::BitOr {
            self.advance();
            let rhs = self.parse_bitxor()?;
            lhs = Expr::BinOp { op: BinOp::BitOr, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == &Token::BitXor {
            self.advance();
            let rhs = self.parse_bitand()?;
            lhs = Expr::BinOp { op: BinOp::BitXor, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_bitand(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_eq()?;
        while self.peek() == &Token::BitAnd {
            self.advance();
            let rhs = self.parse_eq()?;
            lhs = Expr::BinOp { op: BinOp::BitAnd, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_eq(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_rel()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_rel()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_rel(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_shift()?;
        loop {
            match self.peek().clone() {
                Token::Lt => {
                    self.advance();
                    let rhs = self.parse_shift()?;
                    lhs = Expr::BinOp { op: BinOp::Lt, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Token::Le => {
                    self.advance();
                    let rhs = self.parse_shift()?;
                    lhs = Expr::BinOp { op: BinOp::Le, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Token::Gt => {
                    self.advance();
                    let rhs = self.parse_shift()?;
                    lhs = Expr::BinOp { op: BinOp::Gt, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Token::Ge => {
                    self.advance();
                    let rhs = self.parse_shift()?;
                    lhs = Expr::BinOp { op: BinOp::Ge, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Token::Instanceof => {
                    self.advance();
                    let ty = self.parse_type_expr()?;
                    // Pattern matching: `instanceof Type name`
                    if matches!(self.peek(), Token::Ident(_)) {
                        let name = self.expect_ident()?;
                        lhs = Expr::InstanceofPattern { expr: Box::new(lhs), ty, name };
                    } else {
                        lhs = Expr::Instanceof { expr: Box::new(lhs), ty };
                    }
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Token::Shl  => BinOp::Shl,
                Token::Shr  => BinOp::Shr,
                Token::UShr => BinOp::UShr,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_add()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star    => BinOp::Mul,
                Token::Slash   => BinOp::Div,
                Token::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Neg, expr: Box::new(e) })
            }
            Token::Not => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Not, expr: Box::new(e) })
            }
            Token::Tilde => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::BitNot, expr: Box::new(e) })
            }
            Token::PlusPlus => {
                self.advance();
                let e = self.parse_postfix()?;
                Ok(Expr::UnaryOp { op: UnaryOp::PreInc, expr: Box::new(e) })
            }
            Token::MinusMinus => {
                self.advance();
                let e = self.parse_postfix()?;
                Ok(Expr::UnaryOp { op: UnaryOp::PreDec, expr: Box::new(e) })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek().clone() {
                Token::Dot => {
                    self.advance();
                    let name = self.expect_ident()?;
                    if self.peek() == &Token::LParen {
                        let args = self.parse_args()?;
                        expr = Expr::Call {
                            callee: Box::new(Expr::Field { obj: Box::new(expr), name }),
                            args,
                        };
                    } else {
                        expr = Expr::Field { obj: Box::new(expr), name };
                    }
                }
                Token::ColonColon => {
                    self.advance();
                    let name = if self.eat(&Token::New) {
                        "new".to_string()
                    } else {
                        self.expect_ident()?
                    };
                    expr = Expr::MethodRef { obj: Box::new(expr), name };
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index { arr: Box::new(expr), idx: Box::new(idx) };
                }
                Token::PlusPlus => {
                    self.advance();
                    expr = Expr::UnaryOp { op: UnaryOp::PostInc, expr: Box::new(expr) };
                }
                Token::MinusMinus => {
                    self.advance();
                    expr = Expr::UnaryOp { op: UnaryOp::PostDec, expr: Box::new(expr) };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.peek().clone() {
            Token::IntLit(n)   => { self.advance(); Ok(Expr::IntLit(n)) }
            Token::FloatLit(f) => { self.advance(); Ok(Expr::FloatLit(f)) }
            Token::StrLit(s)   => { self.advance(); Ok(Expr::StrLit(s)) }
            Token::CharLit(c)  => { self.advance(); Ok(Expr::CharLit(c)) }
            Token::BoolLit(b)  => { self.advance(); Ok(Expr::BoolLit(b)) }
            Token::Null        => { self.advance(); Ok(Expr::Null) }
            Token::Switch      => {
                // Switch expression: switch (expr) { case X -> val; ... }
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::LBrace)?;
                let mut cases = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    let label = if self.eat(&Token::Default) {
                        None
                    } else {
                        self.expect(&Token::Case)?;
                        let mut labels = vec![self.parse_case_label()?];
                        while self.eat(&Token::Comma) {
                            labels.push(self.parse_case_label()?);
                        }
                        Some(labels)
                    };
                    let mut body = Vec::new();
                    if self.eat(&Token::Arrow) {
                        if self.peek() == &Token::LBrace {
                            body = self.parse_block()?.0;
                        } else {
                            let e = self.parse_expr()?;
                            body.push(Stmt::Yield(e));
                            self.eat(&Token::Semi);
                        }
                    } else {
                        self.expect(&Token::Colon)?;
                        while self.peek() != &Token::Case && self.peek() != &Token::Default
                            && self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                            body.push(self.parse_stmt()?);
                        }
                    }
                    cases.push(SwitchCase { labels: label, body });
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::SwitchExpr { expr: Box::new(expr), cases })
            }
            Token::This => {
                self.advance();
                // this::method
                if self.peek() == &Token::ColonColon {
                    self.advance();
                    let name = self.expect_ident()?;
                    return Ok(Expr::MethodRef { obj: Box::new(Expr::This), name });
                }
                Ok(Expr::This)
            }
            Token::Super => {
                self.advance();
                if self.peek() == &Token::Dot {
                    self.advance();
                    let name = self.expect_ident()?;
                    if self.peek() == &Token::LParen {
                        let args = self.parse_args()?;
                        Ok(Expr::Call {
                            callee: Box::new(Expr::Field { obj: Box::new(Expr::Super), name }),
                            args,
                        })
                    } else {
                        Ok(Expr::Field { obj: Box::new(Expr::Super), name })
                    }
                } else if self.peek() == &Token::LParen {
                    let args = self.parse_args()?;
                    Ok(Expr::Call { callee: Box::new(Expr::Super), args })
                } else {
                    Ok(Expr::Super)
                }
            }
            Token::New => {
                self.advance();
                let ty = self.parse_type_expr()?;
                if self.peek() == &Token::LBracket {
                    self.advance();
                    // new Type[] { ... } — array init with explicit type
                    if self.peek() == &Token::RBracket {
                        self.advance();
                        if self.peek() == &Token::LBrace {
                            let elements = self.parse_array_init()?;
                            return Ok(Expr::ArrayInit { ty: Some(ty), elements });
                        }
                        return Ok(Expr::NewArray {
                            ty, len: Box::new(Expr::IntLit(0)),
                        });
                    }
                    let len = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    let mut extra_dims: Vec<Expr> = Vec::new();
                    while self.peek() == &Token::LBracket {
                        self.advance();
                        if self.peek() == &Token::RBracket {
                            self.advance();
                        } else {
                            let dim = self.parse_expr()?;
                            self.expect(&Token::RBracket)?;
                            extra_dims.push(dim);
                        }
                    }
                    if extra_dims.is_empty() {
                        Ok(Expr::NewArray { ty, len: Box::new(len) })
                    } else {
                        let mut dims = vec![len];
                        dims.extend(extra_dims);
                        Ok(Expr::NewMultiArray { ty, dims })
                    }
                } else {
                    let args = self.parse_args()?;
                    // Anonymous class body: new Type(args) { members... }
                    let body = if self.peek() == &Token::LBrace {
                        self.advance();
                        let mut members = Vec::new();
                        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                            if let Some(m) = self.parse_member("__anon__")? {
                                members.push(m);
                            }
                        }
                        self.expect(&Token::RBrace)?;
                        Some(members)
                    } else {
                        None
                    };
                    Ok(Expr::New { ty, args, body })
                }
            }
            Token::LParen => {
                if self.is_lambda() {
                    return self.parse_lambda();
                }
                self.advance();
                if self.is_cast() {
                    let ty = self.parse_type_expr()?;
                    self.expect(&Token::RParen)?;
                    let expr = self.parse_unary()?;
                    return Ok(Expr::Cast { ty, expr: Box::new(expr) });
                }
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::Ident(name) => {
                // Check for lambda: `name -> ...`
                if self.peek2() == &Token::Arrow {
                    self.advance();
                    self.advance(); // consume ->
                    let body = if self.peek() == &Token::LBrace {
                        LambdaBody::Block(self.parse_block()?)
                    } else {
                        LambdaBody::Expr(self.parse_expr()?)
                    };
                    return Ok(Expr::Lambda {
                        params: vec![LambdaParam { name, ty: None }],
                        body: Box::new(body),
                    });
                }
                self.advance();
                // method reference: Type::method
                if self.peek() == &Token::ColonColon {
                    self.advance();
                    let method = if self.eat(&Token::New) {
                        "new".to_string()
                    } else {
                        self.expect_ident()?
                    };
                    return Ok(Expr::MethodRef { obj: Box::new(Expr::Ident(name)), name: method });
                }
                if self.peek() == &Token::LParen {
                    let args = self.parse_args()?;
                    Ok(Expr::Call { callee: Box::new(Expr::Ident(name)), args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            got => Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("unexpected token in expression: {:?}", got),
            }),
        }
    }

    pub(crate) fn parse_args(&mut self) -> Result<Vec<Expr>> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            args.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    pub(crate) fn parse_array_init(&mut self) -> Result<Vec<Expr>> {
        self.expect(&Token::LBrace)?;
        let mut elements = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.peek() == &Token::LBrace {
                let inner = self.parse_array_init()?;
                elements.push(Expr::ArrayInit { ty: None, elements: inner });
            } else {
                elements.push(self.parse_expr()?);
            }
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RBrace)?;
        Ok(elements)
    }

    /// Detect lambda: `(params) -> ...`
    fn is_lambda(&self) -> bool {
        if self.peek() != &Token::LParen { return false; }
        let mut i = self.pos + 1;
        let mut depth = 1i32;
        loop {
            match self.tokens.get(i) {
                Some(Token::LParen) => { depth += 1; i += 1; }
                Some(Token::RParen) => {
                    depth -= 1;
                    i += 1;
                    if depth == 0 { break; }
                }
                None | Some(Token::Eof) => return false,
                _ => { i += 1; }
            }
        }
        matches!(self.tokens.get(i), Some(Token::Arrow))
    }

    pub(crate) fn parse_lambda(&mut self) -> Result<Expr> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            if self.peek() == &Token::Var {
                self.advance();
                let name = self.expect_ident()?;
                params.push(LambdaParam { name, ty: Some(TypeExpr::simple("var")) });
            } else if matches!(self.peek(), Token::Ident(_))
                && matches!(self.peek2(), Token::Comma | Token::RParen)
            {
                let name = self.expect_ident()?;
                params.push(LambdaParam { name, ty: None });
            } else {
                let ty = self.parse_type_expr()?;
                let name = self.expect_ident()?;
                params.push(LambdaParam { name, ty: Some(ty) });
            }
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::Arrow)?;
        let body = if self.peek() == &Token::LBrace {
            LambdaBody::Block(self.parse_block()?)
        } else {
            LambdaBody::Expr(self.parse_expr()?)
        };
        Ok(Expr::Lambda { params, body: Box::new(body) })
    }

    /// Heuristic: `(Type)` cast vs `(expr)` grouping.
    fn is_cast(&self) -> bool {
        let mut i = self.pos;
        loop {
            match self.tokens.get(i) {
                Some(Token::Ident(_) | Token::Int | Token::Long | Token::Double |
                     Token::Float | Token::Boolean | Token::Byte | Token::Short | Token::Char) => { i += 1; }
                Some(Token::Dot) => { i += 1; }
                _ => break,
            }
        }
        // skip array dims
        while matches!(self.tokens.get(i), Some(Token::LBracket)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::RBracket)) { i += 1; }
        }
        matches!(self.tokens.get(i), Some(Token::RParen))
    }
}
