use rava_common::error::Result;
use crate::ast::*;
use crate::lexer::Token;
use super::Parser;

impl Parser {
    pub(crate) fn parse_class(&mut self) -> Result<ClassDecl> {
        let modifiers = self.parse_modifiers();

        let kind = match self.peek().clone() {
            Token::Class     => { self.advance(); ClassKind::Class }
            Token::Interface => { self.advance(); ClassKind::Interface }
            Token::Enum      => { self.advance(); ClassKind::Enum }
            Token::Record    => { self.advance(); ClassKind::Record }
            got => return Err(rava_common::error::RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected 'class', 'interface', 'enum', or 'record', got {:?}", got),
            }),
        };
        let name = self.expect_ident()?;
        self.skip_type_params();

        // Record components: record Point(int x, int y) { ... }
        let record_components: Vec<(TypeExpr, String)> = if kind == ClassKind::Record && self.peek() == &Token::LParen {
            let params = self.parse_params()?;
            params.into_iter().map(|p| (p.ty, p.name)).collect()
        } else {
            Vec::new()
        };

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

        // sealed classes: `permits SubA, SubB` — parse and discard for now
        if self.eat(&Token::Permits) {
            self.parse_type_name()?;
            while self.eat(&Token::Comma) {
                self.parse_type_name()?;
            }
        }

        self.expect(&Token::LBrace)?;
        let mut members = Vec::new();

        // Desugar record components into fields + constructor + accessors
        if kind == ClassKind::Record && !record_components.is_empty() {
            // Final fields
            for (ty, fname) in &record_components {
                members.push(Member::Field(FieldDecl {
                    name: fname.clone(),
                    modifiers: vec![Modifier::Private, Modifier::Final],
                    ty: ty.clone(),
                    init: None,
                }));
            }

            let params: Vec<Param> = record_components.iter().map(|(ty, fname)| {
                Param { name: fname.clone(), ty: ty.clone(), variadic: false }
            }).collect();
            let assign_stmts: Vec<Stmt> = record_components.iter().map(|(_, fname)| {
                Stmt::Expr(Expr::Assign {
                    lhs: Box::new(Expr::Field { obj: Box::new(Expr::This), name: fname.clone() }),
                    rhs: Box::new(Expr::Ident(fname.clone())),
                })
            }).collect();

            // Parse body: look for compact constructor `ClassName { ... }` or explicit constructor
            let mut has_explicit_ctor = false;
            let mut compact_body: Option<Block> = None;

            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                // skip annotations
                while self.peek() == &Token::At {
                    self.advance(); self.expect_ident()?;
                    if self.peek() == &Token::LParen { self.skip_balanced(Token::LParen, Token::RParen); }
                }
                if self.peek() == &Token::RBrace { break; }

                // Compact constructor: `ClassName {` (no LParen)
                if matches!(self.peek(), Token::Ident(n) if n == &name)
                    && self.peek2() == &Token::LBrace
                {
                    self.advance(); // consume class name
                    compact_body = Some(self.parse_block()?);
                    has_explicit_ctor = true;
                    continue;
                }
                // Explicit canonical constructor: `ClassName (`
                if matches!(self.peek(), Token::Ident(n) if n == &name)
                    && self.peek2() == &Token::LParen
                {
                    has_explicit_ctor = true;
                }
                if let Some(m) = self.parse_member(&name)? {
                    members.push(m);
                }
            }
            self.expect(&Token::RBrace)?;

            if !has_explicit_ctor {
                members.push(Member::Constructor(ConstructorDecl {
                    name: name.clone(),
                    modifiers: vec![Modifier::Public],
                    params: params.clone(),
                    body: Block(assign_stmts.clone()),
                }));
            } else if let Some(cb) = compact_body {
                // Compact constructor: field assignments first, then compact body
                let mut body_stmts = assign_stmts.clone();
                body_stmts.extend(cb.0);
                members.push(Member::Constructor(ConstructorDecl {
                    name: name.clone(),
                    modifiers: vec![Modifier::Public],
                    params: params.clone(),
                    body: Block(body_stmts),
                }));
            }

            // Accessor methods
            for (ty, fname) in &record_components {
                let already_defined = members.iter().any(|m| {
                    matches!(m, Member::Method(md) if md.name == *fname)
                });
                if !already_defined {
                    members.push(Member::Method(MethodDecl {
                        name: fname.clone(),
                        modifiers: vec![Modifier::Public],
                        return_ty: ty.clone(),
                        params: vec![],
                        body: Some(Block(vec![
                            Stmt::Return(Some(Expr::Field {
                                obj: Box::new(Expr::This),
                                name: fname.clone(),
                            })),
                        ])),
                    }));
                }
            }

            return Ok(ClassDecl { name, kind, modifiers, superclass, interfaces, members });
        }

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

    pub(crate) fn parse_modifiers(&mut self) -> Vec<Modifier> {
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
                Token::Sealed       => { self.advance(); mods.push(Modifier::Abstract); }
                _ => break,
            }
        }
        mods
    }

    pub(crate) fn parse_member(&mut self, class_name: &str) -> Result<Option<Member>> {
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

        // inner class/interface/enum/record
        if matches!(self.peek(), Token::Class | Token::Interface | Token::Enum | Token::Record) {
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
}
