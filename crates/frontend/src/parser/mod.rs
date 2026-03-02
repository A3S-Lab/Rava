//! Java parser — tokens → AST.
//!
//! Recursive-descent parser with full Java 21 syntax coverage:
//! class/interface/enum, all statements, all expressions including
//! lambda, method reference, ternary, instanceof pattern matching.

use crate::ast::*;
use crate::lexer::Token;
use rava_common::error::{RavaError, Result};

mod class;
mod expr;
mod stmt;
mod types;

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) pending_fields: Vec<crate::ast::Member>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            pending_fields: Vec::new(),
        }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    pub(crate) fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    pub(crate) fn peek2(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    #[allow(dead_code)]
    pub(crate) fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    pub(crate) fn advance(&mut self) -> &Token {
        let t = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    pub(crate) fn expect(&mut self, expected: &Token) -> Result<()> {
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

    pub(crate) fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect_ident(&mut self) -> Result<String> {
        match self.advance().clone() {
            Token::Ident(s) => Ok(s),
            got => Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected identifier, got {:?}", got),
            }),
        }
    }

    /// Parse zero or more annotations: `@Name`, `@Name(value)`, `@Name(key=value, ...)`.
    pub(crate) fn parse_annotations(&mut self) -> Result<Vec<crate::ast::Annotation>> {
        let mut annotations = Vec::new();
        while self.peek() == &Token::At {
            // `@interface` starts an annotation type declaration, not an annotation instance.
            if self.peek2() == &Token::Interface {
                break;
            }
            self.advance(); // consume @
            let name = self.expect_ident()?;
            let mut attrs = Vec::new();
            if self.peek() == &Token::LParen {
                self.advance(); // consume (
                if self.peek() != &Token::RParen {
                    loop {
                        // key=value or just value
                        let (key, val) = if matches!(self.peek(), Token::Ident(_))
                            && self.peek2() == &Token::Assign
                        {
                            let k = self.expect_ident()?;
                            self.advance(); // consume =
                            let v = self.parse_expr()?;
                            (k, v)
                        } else {
                            let v = self.parse_expr()?;
                            (String::new(), v)
                        };
                        attrs.push((key, val));
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RParen)?;
            }
            annotations.push(crate::ast::Annotation { name, attrs });
        }
        Ok(annotations)
    }

    // ── Top-level ───────────────────────────────────────────────────────────��─

    pub fn parse_file(&mut self) -> Result<SourceFile> {
        let mut package = None;
        let mut imports = Vec::new();
        let mut module = None;

        // module-info.java: `[open] module <name> { requires ...; exports ...; }` — parse and discard
        let mut module_open = false;
        if matches!(self.peek(), Token::Ident(s) if s == "open")
            && matches!(self.peek2(), Token::Ident(s) if s == "module")
        {
            self.advance(); // open
            module_open = true;
        }
        if matches!(self.peek(), Token::Ident(s) if s == "module") {
            module = Some(self.parse_module_decl(module_open)?);
            return Ok(SourceFile {
                package,
                imports,
                module,
                classes: Vec::new(),
            });
        }

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

        Ok(SourceFile {
            package,
            imports,
            module,
            classes,
        })
    }

    pub(crate) fn parse_qualified_name(&mut self) -> Result<String> {
        let mut name = self.expect_ident()?;
        while self.peek() == &Token::Dot {
            if self.peek2() == &Token::Star {
                break;
            }
            if !matches!(self.peek2(), Token::Ident(_)) {
                break;
            }
            self.advance();
            name.push('.');
            name.push_str(&self.expect_ident()?);
        }
        Ok(name)
    }

    fn eat_ident_kw(&mut self, kw: &str) -> bool {
        if matches!(self.peek(), Token::Ident(s) if s == kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_module_decl(&mut self, open: bool) -> Result<ModuleDecl> {
        if !self.eat_ident_kw("module") {
            return Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: "expected 'module' keyword".into(),
            });
        }
        let name = self.parse_qualified_name()?;
        self.expect(&Token::LBrace)?;

        let mut directives = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.eat_ident_kw("requires") {
                let mut is_static = false;
                let mut is_transitive = false;
                loop {
                    if self.eat(&Token::Static) {
                        is_static = true;
                    } else if self.eat_ident_kw("transitive") {
                        is_transitive = true;
                    } else {
                        break;
                    }
                }
                let module = self.parse_qualified_name()?;
                self.expect(&Token::Semi)?;
                directives.push(ModuleDirective::Requires {
                    module,
                    is_static,
                    is_transitive,
                });
                continue;
            }

            if self.eat_ident_kw("exports") {
                let package = self.parse_qualified_name()?;
                let mut to = Vec::new();
                if self.eat_ident_kw("to") {
                    to.push(self.parse_qualified_name()?);
                    while self.eat(&Token::Comma) {
                        to.push(self.parse_qualified_name()?);
                    }
                }
                self.expect(&Token::Semi)?;
                directives.push(ModuleDirective::Exports { package, to });
                continue;
            }

            if self.eat_ident_kw("opens") {
                let package = self.parse_qualified_name()?;
                let mut to = Vec::new();
                if self.eat_ident_kw("to") {
                    to.push(self.parse_qualified_name()?);
                    while self.eat(&Token::Comma) {
                        to.push(self.parse_qualified_name()?);
                    }
                }
                self.expect(&Token::Semi)?;
                directives.push(ModuleDirective::Opens { package, to });
                continue;
            }

            if self.eat_ident_kw("uses") {
                let service = self.parse_qualified_name()?;
                self.expect(&Token::Semi)?;
                directives.push(ModuleDirective::Uses { service });
                continue;
            }

            if self.eat_ident_kw("provides") {
                let service = self.parse_qualified_name()?;
                if !self.eat_ident_kw("with") {
                    return Err(RavaError::Parse {
                        location: format!("pos {}", self.pos),
                        message: "expected 'with' in provides directive".into(),
                    });
                }
                let mut implementations = vec![self.parse_qualified_name()?];
                while self.eat(&Token::Comma) {
                    implementations.push(self.parse_qualified_name()?);
                }
                self.expect(&Token::Semi)?;
                directives.push(ModuleDirective::Provides {
                    service,
                    implementations,
                });
                continue;
            }

            // Unknown directive: skip to statement terminator to remain robust.
            while self.peek() != &Token::Semi
                && self.peek() != &Token::RBrace
                && self.peek() != &Token::Eof
            {
                self.advance();
            }
            self.eat(&Token::Semi);
        }

        self.expect(&Token::RBrace)?;
        Ok(ModuleDecl {
            name,
            open,
            directives,
        })
    }

    pub(crate) fn skip_balanced(&mut self, open: Token, close: Token) {
        if self.peek() != &open {
            return;
        }
        let mut depth = 0i32;
        loop {
            let t = self.advance().clone();
            if t == open {
                depth += 1;
            }
            if t == close {
                depth -= 1;
                if depth <= 0 {
                    break;
                }
            }
            if t == Token::Eof {
                break;
            }
        }
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        assert!(matches!(
            m.body.as_ref().unwrap().0[0],
            Stmt::ForEach { .. }
        ));
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        assert!(matches!(
            m.body.as_ref().unwrap().0[0],
            Stmt::TryCatch { .. }
        ));
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        let stmts = &m.body.as_ref().unwrap().0;
        if let Stmt::LocalVar {
            init: Some(Expr::Lambda { params, .. }),
            ..
        } = &stmts[0]
        {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
        } else {
            panic!("expected lambda");
        }
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        if let Stmt::If {
            cond: Expr::InstanceofPattern { name, .. },
            ..
        } = &m.body.as_ref().unwrap().0[0]
        {
            assert_eq!(name, "s");
        } else {
            panic!("expected instanceof pattern");
        }
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
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        if let Stmt::LocalVar {
            init: Some(Expr::MethodRef { name, .. }),
            ..
        } = &m.body.as_ref().unwrap().0[0]
        {
            assert_eq!(name, "println");
        } else {
            panic!("expected method ref");
        }
    }

    #[test]
    fn parse_record() {
        let src = r#"
            record Point(int x, int y) {
            }
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].kind, ClassKind::Record);
        assert_eq!(file.classes[0].name, "Point");
        // Should have 2 fields + 1 constructor + 2 accessors = 5 members
        assert_eq!(file.classes[0].members.len(), 5);
    }

    #[test]
    fn parse_text_block() {
        let src =
            "class T { void f() { String s = \"\"\"\n            hello\n            \"\"\"; } }";
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        if let Stmt::LocalVar {
            init: Some(Expr::StrLit(s)),
            ..
        } = &m.body.as_ref().unwrap().0[0]
        {
            assert!(s.contains("hello"));
        } else {
            panic!("expected text block string");
        }
    }

    #[test]
    fn parse_switch_arrow() {
        let src = r#"
            class T {
                void f() {
                    switch (x) {
                        case 1 -> System.out.println("one");
                        case 2 -> { System.out.println("two"); }
                        default -> System.out.println("other");
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        if let Stmt::Switch { cases, .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(cases.len(), 3);
        } else {
            panic!("expected switch");
        }
    }

    #[test]
    fn parse_sealed_class() {
        let src = r#"
            sealed class Shape permits Circle, Rect {
            }
            class Circle extends Shape {}
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].name, "Shape");
        assert_eq!(file.classes[1].name, "Circle");
    }

    #[test]
    fn parse_yield_stmt() {
        let src = r#"
            class T {
                void f() {
                    yield 42;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        if let Stmt::Yield(Expr::IntLit(42)) = &m.body.as_ref().unwrap().0[0] {
            // ok
        } else {
            panic!("expected yield 42");
        }
    }

    #[test]
    fn parse_non_sealed_modifier() {
        let src = r#"
            non-sealed class Child extends Base {}
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].name, "Child");
        assert!(file.classes[0].modifiers.contains(&Modifier::NonSealed));
    }

    #[test]
    fn parse_module_info_keeps_module_decl() {
        let src = r#"
            open module com.example.app {
                requires java.base;
                exports com.example;
            }
        "#;
        let file = parse(src);
        let module = file.module.expect("module should be parsed");
        assert!(module.open);
        assert_eq!(module.name, "com.example.app");
        assert_eq!(module.directives.len(), 2);
        assert!(file.classes.is_empty());
    }

    #[test]
    fn parse_module_info_directives() {
        let src = r#"
            module com.acme.app {
                requires static transitive com.acme.core;
                exports com.acme.api to com.foo.client, com.bar.client;
                opens com.acme.internal;
                uses com.acme.spi.Service;
                provides com.acme.spi.Service with com.acme.impl.ServiceImpl, com.acme.impl.Fallback;
            }
        "#;
        let file = parse(src);
        let module = file.module.expect("module should be parsed");
        assert_eq!(module.name, "com.acme.app");
        assert_eq!(module.directives.len(), 5);

        match &module.directives[0] {
            ModuleDirective::Requires {
                module,
                is_static,
                is_transitive,
            } => {
                assert_eq!(module, "com.acme.core");
                assert!(*is_static);
                assert!(*is_transitive);
            }
            _ => panic!("expected requires directive"),
        }

        match &module.directives[1] {
            ModuleDirective::Exports { package, to } => {
                assert_eq!(package, "com.acme.api");
                assert_eq!(to.len(), 2);
            }
            _ => panic!("expected exports directive"),
        }

        match &module.directives[2] {
            ModuleDirective::Opens { package, to } => {
                assert_eq!(package, "com.acme.internal");
                assert!(to.is_empty());
            }
            _ => panic!("expected opens directive"),
        }

        match &module.directives[3] {
            ModuleDirective::Uses { service } => {
                assert_eq!(service, "com.acme.spi.Service");
            }
            _ => panic!("expected uses directive"),
        }

        match &module.directives[4] {
            ModuleDirective::Provides {
                service,
                implementations,
            } => {
                assert_eq!(service, "com.acme.spi.Service");
                assert_eq!(implementations.len(), 2);
            }
            _ => panic!("expected provides directive"),
        }
    }

    #[test]
    fn parse_annotation_interface_declaration() {
        let src = r#"
            @interface MyAnno {
                String value();
            }
        "#;
        let file = parse(src);
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].name, "MyAnno");
        assert_eq!(file.classes[0].kind, ClassKind::Annotation);
    }

    #[test]
    fn parse_sealed_permits_list() {
        let src = r#"
            sealed class Shape permits Circle, Rect {}
            final class Circle extends Shape {}
            final class Rect extends Shape {}
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].name, "Shape");
        assert_eq!(file.classes[0].permitted_subclasses.len(), 2);
        assert_eq!(file.classes[0].permitted_subclasses[0], "Circle");
        assert_eq!(file.classes[0].permitted_subclasses[1], "Rect");
    }

    #[test]
    fn parse_interface_extends_list() {
        let src = r#"
            interface B extends A, C {}
        "#;
        let file = parse(src);
        let iface = &file.classes[0];
        assert_eq!(iface.kind, ClassKind::Interface);
        assert!(iface.superclass.is_none());
        assert_eq!(iface.interfaces.len(), 2);
        assert_eq!(iface.interfaces[0], "A");
        assert_eq!(iface.interfaces[1], "C");
    }

    #[test]
    fn parse_class_type_params_are_retained() {
        let src = r#"
            class Box<T extends Number> {}
        "#;
        let file = parse(src);
        let raw = file.classes[0]
            .type_params_raw
            .as_ref()
            .expect("class type params should be captured");
        assert!(raw.contains("T"));
        assert!(raw.contains("extends"));
        assert!(raw.contains("Number"));
    }

    #[test]
    fn parse_method_type_params_are_retained() {
        let src = r#"
            class T {
                <U extends Comparable<U>> U pick(U a, U b) {
                    return a;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        let raw = m
            .type_params_raw
            .as_ref()
            .expect("method type params should be captured");
        assert!(raw.contains("Comparable"));
        assert!(raw.contains("U"));
    }

    #[test]
    fn parse_generic_type_args_are_retained() {
        let src = r#"
            class T {
                java.util.List<String> names;
                java.util.Map<String, java.util.List<Integer>> index;
            }
        "#;
        let file = parse(src);
        let Member::Field(f1) = &file.classes[0].members[0] else {
            panic!()
        };
        let Member::Field(f2) = &file.classes[0].members[1] else {
            panic!()
        };

        let args1 = f1
            .ty
            .generic_args_raw
            .as_ref()
            .expect("list generic args should be captured");
        let args2 = f2
            .ty
            .generic_args_raw
            .as_ref()
            .expect("map generic args should be captured");

        assert!(args1.contains("String"));
        assert!(args2.contains("String"));
        assert!(args2.contains("List"));
        assert!(args2.contains("Integer"));
    }

    #[test]
    fn parse_inheritance_generic_args_are_retained() {
        let src = r#"
            class Child extends Base<String> implements A<Integer>, B {}
        "#;
        let file = parse(src);
        let class = &file.classes[0];
        assert_eq!(class.superclass.as_deref(), Some("Base"));
        assert_eq!(class.superclass_type_args_raw.as_deref(), Some("String"));
        assert_eq!(class.interfaces.len(), 2);
        assert_eq!(class.interfaces[0], "A");
        assert_eq!(
            class.interfaces_type_args_raw[0].as_deref(),
            Some("Integer")
        );
        assert_eq!(class.interfaces[1], "B");
        assert!(class.interfaces_type_args_raw[1].is_none());
    }

    #[test]
    fn parse_explicit_method_type_args_call() {
        let src = r#"
            class T {
                void m() {
                    Util.<String>id("x");
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        let Stmt::Expr(Expr::Call { type_args_raw, .. }) = &m.body.as_ref().unwrap().0[0] else {
            panic!()
        };
        assert_eq!(type_args_raw.as_deref(), Some("String"));
    }

    #[test]
    fn parse_new_diamond_operator_retained() {
        let src = r#"
            class T {
                void m() {
                    java.util.List<String> xs = new java.util.ArrayList<>();
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else {
            panic!()
        };
        let Stmt::LocalVar {
            init: Some(Expr::New { ty, .. }),
            ..
        } = &m.body.as_ref().unwrap().0[0]
        else {
            panic!()
        };
        assert_eq!(ty.name, "java.util.ArrayList");
        assert_eq!(ty.generic_args_raw.as_deref(), Some(""));
    }

    #[test]
    fn parse_structured_wildcard_type_args() {
        let src = r#"
            class T {
                java.util.List<? extends Number> upper;
                java.util.Map<String, ? super Integer> lower;
            }
        "#;
        let file = parse(src);
        let Member::Field(f1) = &file.classes[0].members[0] else {
            panic!()
        };
        let Member::Field(f2) = &file.classes[0].members[1] else {
            panic!()
        };

        let args1 = f1
            .ty
            .generic_args
            .as_ref()
            .expect("structured args expected");
        let args2 = f2
            .ty
            .generic_args
            .as_ref()
            .expect("structured args expected");

        assert!(matches!(args1[0], TypeArg::WildcardExtends(ref t) if t.name == "Number"));
        assert!(matches!(args2[0], TypeArg::Type(ref t) if t.name == "String"));
        assert!(matches!(args2[1], TypeArg::WildcardSuper(ref t) if t.name == "Integer"));
    }

    #[test]
    fn parse_structured_nested_type_args() {
        let src = r#"
            class T {
                java.util.Map<String, java.util.List<Integer>> x;
            }
        "#;
        let file = parse(src);
        let Member::Field(f) = &file.classes[0].members[0] else {
            panic!()
        };
        let args =
            f.ty.generic_args
                .as_ref()
                .expect("structured args expected");
        assert!(matches!(args[0], TypeArg::Type(ref t) if t.name == "String"));
        let TypeArg::Type(inner) = &args[1] else {
            panic!()
        };
        assert_eq!(inner.name, "java.util.List");
        let inner_args = inner
            .generic_args
            .as_ref()
            .expect("nested generic args expected");
        assert!(matches!(inner_args[0], TypeArg::Type(ref t) if t.name == "Integer"));
    }
}
