use super::Parser;
use crate::ast::*;
use crate::lexer::Token;
use rava_common::error::Result;

impl Parser {
    pub(crate) fn parse_class(&mut self) -> Result<ClassDecl> {
        let annotations = self.parse_annotations()?;
        let modifiers = self.parse_modifiers();

        let kind = match self.peek().clone() {
            Token::At if self.peek2() == &Token::Interface => {
                self.advance(); // @
                self.advance(); // interface
                ClassKind::Annotation
            }
            Token::Class => {
                self.advance();
                ClassKind::Class
            }
            Token::Interface => {
                self.advance();
                ClassKind::Interface
            }
            Token::Enum => {
                self.advance();
                ClassKind::Enum
            }
            Token::Record => {
                self.advance();
                ClassKind::Record
            }
            got => {
                return Err(rava_common::error::RavaError::Parse {
                    location: format!("pos {}", self.pos),
                    message: format!(
                    "expected 'class', 'interface', 'enum', 'record', or '@interface', got {:?}",
                    got
                ),
                })
            }
        };
        let name = self.expect_ident()?;
        let type_params_raw = self.parse_angle_raw();

        // Record components: record Point(int x, int y) { ... }
        let record_components: Vec<(TypeExpr, String)> =
            if kind == ClassKind::Record && self.peek() == &Token::LParen {
                let params = self.parse_params()?;
                params.into_iter().map(|p| (p.ty, p.name)).collect()
            } else {
                Vec::new()
            };

        let mut interfaces = Vec::new();
        let mut interfaces_type_args_raw = Vec::new();
        let mut superclass = None;
        let mut superclass_type_args_raw = None;

        match kind {
            ClassKind::Interface | ClassKind::Annotation => {
                // Interface inheritance uses `extends A, B`.
                if self.eat(&Token::Extends) {
                    let first = self.parse_type_name()?;
                    let first_type_args = self.parse_angle_raw();
                    interfaces.push(first);
                    interfaces_type_args_raw.push(first_type_args);
                    while self.eat(&Token::Comma) {
                        let iface = self.parse_type_name()?;
                        let iface_type_args = self.parse_angle_raw();
                        interfaces.push(iface);
                        interfaces_type_args_raw.push(iface_type_args);
                    }
                }
            }
            _ => {
                if self.eat(&Token::Extends) {
                    let base = self.parse_type_name()?;
                    superclass_type_args_raw = self.parse_angle_raw();
                    superclass = Some(base);
                }
                if self.eat(&Token::Implements) {
                    let first = self.parse_type_name()?;
                    let first_type_args = self.parse_angle_raw();
                    interfaces.push(first);
                    interfaces_type_args_raw.push(first_type_args);
                    while self.eat(&Token::Comma) {
                        let iface = self.parse_type_name()?;
                        let iface_type_args = self.parse_angle_raw();
                        interfaces.push(iface);
                        interfaces_type_args_raw.push(iface_type_args);
                    }
                }
            }
        }

        let mut permitted_subclasses = Vec::new();
        let mut permitted_type_args_raw = Vec::new();
        // sealed classes: `permits SubA, SubB`
        if self.eat(&Token::Permits) {
            let first = self.parse_type_name()?;
            let first_type_args = self.parse_angle_raw();
            permitted_subclasses.push(first);
            permitted_type_args_raw.push(first_type_args);
            while self.eat(&Token::Comma) {
                let subclass = self.parse_type_name()?;
                let subclass_type_args = self.parse_angle_raw();
                permitted_subclasses.push(subclass);
                permitted_type_args_raw.push(subclass_type_args);
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
                    annotations: vec![],
                    ty: ty.clone(),
                    init: None,
                }));
            }

            let params: Vec<Param> = record_components
                .iter()
                .map(|(ty, fname)| Param {
                    name: fname.clone(),
                    ty: ty.clone(),
                    variadic: false,
                    annotations: vec![],
                })
                .collect();
            let assign_stmts: Vec<Stmt> = record_components
                .iter()
                .map(|(_, fname)| {
                    Stmt::Expr(Expr::Assign {
                        lhs: Box::new(Expr::Field {
                            obj: Box::new(Expr::This),
                            name: fname.clone(),
                        }),
                        rhs: Box::new(Expr::Ident(fname.clone())),
                    })
                })
                .collect();

            // Parse body: look for compact constructor `ClassName { ... }` or explicit constructor
            let mut has_explicit_ctor = false;
            let mut compact_body: Option<Block> = None;

            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                self.parse_annotations()?; // consume any annotations
                if self.peek() == &Token::RBrace {
                    break;
                }

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
                    members.append(&mut self.pending_fields);
                }
            }
            self.expect(&Token::RBrace)?;

            if !has_explicit_ctor {
                members.push(Member::Constructor(ConstructorDecl {
                    name: name.clone(),
                    type_params_raw: None,
                    modifiers: vec![Modifier::Public],
                    annotations: vec![],
                    params: params.clone(),
                    body: Block(assign_stmts.clone()),
                }));
            } else if let Some(cb) = compact_body {
                // Compact constructor: field assignments first, then compact body
                let mut body_stmts = assign_stmts.clone();
                body_stmts.extend(cb.0);
                members.push(Member::Constructor(ConstructorDecl {
                    name: name.clone(),
                    type_params_raw: None,
                    modifiers: vec![Modifier::Public],
                    annotations: vec![],
                    params: params.clone(),
                    body: Block(body_stmts),
                }));
            }

            // Accessor methods
            for (ty, fname) in &record_components {
                let already_defined = members
                    .iter()
                    .any(|m| matches!(m, Member::Method(md) if md.name == *fname));
                if !already_defined {
                    members.push(Member::Method(MethodDecl {
                        name: fname.clone(),
                        type_params_raw: None,
                        modifiers: vec![Modifier::Public],
                        annotations: vec![],
                        return_ty: ty.clone(),
                        params: vec![],
                        body: Some(Block(vec![Stmt::Return(Some(Expr::Field {
                            obj: Box::new(Expr::This),
                            name: fname.clone(),
                        }))])),
                    }));
                }
            }

            return Ok(ClassDecl {
                name,
                kind,
                type_params_raw,
                modifiers,
                annotations,
                superclass,
                superclass_type_args_raw,
                interfaces,
                interfaces_type_args_raw,
                permitted_subclasses,
                permitted_type_args_raw,
                members,
            });
        }

        if kind == ClassKind::Enum {
            // Parse enum constants
            while self.peek() != &Token::RBrace
                && self.peek() != &Token::Semi
                && self.peek() != &Token::Eof
            {
                self.parse_annotations()?; // consume any annotations on enum constants
                if matches!(self.peek(), Token::RBrace | Token::Semi) {
                    break;
                }
                let const_name = self.expect_ident()?;
                let args = if self.peek() == &Token::LParen {
                    self.parse_args()?
                } else {
                    Vec::new()
                };
                // skip optional class body
                if self.peek() == &Token::LBrace {
                    self.skip_balanced(Token::LBrace, Token::RBrace);
                }
                members.push(Member::EnumConstant(EnumConstant {
                    name: const_name,
                    args,
                }));
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.eat(&Token::Semi);
        }

        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            self.parse_annotations()?; // consume any member annotations
            if self.peek() == &Token::RBrace {
                break;
            }
            if let Some(m) = self.parse_member(&name)? {
                members.push(m);
                members.append(&mut self.pending_fields);
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(ClassDecl {
            name,
            kind,
            type_params_raw,
            modifiers,
            annotations,
            superclass,
            superclass_type_args_raw,
            interfaces,
            interfaces_type_args_raw,
            permitted_subclasses,
            permitted_type_args_raw,
            members,
        })
    }

    pub(crate) fn parse_modifiers(&mut self) -> Vec<Modifier> {
        let mut mods = Vec::new();
        loop {
            match self.peek() {
                Token::Public => {
                    self.advance();
                    mods.push(Modifier::Public);
                }
                Token::Private => {
                    self.advance();
                    mods.push(Modifier::Private);
                }
                Token::Protected => {
                    self.advance();
                    mods.push(Modifier::Protected);
                }
                Token::Static => {
                    self.advance();
                    mods.push(Modifier::Static);
                }
                Token::Final => {
                    self.advance();
                    mods.push(Modifier::Final);
                }
                Token::Abstract => {
                    self.advance();
                    mods.push(Modifier::Abstract);
                }
                Token::Synchronized => {
                    self.advance();
                    mods.push(Modifier::Synchronized);
                }
                Token::Native => {
                    self.advance();
                    mods.push(Modifier::Native);
                }
                Token::Volatile => {
                    self.advance();
                    mods.push(Modifier::Volatile);
                }
                Token::Transient => {
                    self.advance();
                    mods.push(Modifier::Transient);
                }
                Token::Strictfp => {
                    self.advance();
                    mods.push(Modifier::Strictfp);
                }
                Token::Default => {
                    self.advance();
                    mods.push(Modifier::Default);
                }
                Token::Sealed => {
                    self.advance();
                    mods.push(Modifier::Abstract);
                }
                Token::NonSealed => {
                    self.advance();
                    mods.push(Modifier::NonSealed);
                }
                _ => break,
            }
        }
        mods
    }

    pub(crate) fn parse_member(&mut self, class_name: &str) -> Result<Option<Member>> {
        let annotations = self.parse_annotations()?;
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
        if matches!(
            self.peek(),
            Token::Class | Token::Interface | Token::Enum | Token::Record
        ) {
            let inner = self.parse_class()?;
            return Ok(Some(Member::InnerClass(inner)));
        }

        // Skip method-level generic type parameters: `<T extends Foo<T>>`
        let member_type_params_raw = self.parse_angle_raw();

        // constructor: ClassName(
        if matches!(self.peek(), Token::Ident(n) if n == class_name)
            && self.peek2() == &Token::LParen
        {
            let name = self.expect_ident()?;
            let params = self.parse_params()?;
            self.skip_throws();
            let body = self.parse_block()?;
            return Ok(Some(Member::Constructor(ConstructorDecl {
                name,
                type_params_raw: member_type_params_raw,
                modifiers,
                annotations,
                params,
                body,
            })));
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
                // Annotation element default value: `String value() default "x";`
                if self.eat(&Token::Default) {
                    if self.peek() == &Token::LBrace {
                        let _ = self.parse_array_init()?;
                    } else {
                        let _ = self.parse_expr()?;
                    }
                }
                self.expect(&Token::Semi)?;
                None
            };
            Ok(Some(Member::Method(MethodDecl {
                name,
                type_params_raw: member_type_params_raw,
                modifiers,
                annotations,
                return_ty: ty,
                params,
                body,
            })))
        } else {
            // field — may be comma-separated: `int w, h;`
            let init = if self.eat(&Token::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            if self.peek() == &Token::Comma {
                // Multiple declarators: emit first field, then loop for the rest
                // We return the first and push extras into inner_classes (reuse parse_member loop)
                // Simpler: collect all names and return the first, stash extras via a side channel.
                // Since parse_member returns Option<Member>, we handle this by returning the first
                // field and letting the caller loop — but the caller only calls parse_member once.
                // Best approach: parse all names here and push extra fields to a pending list.
                // For now, skip extra declarators (they'll be uninitialized) by consuming them.
                let mut extra_fields: Vec<(String, Option<Expr>)> = vec![];
                while self.eat(&Token::Comma) {
                    if let Token::Ident(n) = self.peek().clone() {
                        self.advance();
                        let extra_init = if self.eat(&Token::Assign) {
                            Some(self.parse_expr()?)
                        } else {
                            None
                        };
                        extra_fields.push((n, extra_init));
                    }
                }
                self.expect(&Token::Semi)?;
                // Push extra fields as inner members via pending_fields, preserving initializers
                for (extra_name, extra_init) in extra_fields {
                    self.pending_fields.push(Member::Field(FieldDecl {
                        name: extra_name,
                        modifiers: modifiers.clone(),
                        annotations: vec![],
                        ty: ty.clone(),
                        init: extra_init,
                    }));
                }
            } else {
                self.expect(&Token::Semi)?;
            }
            Ok(Some(Member::Field(FieldDecl {
                name,
                modifiers,
                annotations,
                ty,
                init,
            })))
        }
    }
}
