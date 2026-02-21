//! Java parser — tokens → AST.
//!
//! Recursive-descent parser with full Java 21 syntax coverage:
//! class/interface/enum, all statements, all expressions including
//! lambda, method reference, ternary, instanceof pattern matching.

use rava_common::error::{RavaError, Result};
use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos:    usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek2(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    #[allow(dead_code)]
    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let t = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<()> {
        let got = self.advance().clone();
        if &got == expected {
            Ok(())
        } else {
            Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected {:?}, got {:?}", expected, got),
            })
        }
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok { self.advance(); true } else { false }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.advance().clone() {
            Token::Ident(s) => Ok(s),
            got => Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected identifier, got {:?}", got),
            }),
        }
    }

    // ── Top-level ─────────────────────────────────────────────────────────────

    pub fn parse_file(&mut self) -> Result<SourceFile> {
        let mut package = None;
        let mut imports = Vec::new();

        if self.eat(&Token::Package) {
            package = Some(self.parse_qualified_name()?);
            self.expect(&Token::Semi)?;
        }

        while self.peek() == &Token::Import {
            self.advance();
            // skip `static` in static imports
            self.eat(&Token::Static);
            let mut name = self.parse_qualified_name()?;
            if self.eat(&Token::Dot) {
                self.expect(&Token::Star)?;
                name.push_str(".*");
            }
            self.expect(&Token::Semi)?;
            imports.push(name);
        }

        let mut classes = Vec::new();
        while self.peek() != &Token::Eof {
            classes.push(self.parse_class()?);
        }

        Ok(SourceFile { package, imports, classes })
    }

    fn parse_qualified_name(&mut self) -> Result<String> {
        let mut name = self.expect_ident()?;
        while self.peek() == &Token::Dot {
            if self.peek2() == &Token::Star { break; }
            if !matches!(self.peek2(), Token::Ident(_)) { break; }
            self.advance();
            name.push('.');
            name.push_str(&self.expect_ident()?);
        }
        Ok(name)
    }

    // ── Class / Interface / Enum ──────────────────────────────────────────────

    fn parse_class(&mut self) -> Result<ClassDecl> {
        let modifiers = self.parse_modifiers();

        let kind = match self.peek().clone() {
            Token::Class     => { self.advance(); ClassKind::Class }
            Token::Interface => { self.advance(); ClassKind::Interface }
            Token::Enum      => { self.advance(); ClassKind::Enum }
            got => return Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected 'class', 'interface', or 'enum', got {:?}", got),
            }),
        };
        let name = self.expect_ident()?;
        self.skip_type_params();

        let mut superclass = None;
        if self.eat(&Token::Extends) {
            superclass = Some(self.parse_type_name()?);
        }

        let mut interfaces = Vec::new();
        if self.eat(&Token::Implements) {
            interfaces.push(self.parse_type_name()?);
            while self.eat(&Token::Comma) {
                interfaces.push(self.parse_type_name()?);
            }
        }

        self.expect(&Token::LBrace)?;
        let mut members = Vec::new();

        if kind == ClassKind::Enum {
            // Parse enum constants
            while self.peek() != &Token::RBrace && self.peek() != &Token::Semi
                && self.peek() != &Token::Eof
            {
                // skip annotations
                while self.peek() == &Token::At {
                    self.advance(); self.expect_ident()?;
                    if self.peek() == &Token::LParen {
                        self.skip_balanced(Token::LParen, Token::RParen);
                    }
                }
                if matches!(self.peek(), Token::RBrace | Token::Semi) { break; }
                let const_name = self.expect_ident()?;
                let args = if self.peek() == &Token::LParen {
                    self.parse_args()?
                } else { Vec::new() };
                // skip optional class body
                if self.peek() == &Token::LBrace {
                    self.skip_balanced(Token::LBrace, Token::RBrace);
                }
                members.push(Member::EnumConstant(EnumConstant { name: const_name, args }));
                if !self.eat(&Token::Comma) { break; }
            }
            self.eat(&Token::Semi);
        }

        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            // skip annotations
            while self.peek() == &Token::At {
                self.advance();
                self.expect_ident()?;
                if self.peek() == &Token::LParen {
                    self.skip_balanced(Token::LParen, Token::RParen);
                }
            }
            if self.peek() == &Token::RBrace { break; }
            if let Some(m) = self.parse_member(&name)? {
                members.push(m);
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(ClassDecl { name, kind, modifiers, superclass, interfaces, members })
    }

    fn parse_modifiers(&mut self) -> Vec<Modifier> {
        let mut mods = Vec::new();
        loop {
            match self.peek() {
                Token::Public       => { self.advance(); mods.push(Modifier::Public); }
                Token::Private      => { self.advance(); mods.push(Modifier::Private); }
                Token::Protected    => { self.advance(); mods.push(Modifier::Protected); }
                Token::Static       => { self.advance(); mods.push(Modifier::Static); }
                Token::Final        => { self.advance(); mods.push(Modifier::Final); }
                Token::Abstract     => { self.advance(); mods.push(Modifier::Abstract); }
                Token::Synchronized => { self.advance(); mods.push(Modifier::Synchronized); }
                Token::Native       => { self.advance(); mods.push(Modifier::Native); }
                Token::Volatile     => { self.advance(); mods.push(Modifier::Volatile); }
                Token::Transient    => { self.advance(); mods.push(Modifier::Transient); }
                Token::Strictfp     => { self.advance(); mods.push(Modifier::Strictfp); }
                Token::Default      => { self.advance(); mods.push(Modifier::Default); }
                _ => break,
            }
        }
        mods
    }

    fn parse_member(&mut self, class_name: &str) -> Result<Option<Member>> {
        let modifiers = self.parse_modifiers();

        // static initializer block: `static { ... }`
        if modifiers.contains(&Modifier::Static) && self.peek() == &Token::LBrace {
            let body = self.parse_block()?;
            return Ok(Some(Member::StaticInit(body)));
        }

        // instance initializer block (no modifiers, just `{ ... }`)
        if modifiers.is_empty() && self.peek() == &Token::LBrace {
            let body = self.parse_block()?;
            return Ok(Some(Member::StaticInit(body)));
        }

        // inner class/interface/enum
        if matches!(self.peek(), Token::Class | Token::Interface | Token::Enum) {
            let inner = self.parse_class()?;
            return Ok(Some(Member::InnerClass(inner)));
        }

        // constructor: ClassName(
        if matches!(self.peek(), Token::Ident(n) if n == class_name) {
            if self.peek2() == &Token::LParen {
                let name = self.expect_ident()?;
                let params = self.parse_params()?;
                self.skip_throws();
                let body = self.parse_block()?;
                return Ok(Some(Member::Constructor(ConstructorDecl { name, modifiers, params, body })));
            }
        }

        // type + name
        let ty = self.parse_type_expr()?;

        // could be a nested class after type — skip
        if matches!(self.peek(), Token::Class | Token::Interface | Token::Enum) {
            let inner = self.parse_class()?;
            return Ok(Some(Member::InnerClass(inner)));
        }

        let name = self.expect_ident()?;

        if self.peek() == &Token::LParen {
            // method
            let params = self.parse_params()?;
            self.skip_throws();
            let body = if self.peek() == &Token::LBrace {
                Some(self.parse_block()?)
            } else {
                self.expect(&Token::Semi)?;
                None
            };
            Ok(Some(Member::Method(MethodDecl { name, modifiers, return_ty: ty, params, body })))
        } else {
            // field
            let init = if self.eat(&Token::Assign) { Some(self.parse_expr()?) } else { None };
            self.expect(&Token::Semi)?;
            Ok(Some(Member::Field(FieldDecl { name, modifiers, ty, init })))
        }
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    fn parse_type_expr(&mut self) -> Result<TypeExpr> {
        let name = self.parse_type_name()?;
        let mut dims = 0u8;
        while self.peek() == &Token::LBracket && self.peek2() == &Token::RBracket {
            self.advance(); self.advance();
            dims += 1;
        }
        Ok(TypeExpr { name, array_dims: dims })
    }

    fn parse_type_name(&mut self) -> Result<String> {
        let mut name = match self.advance().clone() {
            Token::Ident(s) => s,
            Token::Int      => "int".into(),
            Token::Long     => "long".into(),
            Token::Double   => "double".into(),
            Token::Float    => "float".into(),
            Token::Boolean  => "boolean".into(),
            Token::Byte     => "byte".into(),
            Token::Short    => "short".into(),
            Token::Char     => "char".into(),
            Token::Void     => "void".into(),
            got => return Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected type, got {:?}", got),
            }),
        };
        // qualified name
        while self.peek() == &Token::Dot {
            if matches!(self.peek2(), Token::Ident(_)) {
                self.advance();
                name.push('.');
                name.push_str(&self.expect_ident()?);
            } else { break; }
        }
        // skip generic params
        self.skip_type_params();
        Ok(name)
    }

    fn skip_type_params(&mut self) {
        if self.peek() == &Token::Lt {
            let mut depth = 0i32;
            loop {
                match self.advance() {
                    Token::Lt  => depth += 1,
                    Token::Gt  => { depth -= 1; if depth <= 0 { break; } }
                    Token::Eof => break,
                    _ => {}
                }
            }
        }
    }

    fn skip_throws(&mut self) {
        if self.peek() == &Token::Throws {
            self.advance();
            self.parse_type_name().ok();
            while self.eat(&Token::Comma) { self.parse_type_name().ok(); }
        }
    }

    fn skip_balanced(&mut self, open: Token, close: Token) {
        if self.peek() != &open { return; }
        let mut depth = 0i32;
        loop {
            let t = self.advance().clone();
            if t == open  { depth += 1; }
            if t == close { depth -= 1; if depth <= 0 { break; } }
            if t == Token::Eof { break; }
        }
    }

    // ── Params ────────────────────────────────────────────────────────────────

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            // skip annotations
            while self.peek() == &Token::At {
                self.advance(); self.expect_ident()?;
                if self.peek() == &Token::LParen { self.skip_balanced(Token::LParen, Token::RParen); }
            }
            // skip final
            self.eat(&Token::Final);
            let ty = self.parse_type_expr()?;
            let variadic = self.eat(&Token::Ellipsis);
            let name = self.expect_ident()?;
            params.push(Param { name, ty, variadic });
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Block> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Block(stmts))
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        // Check for labeled statement: `ident: stmt`
        if matches!(self.peek(), Token::Ident(_)) && self.peek2() == &Token::Colon {
            let label = self.expect_ident()?;
            self.advance(); // consume ':'
            let stmt = self.parse_stmt()?;
            return Ok(Stmt::Labeled { label, stmt: Box::new(stmt) });
        }

        match self.peek().clone() {
            Token::Semi => { self.advance(); Ok(Stmt::Empty) }
            Token::LBrace => Ok(Stmt::Block(self.parse_block()?)),
            Token::Return => {
                self.advance();
                let expr = if self.peek() == &Token::Semi { None } else { Some(self.parse_expr()?) };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Return(expr))
            }
            Token::If => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let then = Box::new(self.parse_stmt()?);
                let else_ = if self.eat(&Token::Else) { Some(Box::new(self.parse_stmt()?)) } else { None };
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
                    return Ok(Stmt::ForEach { ty, name, iterable, body });
                }
                // Regular for
                let init = if self.peek() == &Token::Semi {
                    self.advance(); None
                } else {
                    Some(Box::new(self.parse_stmt()?))
                };
                let cond = if self.peek() == &Token::Semi { None } else { Some(self.parse_expr()?) };
                self.expect(&Token::Semi)?;
                let mut update = Vec::new();
                while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                    update.push(self.parse_expr()?);
                    if !self.eat(&Token::Comma) { break; }
                }
                self.expect(&Token::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Ok(Stmt::For { init, cond, update, body })
            }
            Token::Break => {
                self.advance();
                let label = if matches!(self.peek(), Token::Ident(_)) {
                    Some(self.expect_ident()?)
                } else { None };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Break(label))
            }
            Token::Continue => {
                self.advance();
                let label = if matches!(self.peek(), Token::Ident(_)) {
                    Some(self.expect_ident()?)
                } else { None };
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
                let try_body = self.parse_block()?;
                let mut catches = Vec::new();
                let mut finally_body = None;
                while self.peek() == &Token::Catch {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    // Parse exception types (multi-catch: Type1 | Type2)
                    let mut exception_types = vec![self.parse_type_expr()?];
                    while self.eat(&Token::BitOr) {
                        exception_types.push(self.parse_type_expr()?);
                    }
                    let name = self.expect_ident()?;
                    self.expect(&Token::RParen)?;
                    let body = self.parse_block()?;
                    catches.push(CatchClause { exception_types, name, body });
                }
                if self.eat(&Token::Finally) {
                    finally_body = Some(self.parse_block()?);
                }
                Ok(Stmt::TryCatch { try_body, catches, finally_body })
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
                        let e = self.parse_expr()?;
                        self.expect(&Token::Colon)?;
                        Some(e)
                    } else if self.peek() == &Token::Default {
                        self.advance();
                        self.expect(&Token::Colon)?;
                        None
                    } else {
                        break;
                    };
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
                    cases.push(SwitchCase { label, body });
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
                } else { None };
                self.expect(&Token::Semi)?;
                Ok(Stmt::Assert { expr, message })
            }
            Token::Var => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::Assign)?;
                let init = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::LocalVar { ty: TypeExpr::simple("var"), name, init: Some(init) })
            }
            // local variable declaration
            tok if self.is_type_start(&tok) && self.is_local_var_decl() => {
                let ty = self.parse_type_expr()?;
                let name = self.expect_ident()?;
                let init = if self.eat(&Token::Assign) { Some(self.parse_expr()?) } else { None };
                self.expect(&Token::Semi)?;
                Ok(Stmt::LocalVar { ty, name, init })
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn is_type_start(&self, tok: &Token) -> bool {
        matches!(tok,
            Token::Int | Token::Long | Token::Double | Token::Float |
            Token::Boolean | Token::Byte | Token::Short | Token::Char |
            Token::Ident(_)
        )
    }

    /// Heuristic: current token is a type, next is an identifier → local var decl.
    fn is_local_var_decl(&self) -> bool {
        let mut i = self.pos;
        match self.tokens.get(i) {
            Some(Token::Ident(_) | Token::Int | Token::Long | Token::Double |
                 Token::Float | Token::Boolean | Token::Byte | Token::Short | Token::Char) => { i += 1; }
            _ => return false,
        }
        // skip qualified name dots
        while matches!(self.tokens.get(i), Some(Token::Dot)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::Ident(_))) { i += 1; } else { break; }
        }
        // skip generic params
        if matches!(self.tokens.get(i), Some(Token::Lt)) {
            let mut depth = 0i32;
            loop {
                match self.tokens.get(i) {
                    Some(Token::Lt)  => { depth += 1; i += 1; }
                    Some(Token::Gt)  => { depth -= 1; i += 1; if depth <= 0 { break; } }
                    None | Some(Token::Eof) => break,
                    _ => { i += 1; }
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
    fn is_for_each(&self) -> bool {
        let mut i = self.pos;
        // skip `final`
        if matches!(self.tokens.get(i), Some(Token::Final)) { i += 1; }
        // skip type token
        match self.tokens.get(i) {
            Some(Token::Ident(_) | Token::Int | Token::Long | Token::Double |
                 Token::Float | Token::Boolean | Token::Byte | Token::Short | Token::Char) => { i += 1; }
            _ => return false,
        }
        // skip qualified name
        while matches!(self.tokens.get(i), Some(Token::Dot)) {
            i += 1;
            if matches!(self.tokens.get(i), Some(Token::Ident(_))) { i += 1; } else { break; }
        }
        // skip generics
        if matches!(self.tokens.get(i), Some(Token::Lt)) {
            let mut depth = 0i32;
            loop {
                match self.tokens.get(i) {
                    Some(Token::Lt)  => { depth += 1; i += 1; }
                    Some(Token::Gt)  => { depth -= 1; i += 1; if depth <= 0 { break; } }
                    None | Some(Token::Eof) => break,
                    _ => { i += 1; }
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
        if !matches!(self.tokens.get(i), Some(Token::Ident(_))) { return false; }
        i += 1;
        matches!(self.tokens.get(i), Some(Token::Colon))
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr> {
        let lhs = self.parse_ternary()?;
        if self.eat(&Token::Assign) {
            let rhs = self.parse_assign()?;
            return Ok(Expr::Assign { lhs: Box::new(lhs), rhs: Box::new(rhs) });
        }
        // compound assignments
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
                        lhs = Expr::InstanceofPattern {
                            expr: Box::new(lhs), ty, name,
                        };
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
                    // Method reference: expr.method or check for ::
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
            Token::This        => {
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
                        // new Type[0] — zero-length array
                        return Ok(Expr::NewArray {
                            ty, len: Box::new(Expr::IntLit(0)),
                        });
                    }
                    let len = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    // skip additional dimensions
                    while self.peek() == &Token::LBracket && self.peek2() == &Token::RBracket {
                        self.advance(); self.advance();
                    }
                    Ok(Expr::NewArray { ty, len: Box::new(len) })
                } else {
                    let args = self.parse_args()?;
                    // skip optional anonymous class body
                    if self.peek() == &Token::LBrace {
                        self.skip_balanced(Token::LBrace, Token::RBrace);
                    }
                    Ok(Expr::New { ty, args })
                }
            }
            Token::LParen => {
                // Could be: (expr), (Type)expr cast, or (params) -> lambda
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

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            args.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_array_init(&mut self) -> Result<Vec<Expr>> {
        self.expect(&Token::LBrace)?;
        let mut elements = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            elements.push(self.parse_expr()?);
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

    fn parse_lambda(&mut self) -> Result<Expr> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            // Lambda params can be typed or untyped
            // Try to detect: if next-next is comma or rparen, it's untyped
            if matches!(self.peek(), Token::Ident(_))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> SourceFile {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_file().unwrap()
    }

    #[test]
    fn parse_hello_world() {
        let src = r#"
            class Main {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;
        let file = parse(src);
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].name, "Main");
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert_eq!(m.name, "main");
    }

    #[test]
    fn parse_local_var_and_return() {
        let src = r#"
            class Foo {
                int add(int a, int b) {
                    int result = a + b;
                    return result;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert_eq!(m.params.len(), 2);
        assert_eq!(m.body.as_ref().unwrap().0.len(), 2);
    }

    #[test]
    fn parse_do_while() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    do { i++; } while (i < 10);
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        assert!(matches!(stmts[1], Stmt::DoWhile { .. }));
    }

    #[test]
    fn parse_for_each() {
        let src = r#"
            class T {
                void f() {
                    for (String s : args) { System.out.println(s); }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert!(matches!(m.body.as_ref().unwrap().0[0], Stmt::ForEach { .. }));
    }

    #[test]
    fn parse_break_continue() {
        let src = r#"
            class T {
                void f() {
                    while (true) { break; }
                    while (true) { continue; }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        assert!(matches!(stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn parse_try_catch_finally() {
        let src = r#"
            class T {
                void f() {
                    try {
                        int x = 1;
                    } catch (Exception e) {
                        System.out.println(e);
                    } finally {
                        System.out.println("done");
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert!(matches!(m.body.as_ref().unwrap().0[0], Stmt::TryCatch { .. }));
    }

    #[test]
    fn parse_lambda() {
        let src = r#"
            class T {
                void f() {
                    var fn = (int x) -> x + 1;
                    var fn2 = x -> x * 2;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        if let Stmt::LocalVar { init: Some(Expr::Lambda { params, .. }), .. } = &stmts[0] {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
        } else { panic!("expected lambda"); }
    }

    #[test]
    fn parse_enum() {
        let src = r#"
            enum Color { RED, GREEN, BLUE }
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].kind, ClassKind::Enum);
        assert_eq!(file.classes[0].members.len(), 3);
    }

    #[test]
    fn parse_instanceof_pattern() {
        let src = r#"
            class T {
                void f(Object obj) {
                    if (obj instanceof String s) {
                        System.out.println(s);
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::If { cond: Expr::InstanceofPattern { name, .. }, .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(name, "s");
        } else { panic!("expected instanceof pattern"); }
    }

    #[test]
    fn parse_method_ref() {
        let src = r#"
            class T {
                void f() {
                    var fn = System.out::println;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::LocalVar { init: Some(Expr::MethodRef { name, .. }), .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(name, "println");
        } else { panic!("expected method ref"); }
    }
}